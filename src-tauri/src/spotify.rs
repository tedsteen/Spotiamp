use std::sync::{Arc, Mutex};

use crate::oauth::{OAuthError, OAuthFlow};
use librespot::{
    core::{
        authentication::Credentials, cache::Cache, config::SessionConfig, session::Session,
        spotify_id::SpotifyId, Error,
    },
    metadata::{Metadata, Track},
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::VolumeGetter,
        player::Player,
    },
};
use oauth2::TokenResponse;
use tauri::AppHandle;
use thiserror::Error;

use crate::settings::get_config_dir;

pub struct SpotifyPlayer {
    player: Arc<Player>,
    session: Session,
    volume: Arc<Mutex<u16>>,
    cache: Cache,
}

const DEFAULT_VOLUME: u16 = 80;
impl SpotifyPlayer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let cache = get_config_dir()
            .and_then(|config_dir| {
                Cache::new(
                    Some(config_dir.clone()),
                    Some(config_dir.clone()),
                    Some(config_dir),
                    None,
                )
                .ok()
            })
            .expect("a cache to be created");

        let backend = audio_backend::find(None).unwrap();
        let player_config = PlayerConfig::default();

        struct SpotiampVolumeGetter {
            volume: Arc<Mutex<u16>>,
        }

        impl VolumeGetter for SpotiampVolumeGetter {
            fn attenuation_factor(&self) -> f64 {
                *self.volume.lock().unwrap() as f64 / 100.0
            }
        }
        let session = Session::new(SessionConfig::default(), Some(cache.clone()));
        let volume = Arc::new(Mutex::new(cache.volume().unwrap_or(DEFAULT_VOLUME)));
        let player = Player::new(
            player_config,
            session.clone(),
            Box::new(SpotiampVolumeGetter {
                volume: volume.clone(),
            }),
            move || {
                let audio_format = AudioFormat::default();
                backend(None, audio_format)
            },
        );
        Self {
            player,
            session,
            volume,
            cache,
        }
    }

    pub async fn login(&self, app: &AppHandle) -> Result<(), SessionError> {
        log::debug!("Getting credentials");
        let credentials = match self.cache.credentials() {
            Some(credentials) => credentials,
            None => {
                log::debug!("No credentials in cache, starting OAuth flow...");
                Self::get_credentials_from_oauth(app).await?
            }
        };

        self.session
            .connect(credentials, true)
            .await
            .map_err(|e| SessionError::ConnectError { e })?;
        log::debug!("Success! Using credentials from OAuth-flow and saving them for next time");
        Ok(())
    }

    async fn get_credentials_from_oauth(app: &AppHandle) -> Result<Credentials, SessionError> {
        let oauth_flow = OAuthFlow::new(
            "https://accounts.spotify.com/authorize",
            "https://accounts.spotify.com/api/token",
            "65b708073fc0480ea92a077233ca87bd",
        )
        .map_err(|e| SessionError::OauthError { e })?;

        let auth_url = oauth_flow.get_auth_url();
        log::debug!("Opening URL: {auth_url}");

        tauri::WebviewWindowBuilder::new(
            app,
            "login",
            tauri::WebviewUrl::External(auth_url.parse().expect("a valid auth URL")),
        )
        .title("Login")
        .inner_size(600.0, 800.0)
        .closable(false)
        .build()
        .map_err(|e| SessionError::OpenURLFailed { url: auth_url, e })?;

        let token = oauth_flow
            .start()
            .await
            .map_err(|e| SessionError::TokenExchangeFailure { e })?;

        Ok(Credentials::with_access_token(
            token.access_token().secret(),
        ))
    }

    pub async fn play(&mut self, uri: Option<&str>) -> Result<(), PlayError> {
        log::debug!("Play!");
        if let Some(uri) = uri {
            self.player.load(
                SpotifyId::from_uri(uri).map_err(|e| PlayError::TrackMetadataError { e })?,
                false,
                0,
            );
        }

        self.player.play();
        Ok(())
    }

    pub async fn pause(&mut self) -> Result<(), PlayError> {
        log::debug!("Pause!");
        self.player.pause();
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), PlayError> {
        log::debug!("Stop!");
        self.player.stop();
        Ok(())
    }

    pub async fn get_track(&mut self, track: SpotifyId) -> Result<Track, PlayError> {
        log::debug!("Getting track data: {:?}", track);
        //TODO: Check why we get `TrackMetadataError { e: Error { kind: Internal, error: ErrorMessage("channel closed") } }` here after leaving the mac in standby for a while.
        Track::get(&self.session, &track)
            .await
            .map_err(|e| PlayError::TrackMetadataError { e })
    }

    pub fn set_volume(&mut self, volume: u16) {
        *self.volume.lock().unwrap() = volume;
        self.cache.save_volume(volume);
    }

    pub fn get_volume(&self) -> u16 {
        *self.volume.lock().unwrap()
    }
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Failed to connect ({e:?}")]
    ConnectError { e: Error },

    #[error("OAuth error ({e:?}")]
    OauthError { e: OAuthError },

    #[error("Could not open URL {url} ({e:?})")]
    OpenURLFailed { url: String, e: tauri::Error },

    #[error("Could not get token ({e:?}")]
    TokenExchangeFailure { e: OAuthError },
}

#[derive(Debug, Error)]
pub enum PlayError {
    #[error("Failed to fetch metadata ({e:?})")]
    TrackMetadataError { e: Error },
}

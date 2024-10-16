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
use thiserror::Error;

use crate::settings::get_config_dir;

pub struct SpotifyPlayer {
    player: Option<Arc<Player>>,
    volume: Arc<Mutex<u16>>,
    cache: Cache,
    session: Session,
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
        Self {
            player: None,
            volume: Arc::new(Mutex::new(cache.volume().unwrap_or(DEFAULT_VOLUME))),
            cache: cache.clone(),
            session: Session::new(SessionConfig::default(), Some(cache)),
        }
    }

    async fn get_credentials_from_oauth() -> Result<Credentials, SessionError> {
        let oauth_flow = OAuthFlow::new(
            "https://accounts.spotify.com/authorize",
            "https://accounts.spotify.com/api/token",
            "65b708073fc0480ea92a077233ca87bd",
        )
        .map_err(|e| SessionError::OauthError { e })?;

        let auth_url = oauth_flow.get_auth_url();
        log::debug!("Opening URL: {auth_url}");
        open::that(oauth_flow.get_auth_url())
            .map_err(|e| SessionError::OpenURLFailed { url: auth_url, e })?;

        let token = oauth_flow
            .start()
            .await
            .map_err(|e| SessionError::TokenExchangeFailure { e })?;

        Ok(Credentials::with_access_token(
            token.access_token().secret(),
        ))
    }

    async fn login_session(&self) -> Result<Session, SessionError> {
        log::debug!("Getting credentials");
        if let Some(credentials) = self.cache.credentials() {
            log::debug!("Credentials found in cache, trying that...");
            if self.session.connect(credentials, true).await.is_ok() {
                log::debug!("Success! Using cached credentials");
                return Ok(self.session.clone());
            }
        }
        log::debug!("Not logged in, starting OAuth flow...");
        let credentials = Self::get_credentials_from_oauth().await?;
        self.session
            .connect(credentials, true)
            .await
            .map_err(|e| SessionError::ConnectError { e })?;
        log::debug!("Success! Using credentials from OAuth-flow and saving them for next time");
        Ok(self.session.clone())
    }

    async fn get_player(&mut self) -> Result<Arc<Player>, SessionError> {
        if self.player.is_none() {
            let session = self.login_session().await?;
            let backend = audio_backend::find(None).unwrap();
            let player_config = PlayerConfig::default();

            struct SpotiyampVolumeGetter {
                volume: Arc<Mutex<u16>>,
            }

            impl VolumeGetter for SpotiyampVolumeGetter {
                fn attenuation_factor(&self) -> f64 {
                    *self.volume.lock().unwrap() as f64 / 100.0
                }
            }

            self.player = Some(Player::new(
                player_config,
                session.clone(),
                Box::new(SpotiyampVolumeGetter {
                    volume: self.volume.clone(),
                }),
                move || {
                    let audio_format = AudioFormat::default();
                    backend(None, audio_format)
                },
            ));
        }
        self.player.clone().ok_or(SessionError::LoginFailed)
    }

    pub async fn play(&mut self) -> Result<(), PlayError> {
        log::debug!("Play!");
        let player = self
            .get_player()
            .await
            .map_err(|e| PlayError::SessionError { e })?;
        player.play();
        Ok(())
    }

    pub async fn pause(&mut self) -> Result<(), PlayError> {
        log::debug!("Pause!");
        let player = self
            .get_player()
            .await
            .map_err(|e| PlayError::SessionError { e })?;
        player.pause();
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), PlayError> {
        log::debug!("Stop!");
        let player = self
            .get_player()
            .await
            .map_err(|e| PlayError::SessionError { e })?;
        player.stop();
        Ok(())
    }

    pub async fn load(&mut self, track: SpotifyId) -> Result<Track, PlayError> {
        log::debug!("Loading track: {:?}", track);
        let player = self
            .get_player()
            .await
            .map_err(|e| PlayError::SessionError { e })?;
        player.load(track, false, 0);

        let track = Track::get(&self.session, &track)
            .await
            .map_err(|e| PlayError::TrackMetadataError { e })?;
        Ok(track)
    }

    pub fn set_volume(&mut self, volume: u16) {
        *self.volume.lock().unwrap() = volume;
        self.cache.save_volume(volume);
    }

    pub fn get_volume(&self) -> u16 {
        self.cache.volume().unwrap_or(DEFAULT_VOLUME)
    }
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Failed to connect ({e:?}")]
    ConnectError { e: Error },

    #[error("OAuth error ({e:?}")]
    OauthError { e: OAuthError },

    #[error("Could not open URL {url} ({e:?})")]
    OpenURLFailed { url: String, e: std::io::Error },

    #[error("Could not get token ({e:?}")]
    TokenExchangeFailure { e: OAuthError },

    #[error("Failed to login")]
    LoginFailed,
}

#[derive(Debug, Error)]
pub enum PlayError {
    #[error("Session failed ({e:?})")]
    SessionError { e: SessionError },

    #[error("Failed to fetch metadata ({e:?})")]
    TrackMetadataError { e: Error },
}

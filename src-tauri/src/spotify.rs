use std::{
    sync::{Arc, Mutex, atomic::AtomicU16},
    time::Duration,
};

use crate::{
    oauth::{OAuthError, OAuthFlow},
    settings::Settings,
    sink::SpotiampSink,
    visualizer::Visualizer,
};
use librespot::{
    core::{
        Error, authentication::Credentials, cache::Cache, config::SessionConfig, session::Session,
        spotify_id::SpotifyId,
    },
    metadata::{Album, Metadata, Playlist, Track},
    playback::{
        config::{AudioFormat, Bitrate, NormalisationMethod, NormalisationType, PlayerConfig},
        dither::{TriangularDitherer, mk_ditherer},
        mixer::VolumeGetter,
        player::{Player, PlayerEventChannel, duration_to_coefficient},
    },
};
use oauth2::TokenResponse;
use tauri::AppHandle;
use thiserror::Error;

use crate::settings::get_config_dir;
pub type SharedPlayer = Arc<tokio::sync::Mutex<SpotifyPlayer>>;
pub struct SpotifySession {
    inner: Session,
    cache: Cache,
}

impl SpotifySession {
    pub fn new() -> Self {
        let cache = get_config_dir()
            .and_then(|config_dir| {
                Cache::new(Some(config_dir.clone()), None, Some(config_dir), None).ok()
            })
            .expect("a cache to be created");
        let session = Session::new(SessionConfig::default(), Some(cache.clone()));
        Self {
            inner: session,
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

        self.inner
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

        let window = tauri::WebviewWindowBuilder::new(
            app,
            "login",
            tauri::WebviewUrl::External(auth_url.parse().expect("a valid auth URL")),
        )
        .title("Login")
        .inner_size(600.0, 800.0)
        .closable(true)
        .maximizable(false)
        .resizable(false)
        .build()
        .map_err(|e| SessionError::OpenURLFailed { url: auth_url, e })?;

        let token_received = Arc::new(Mutex::new(false));
        window.on_window_event({
            let token_received = token_received.clone();
            move |e| {
                if let tauri::WindowEvent::CloseRequested { .. } = &e
                    && !*token_received.lock().unwrap()
                {
                    log::info!("No token received when closing login window. Exiting.");
                    std::process::exit(0);
                }
            }
        });

        let token = oauth_flow
            .start()
            .await
            .map_err(|e| SessionError::TokenExchangeFailure { e })?;
        *token_received.lock().unwrap() = true;
        let _ = window.close();

        Ok(Credentials::with_access_token(
            token.access_token().secret(),
        ))
    }
}

pub struct SpotifyPlayer {
    player: Arc<Player>,
    pub session: SpotifySession,
    volume: Arc<AtomicU16>,

    visualizer: Arc<Mutex<Visualizer>>,
}

impl SpotifyPlayer {
    #[allow(clippy::new_without_default)]
    pub fn new(session: SpotifySession) -> Self {
        let player_config = PlayerConfig {
            position_update_interval: None,
            bitrate: Bitrate::Bitrate320,
            gapless: true,
            normalisation: false,
            normalisation_type: NormalisationType::default(),
            normalisation_method: NormalisationMethod::Dynamic,
            normalisation_pregain_db: 0.0,
            normalisation_threshold_dbfs: -2.0,
            normalisation_attack_cf: duration_to_coefficient(Duration::from_millis(5)),
            normalisation_release_cf: duration_to_coefficient(Duration::from_millis(100)),
            normalisation_knee_db: 5.0,
            passthrough: false,
            ditherer: Some(mk_ditherer::<TriangularDitherer>),
        };

        struct SpotiampVolumeGetter {
            volume: Arc<AtomicU16>,
        }

        impl VolumeGetter for SpotiampVolumeGetter {
            fn attenuation_factor(&self) -> f64 {
                self.volume.load(std::sync::atomic::Ordering::Relaxed) as f64 / 100.0
            }
        }

        let volume = Arc::new(AtomicU16::new(Settings::current().player.volume));
        let visualizer = Arc::new(Mutex::new(Visualizer::new()));
        let player = Player::new(
            player_config,
            session.inner.clone(),
            Box::new(SpotiampVolumeGetter {
                volume: volume.clone(),
            }),
            {
                let visualizer = visualizer.clone();
                let volume = volume.clone();
                move || {
                    let audio_format = AudioFormat::F32;
                    Box::new(SpotiampSink::new(None, audio_format, visualizer, volume))
                }
            },
        );

        Self {
            player,
            session,
            volume,
            visualizer,
        }
    }

    pub async fn load_track(&self, uri: &str) -> Result<(), PlayError> {
        self.player.load(
            SpotifyId::from_uri(uri).map_err(|e| PlayError::MetadataError { e })?,
            true,
            0,
        );
        Ok(())
    }

    pub fn play(&mut self) {
        log::debug!("Play!");
        self.player.play();
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

    pub async fn get_track_ids(&self, playlist_id: SpotifyId) -> Result<Vec<SpotifyId>, PlayError> {
        match playlist_id.item_type {
            librespot::core::spotify_id::SpotifyItemType::Playlist => {
                Ok(Playlist::get(&self.session.inner, &playlist_id)
                    .await
                    .map_err(|e| PlayError::MetadataError { e })?
                    .contents
                    .items
                    .iter()
                    .filter(|item| {
                        let is_track = matches!(
                            &item.id.item_type,
                            librespot::core::spotify_id::SpotifyItemType::Track
                        );

                        is_track
                    })
                    .map(|item| &item.id)
                    .cloned()
                    .collect())
            }
            librespot::core::spotify_id::SpotifyItemType::Album => {
                Ok(Album::get(&self.session.inner, &playlist_id)
                    .await
                    .map_err(|e| PlayError::MetadataError { e })?
                    .tracks()
                    .cloned()
                    .collect())
            }
            _ => {
                log::warn!("Trying to get playlist tracks from an id that is not a playlist");
                Ok(vec![])
            }
        }
    }

    pub async fn get_track(&mut self, track_id: SpotifyId) -> Result<Track, PlayError> {
        match track_id.item_type {
            librespot::core::spotify_id::SpotifyItemType::Track => {
                log::debug!("Getting track data: {:?}", track_id);
                //TODO: Check why we get `TrackMetadataError { e: Error { kind: Internal, error: ErrorMessage("channel closed") } }` here after leaving the mac in standby for a while.
                Track::get(&self.session.inner, &track_id)
                    .await
                    .map_err(|e| PlayError::MetadataError { e })
            }
            _ => Err(PlayError::GettingTrackForNonTrackId(track_id)),
        }
    }

    pub fn set_volume(&mut self, volume: u16) {
        self.volume
            .store(volume, std::sync::atomic::Ordering::Relaxed);
        self.session.cache.save_volume(volume);
    }

    pub fn get_volume(&self) -> u16 {
        self.volume.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn seek(&self, position_ms: u32) {
        self.player.seek(position_ms);
    }

    pub fn take_latest_spectrum(&mut self) -> Vec<(f32, f32)> {
        self.visualizer.lock().unwrap().take_latest_spectrum()
    }

    pub fn get_player_event_channel(&self) -> PlayerEventChannel {
        self.player.get_player_event_channel()
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
    MetadataError { e: Error },
    #[error("Cannot get track for non track id ({_0:?})")]
    GettingTrackForNonTrackId(SpotifyId),
}

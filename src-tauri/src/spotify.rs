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
        Error, SpotifyUri, authentication::Credentials, cache::Cache, config::SessionConfig,
        session::Session,
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
    pub inner: Session,
    cache: Cache,
    pub access_token: Arc<tokio::sync::RwLock<Option<String>>>,
}

impl Default for SpotifySession {
    fn default() -> Self {
        let cache = get_config_dir()
            .and_then(|config_dir| {
                Cache::new(Some(config_dir.clone()), None, Some(config_dir), None).ok()
            })
            .expect("a cache to be created");
        let mut config = SessionConfig::default();
        config.client_id = "65b708073fc0480ea92a077233ca87bd".to_string();
        let session = Session::new(config, Some(cache.clone()));
        Self {
            inner: session,
            cache,
            access_token: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }
}

impl SpotifySession {
    pub async fn login(&self, app: &AppHandle) -> Result<(), SessionError> {
        log::debug!("Getting credentials");
        let has_oauth = Settings::current().oauth_token.refresh_token.is_some();
        let credentials = match self.cache.credentials() {
            Some(credentials) if has_oauth => credentials,
            _ => {
                log::debug!("No cached OAuth tokens found or refresh token missing. Forcing OAuth flow...");
                Self::get_credentials_from_oauth(app, self.access_token.clone()).await?
            }
        };

        self.inner
            .connect(credentials, true)
            .await
            .map_err(|e| SessionError::ConnectError { e })?;
        log::debug!("Success! Using credentials from OAuth-flow and saving them for next time");
        Ok(())
    }

    async fn get_credentials_from_oauth(
        app: &AppHandle,
        access_token_cell: Arc<tokio::sync::RwLock<Option<String>>>,
    ) -> Result<Credentials, SessionError> {
        let oauth_flow = OAuthFlow::new(
            "https://accounts.spotify.com/authorize",
            "https://accounts.spotify.com/api/token",
            "d420a117a32841c2b3474932e49fb54b",
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

        let access_token_str = token.access_token().secret().to_string();
        
        {
            let mut settings = crate::settings::Settings::current_mut();
            settings.oauth_token.access_token = Some(access_token_str.clone());
            settings.oauth_token.refresh_token = token.refresh_token().map(|t| t.secret().to_string());
            let expires_in = token.expires_in().unwrap_or(std::time::Duration::from_secs(3600));
            let now = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            settings.oauth_token.expires_at = Some(now + expires_in.as_secs());
        }

        {
            let mut guard = access_token_cell.write().await;
            *guard = Some(access_token_str.clone());
        }

        Ok(Credentials::with_access_token(
            &access_token_str,
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
            local_file_directories: Vec::new(),
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
        let uri = SpotifyUri::from_uri(uri).map_err(|e| PlayError::MetadataError { e })?;
        self.player.load(uri, true, 0);
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

    pub async fn get_track_ids(
        &self,
        playlist_uri: SpotifyUri,
    ) -> Result<Vec<SpotifyUri>, PlayError> {
        match playlist_uri {
            SpotifyUri::Playlist { .. } => Ok(Playlist::get(&self.session.inner, &playlist_uri)
                .await
                .map_err(|e| PlayError::MetadataError { e })?
                .contents
                .items
                .iter()
                .filter(|item| {
                    let is_track = matches!(&item.id, SpotifyUri::Track { .. });

                    is_track
                })
                .map(|item| &item.id)
                .cloned()
                .collect()),
            SpotifyUri::Album { .. } => Ok(Album::get(&self.session.inner, &playlist_uri)
                .await
                .map_err(|e| PlayError::MetadataError { e })?
                .tracks()
                .cloned()
                .collect()),
            _ => {
                log::warn!("Trying to get playlist tracks from an id that is not a playlist");
                Ok(vec![])
            }
        }
    }

    pub async fn get_track(&mut self, track_uri: SpotifyUri) -> Result<Track, PlayError> {
        match track_uri {
            SpotifyUri::Track { .. } => {
                log::debug!("Getting track data: {:?}", track_uri);
                //TODO: Check why we get `TrackMetadataError { e: Error { kind: Internal, error: ErrorMessage("channel closed") } }` here after leaving the mac in standby for a while.
                Track::get(&self.session.inner, &track_uri)
                    .await
                    .map_err(|e| PlayError::MetadataError { e })
            }
            _ => Err(PlayError::GettingTrackForNonTrackUri(track_uri)),
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
    GettingTrackForNonTrackUri(SpotifyUri),
}

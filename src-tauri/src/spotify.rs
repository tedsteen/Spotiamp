use std::{
    sync::{Arc, Mutex, atomic::AtomicU16},
    time::Duration,
};

use crate::{
    account,
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
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use thiserror::Error;

use crate::settings::get_config_dir;
pub type SharedPlayer = Arc<tokio::sync::Mutex<SpotifyPlayer>>;

use std::sync::OnceLock;
static PENDING_AUTH_URL: OnceLock<Mutex<Option<String>>> = OnceLock::new();
pub fn pending_auth_url() -> &'static Mutex<Option<String>> {
    PENDING_AUTH_URL.get_or_init(|| Mutex::new(None))
}
pub struct SpotifySession {
    inner: Session,
    cache: Cache,
}

impl Default for SpotifySession {
    fn default() -> Self {
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
}

impl SpotifySession {
    /// A handle to the underlying librespot session (cheap to clone), so callers
    /// can make network requests without holding the player lock.
    pub fn librespot_session(&self) -> Session {
        self.inner.clone()
    }

    pub async fn login(&self, app: &AppHandle) -> Result<(), SessionError> {
        if account::previous_sign_in_died() {
            log::warn!("The previous sign-in never finished, clearing the saved login");
            Self::remove_cached_credentials();
            app.dialog()
                .message(
                    "Spotiamp closed while signing in last time. This usually means the \
                     account isn't Spotify Premium, which Spotiamp requires.\n\n\
                     Please log in with a Premium account.",
                )
                .title("Spotiamp")
                .kind(MessageDialogKind::Warning)
                .show(|_| {});
        }

        log::debug!("Getting credentials");
        let credentials = match self.cache.credentials() {
            Some(credentials) => credentials,
            None => {
                log::debug!("No credentials in cache, starting OAuth flow...");
                Self::get_credentials_from_oauth(app).await?
            }
        };

        match self.connect(credentials).await {
            Ok(_) => {
                log::debug!("Connected");
                Ok(())
            }
            Err(e) => {
                log::warn!(
                    "Failed to connect with cached credentials ({e:?}), re-authenticating..."
                );
                Self::remove_cached_credentials();
                let new_credentials = Self::get_credentials_from_oauth(app).await?;
                self.connect(new_credentials)
                    .await
                    .map_err(|e| SessionError::ConnectError { e })?;
                log::debug!("Successfully connected after re-authentication");
                Ok(())
            }
        }
    }

    /// Connect, guarded by the sign-in marker (see [`account`]): librespot exits
    /// the process during this if the account isn't Premium.
    async fn connect(&self, credentials: Credentials) -> Result<(), Error> {
        account::mark_signing_in();
        let result = self.inner.connect(credentials, true).await;
        if result.is_ok() {
            account::wait_for_account_type(&self.inner).await;
        }
        account::clear_signing_in();
        result
    }

    fn remove_cached_credentials() {
        if let Some(config_dir) = get_config_dir() {
            let _ = std::fs::remove_file(config_dir.join("credentials.json"));
        }
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

        *pending_auth_url().lock().unwrap() = Some(auth_url.clone());

        let token_received = Arc::new(Mutex::new(false));
        let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);

        let window = tauri::WebviewWindowBuilder::new(
            app,
            "login",
            tauri::WebviewUrl::App("login-proxy".into()),
        )
        .title("Login")
        .inner_size(600.0, 800.0)
        .closable(true)
        .maximizable(false)
        .resizable(false)
        .build()
        .map_err(|e| SessionError::OpenURLFailed {
            url: auth_url.clone(),
            e,
        })?;

        window.on_window_event({
            let token_received = token_received.clone();
            let abort_tx = abort_tx.clone();
            move |e| {
                if let tauri::WindowEvent::CloseRequested { .. } = &e
                    && !*token_received.lock().unwrap()
                {
                    log::info!("No token received when closing login window. Aborting.");
                    let _ = abort_tx.send(true);
                }
            }
        });

        let result = oauth_flow.start(abort_rx).await;
        *token_received.lock().unwrap() = true;
        let _ = window.close();
        let token = result.map_err(|e| SessionError::TokenExchangeFailure { e })?;

        // Catch a non-Premium account before librespot exits the app over it.
        match account::check_in_helper_process(token.access_token().secret()).await {
            account::AccountCheck::Premium => log::info!("The account is Premium"),
            account::AccountCheck::NotPremium => return Err(SessionError::NotPremium),
            account::AccountCheck::Unknown(reason) => {
                log::warn!("Could not check the account type ({reason}), continuing")
            }
        }

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
            // Emit a position update every second so the UI can re-sync its
            // playback clock instead of free-running and drifting from the
            // actual position (which left the seek bar short at end of track).
            position_update_interval: Some(Duration::from_secs(1)),
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

    #[error(
        "Spotiamp requires a Spotify Premium account. Please start Spotiamp again and log in with a Premium account."
    )]
    NotPremium,
}

#[derive(Debug, Error)]
pub enum PlayError {
    #[error("Failed to fetch metadata ({e:?})")]
    MetadataError { e: Error },
    #[error("Cannot get track for non track id ({_0:?})")]
    GettingTrackForNonTrackUri(SpotifyUri),
}

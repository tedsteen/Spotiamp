use std::{
    collections::hash_map::DefaultHasher,
    fs::{File, create_dir_all},
    hash::{Hash, Hasher},
    io::{BufReader, BufWriter},
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tauri::LogicalPosition;

pub fn get_config_dir() -> Option<PathBuf> {
    let path =
        ProjectDirs::from("org.darkbits", "", "spotiamp").map(|pd| pd.config_dir().to_path_buf());
    if let Some(path) = path.clone() {
        if let Err(e) = create_dir_all(path) {
            log::error!("Could not create path: {:?}", e);
        }
    }
    path
}
fn get_settings_file_path() -> PathBuf {
    get_config_dir()
        .expect("a config directory")
        .join("settings.yaml")
}

pub struct AutoSavingSettings<'a> {
    inner: RwLockWriteGuard<'a, Settings>,
    hash_before: u64,
}

impl<'a> AutoSavingSettings<'a> {
    fn new(inner: &'a RwLock<Settings>) -> Self {
        let inner = inner.write().unwrap();
        AutoSavingSettings {
            hash_before: inner.get_hash(),
            inner,
        }
    }
}

impl Deref for AutoSavingSettings<'_> {
    type Target = Settings;

    fn deref(&self) -> &Settings {
        &self.inner
    }
}

impl DerefMut for AutoSavingSettings<'_> {
    fn deref_mut(&mut self) -> &mut Settings {
        &mut self.inner
    }
}

impl Drop for AutoSavingSettings<'_> {
    fn drop(&mut self) {
        if self.hash_before != self.inner.get_hash() {
            self.inner.save()
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Hash)]
pub struct OuterWindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct InnerWindowSize {
    pub width: u32,
    pub height: u32,
}

impl Default for InnerWindowSize {
    fn default() -> Self {
        Self {
            width: 275,
            height: 116,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Hash)]
pub struct WindowState {
    pub outer_position: Option<OuterWindowPosition>,
    pub inner_size: Option<InnerWindowSize>,
}

impl WindowState {
    pub fn get_position(&self) -> Option<LogicalPosition<i32>> {
        self.outer_position
            .as_ref()
            .map(|pos| LogicalPosition::new(pos.x, pos.y))
    }

    pub fn set_position(&mut self, position: LogicalPosition<i32>) {
        self.outer_position = Some(OuterWindowPosition {
            x: position.x,
            y: position.y,
        });
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Hash)]
pub struct PlaylistSettings {
    pub window_state: WindowState,
    pub uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct PlayerSettings {
    pub window_state: WindowState,
    pub double_size_active: bool,
    pub volume: u16,
    pub show_playlist: bool,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            window_state: Default::default(),
            double_size_active: Default::default(),
            volume: 80,
            show_playlist: true,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Hash)]
pub struct Settings {
    pub player: PlayerSettings,
    pub playlist: PlaylistSettings,
}

impl Settings {
    fn _current() -> &'static RwLock<Settings> {
        static MEM: OnceLock<RwLock<Settings>> = OnceLock::new();
        MEM.get_or_init(|| RwLock::new(Settings::load()))
    }

    pub fn current_mut<'a>() -> AutoSavingSettings<'a> {
        AutoSavingSettings::new(Self::_current())
    }

    pub fn current<'a>() -> RwLockReadGuard<'a, Settings> {
        Self::_current().read().unwrap()
    }

    fn load() -> Settings {
        let settings_file_path = get_settings_file_path();
        log::info!("Loading settings from '{settings_file_path:?}'");
        let mut settings: Result<Settings, String> = File::open(settings_file_path.clone())
            .map_err(|e| format!("Could not open file ({e:?}"))
            .and_then(|f| {
                serde_yaml::from_reader(BufReader::new(f))
                    .map_err(|e| format!("Could not deserialize file ({e:?})"))
            });

        if let Err(e) = &mut settings {
            log::warn!("Could not load settings ({settings_file_path:?}): {e:}")
        }
        //TODO: Check if the error is something else than file not found and log
        //eprintln!("Failed to load config ({err}), falling back to default settings");
        settings.unwrap_or_else(|e| {
            log::info!("Could not load a settings file ({e:?}, creating a new one");
            let mut new_settings = Settings::default();
            new_settings.playlist.uris = vec!["spotify:track:0DiWol3AO6WpXZgp0goxAV".to_string()];
            new_settings
        })
    }

    fn save(&self) {
        let settings_file_path = get_settings_file_path();
        if let Err(e) = File::create(settings_file_path.clone())
            .map_err(|e| format!("Could not create file ({e:?})"))
            .and_then(|file| {
                serde_yaml::to_writer(BufWriter::new(file), self)
                    .map_err(|e| format!("Could not serialize ({e:?})"))
            })
        {
            log::error!("Failed to save settings: {:?}", e);
        } else {
            log::debug!("Settings saved");
        }
    }

    fn get_hash(&self) -> u64 {
        let hasher = &mut DefaultHasher::new();
        self.hash(hasher);
        hasher.finish()
    }
}

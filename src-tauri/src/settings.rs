use directories::ProjectDirs;
use std::{fs, path::PathBuf};

pub fn get_config_dir() -> Option<PathBuf> {
    let path =
        ProjectDirs::from("", "spotiamp.com", "spotiamp").map(|pd| pd.config_dir().to_path_buf());
    if let Some(path) = path.clone() {
        if let Err(e) = fs::create_dir_all(path) {
            log::error!("Could not create path: {:?}", e);
        }
    }
    path
}

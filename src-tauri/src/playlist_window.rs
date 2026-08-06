use tauri::{AppHandle, LogicalPosition, WebviewWindow, State};
use oauth2::TokenResponse;

use crate::{
    app_window,
    settings::{InnerWindowSize, PlaylistSettings, Settings},
    spotify::SharedPlayer,
};

#[tauri::command]
pub async fn get_spotify_access_token(
    player: State<'_, SharedPlayer>,
) -> Result<String, String> {
    // 1. Check if we have a valid cached token in settings
    let (access_token, refresh_token, expires_at) = {
        let settings = Settings::current();
        (
            settings.oauth_token.access_token.clone(),
            settings.oauth_token.refresh_token.clone(),
            settings.oauth_token.expires_at,
        )
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if let Some(token) = access_token {
        let is_valid = match expires_at {
            Some(exp) => exp > now + 60,
            None => true,
        };
        if is_valid {
            return Ok(token);
        }
    }

    // 2. If the token is expired or missing, try to refresh it using the refresh token
    if let Some(r_token) = refresh_token {
        log::info!("Refreshing Spotify access token using stored refresh token...");
        let client_id = "d420a117a32841c2b3474932e49fb54b";
        
        let client = oauth2::basic::BasicClient::new(oauth2::ClientId::new(client_id.to_string()))
            .set_auth_uri(oauth2::AuthUrl::new("https://accounts.spotify.com/authorize".to_string()).unwrap())
            .set_token_uri(oauth2::TokenUrl::new("https://accounts.spotify.com/api/token".to_string()).unwrap());

        let http_client = oauth2::reqwest::ClientBuilder::new()
            .redirect(oauth2::reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {:?}", e))?;

        match client
            .exchange_refresh_token(&oauth2::RefreshToken::new(r_token))
            .request_async(&http_client)
            .await
        {
            Ok(token_response) => {
                let new_access = token_response.access_token().secret().to_string();
                let new_refresh = token_response.refresh_token().map(|t| t.secret().to_string());
                let expires_in = token_response.expires_in().unwrap_or(std::time::Duration::from_secs(3600));
                
                // Save new token to settings
                {
                    let mut settings = Settings::current_mut();
                    settings.oauth_token.access_token = Some(new_access.clone());
                    if let Some(ref_token) = new_refresh {
                        settings.oauth_token.refresh_token = Some(ref_token);
                    }
                    settings.oauth_token.expires_at = Some(now + expires_in.as_secs());
                }
                log::info!("Successfully refreshed Spotify access token");
                
                // Also update the memory cache in the player session
                {
                    let player = player.lock().await;
                    let mut guard = player.session.access_token.write().await;
                    *guard = Some(new_access.clone());
                }

                return Ok(new_access);
            }
            Err(e) => {
                log::error!("Failed to refresh Spotify token via OAuth: {:?}", e);
            }
        }
    }

    // 3. Fallback to keymaster / token provider
    log::info!("Attempting fallback to keymaster token provider");
    let player = player.lock().await;
    match player
        .session
        .inner
        .token_provider()
        .get_token_with_client_id(
            "playlist-read-private,playlist-read-collaborative,user-library-read",
            "d420a117a32841c2b3474932e49fb54b"
        )
        .await
    {
        Ok(token) => Ok(token.access_token),
        Err(e) => {
            log::warn!("Failed to get Spotify token from provider: {:?}", e);
            let guard = player.session.access_token.read().await;
            if let Some(token) = &*guard {
                log::info!("Falling back to memory-cached OAuth access token");
                Ok(token.clone())
            } else {
                Err(format!("Failed to get Spotify token: {:?}", e))
            }
        }
    }
}

#[tauri::command]
pub fn get_playlist_settings() -> PlaylistSettings {
    Settings::current().playlist.clone()
}

#[tauri::command]
pub fn set_uris(uris: Vec<String>) {
    Settings::current_mut().playlist.uris = uris;
}

#[tauri::command]
pub fn set_playlist_inner_size(width: u32, height: u32) {
    Settings::current_mut().playlist.window_state.inner_size =
        Some(InnerWindowSize { width, height });
}

pub fn build_window(
    app: &AppHandle,
    initial_position: LogicalPosition<i32>,
) -> Result<WebviewWindow, tauri::Error> {
    let inner_size = Settings::current()
        .playlist
        .window_state
        .inner_size
        .clone()
        .unwrap_or_default();

    let window =
        app_window::build_frameless_window(app, "playlist", "Playlist", "playlist", inner_size)?;

    app_window::apply_position(
        &window,
        Some(
            Settings::current()
                .playlist
                .window_state
                .get_position()
                .unwrap_or(initial_position),
        ),
    );
    app_window::remember_position(&window, "playlist window", |position| {
        Settings::current_mut()
            .playlist
            .window_state
            .set_position(position);
    });
    Ok(window)
}

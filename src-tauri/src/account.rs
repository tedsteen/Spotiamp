//! Spotiamp (librespot) only works with Spotify Premium accounts.
//!
//! librespot doesn't report a non-Premium account as an error: it calls
//! `exit(1)` as soon as it learns the account type, right after connecting. To
//! avoid that silent exit (and the saved login making it happen on every launch):
//!
//! * a fresh login asks the Web API for the account type before connecting, so
//!   a non-Premium account gets a proper message instead, and
//! * a marker file is written while connecting and removed once librespot has
//!   accepted the account. If it is still there on the next start, the previous
//!   sign-in died, so the saved login is cleared and the user is told why.

use std::{path::PathBuf, time::Duration};

use librespot::core::session::Session;
use oauth2::reqwest;

use crate::settings::get_config_dir;

const PREMIUM: &str = "premium";
const SIGN_IN_MARKER_FILE: &str = "signing-in";
const ACCOUNT_TYPE_TIMEOUT: Duration = Duration::from_secs(15);

/// The account's subscription ("premium", "free", ...) according to the Web API.
/// Needs the `user-read-private` scope.
pub async fn product(access_token: &str) -> Result<String, String> {
    let me: serde_json::Value = reqwest::Client::new()
        .get("https://api.spotify.com/v1/me")
        .bearer_auth(access_token)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
        .and_then(|body| serde_json::from_str(&body).map_err(|e| e.to_string()))?;
    me["product"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "no product in profile".to_string())
}

pub fn is_premium(product: &str) -> bool {
    product == PREMIUM
}

/// Wait until librespot has received the account type (it exits the process
/// right then if the account isn't Premium).
pub async fn wait_for_account_type(session: &Session) {
    let deadline = tokio::time::Instant::now() + ACCOUNT_TYPE_TIMEOUT;
    while !session.user_data().attributes.contains_key("type") {
        if tokio::time::Instant::now() >= deadline {
            log::warn!("Did not receive the account type within {ACCOUNT_TYPE_TIMEOUT:?}");
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    log::info!(
        "Signed in to a {:?} account",
        session.user_data().attributes.get("type")
    );
}

fn sign_in_marker() -> Option<PathBuf> {
    get_config_dir().map(|dir| dir.join(SIGN_IN_MARKER_FILE))
}

pub fn mark_signing_in() {
    if let Some(marker) = sign_in_marker()
        && let Err(e) = std::fs::write(&marker, "")
    {
        log::warn!("Could not write {marker:?} ({e:?})");
    }
}

pub fn clear_signing_in() {
    if let Some(marker) = sign_in_marker() {
        let _ = std::fs::remove_file(marker);
    }
}

/// True if the previous run died while signing in (the marker is removed).
pub fn previous_sign_in_died() -> bool {
    let died = sign_in_marker().is_some_and(|marker| marker.exists());
    clear_signing_in();
    died
}

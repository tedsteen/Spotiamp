//! Spotiamp (librespot) only works with Spotify Premium accounts.
//!
//! librespot doesn't report a non-Premium account as an error: it calls
//! `exit(1)` as soon as it learns the account type, right after connecting.
//! Before, that made Spotiamp vanish, and the saved login made it vanish on every
//! launch after that. So:
//!
//! * after a fresh login, a helper process (this binary started with
//!   [`ACCOUNT_CHECK_ARG`]) connects first and reports the account type. If
//!   librespot exits the helper, the account isn't Premium and the user is told
//!   so before the app itself connects.
//! * as a safety net, a marker file is written while connecting and removed once
//!   librespot has accepted the account. If it is still there on the next start,
//!   the previous sign-in died, so the saved login is cleared and the user is
//!   told why.

use std::{io::Read, path::PathBuf, process::Stdio, time::Duration};

use librespot::core::{authentication::Credentials, config::SessionConfig, session::Session};
use tokio::io::AsyncWriteExt;

use crate::settings::get_config_dir;

/// Command-line flag that makes the binary run [`run_account_check`] instead of the app.
pub const ACCOUNT_CHECK_ARG: &str = "--check-account-type";

const SIGN_IN_MARKER_FILE: &str = "signing-in";
const ACCOUNT_TYPE_TIMEOUT: Duration = Duration::from_secs(15);
const ACCOUNT_CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// librespot's exit code when the account isn't Premium.
const LIBRESPOT_REJECTED_EXIT_CODE: i32 = 1;
const ACCOUNT_CHECK_FAILED_EXIT_CODE: i32 = 2;

#[derive(Debug, PartialEq)]
pub enum AccountCheck {
    Premium,
    NotPremium,
    /// The check itself failed (network, timeout, ...).
    Unknown(String),
}

/// Find out the account type of a freshly logged-in account without risking
/// librespot exiting this process (see the module docs).
pub async fn check_in_helper_process(access_token: &str) -> AccountCheck {
    match run_helper(access_token).await {
        Ok((exit_code, stdout)) => interpret(exit_code, &stdout),
        Err(e) => AccountCheck::Unknown(e),
    }
}

async fn run_helper(access_token: &str) -> Result<(Option<i32>, String), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = tokio::process::Command::new(exe)
        .arg(ACCOUNT_CHECK_ARG)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("could not start the helper ({e})"))?;

    // The token goes over stdin so it doesn't show up in the process list.
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    stdin
        .write_all(access_token.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    drop(stdin);

    let output = tokio::time::timeout(ACCOUNT_CHECK_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "the helper timed out".to_string())?
        .map_err(|e| e.to_string())?;
    Ok((
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

/// What the helper's exit code and output (the account type) mean.
fn interpret(exit_code: Option<i32>, stdout: &str) -> AccountCheck {
    match (exit_code, stdout.trim()) {
        (Some(0), "premium") => AccountCheck::Premium,
        // librespot exits before the helper prints anything.
        (Some(LIBRESPOT_REJECTED_EXIT_CODE), "") => AccountCheck::NotPremium,
        (exit_code, output) => {
            AccountCheck::Unknown(format!("exit code {exit_code:?}, output {output:?}"))
        }
    }
}

/// Entry point of the helper process: reads an access token from stdin, connects
/// and prints the account type. librespot exits it with code 1 if the account
/// isn't Premium.
pub fn run_account_check() -> ! {
    let mut access_token = String::new();
    if std::io::stdin().read_to_string(&mut access_token).is_err() {
        std::process::exit(ACCOUNT_CHECK_FAILED_EXIT_CODE);
    }
    let exit_code = tauri::async_runtime::block_on(async {
        let session = Session::new(SessionConfig::default(), None);
        let credentials = Credentials::with_access_token(access_token.trim());
        if let Err(e) = session.connect(credentials, false).await {
            log::error!("Account check could not connect ({e:?})");
            return ACCOUNT_CHECK_FAILED_EXIT_CODE;
        }
        wait_for_account_type(&session).await;
        match session.user_data().attributes.get("type") {
            Some(account_type) => {
                println!("{account_type}");
                0
            }
            None => ACCOUNT_CHECK_FAILED_EXIT_CODE,
        }
    });
    std::process::exit(exit_code)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interprets_the_helper_result() {
        assert_eq!(interpret(Some(0), "premium\n"), AccountCheck::Premium);
        assert_eq!(interpret(Some(1), ""), AccountCheck::NotPremium);
        assert!(matches!(interpret(Some(2), ""), AccountCheck::Unknown(_)));
        assert!(matches!(interpret(None, ""), AccountCheck::Unknown(_)));
        assert!(matches!(interpret(Some(0), ""), AccountCheck::Unknown(_)));
    }
}

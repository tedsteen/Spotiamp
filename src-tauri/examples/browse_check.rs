//! Manual check of `spotiamp_lib::browse` against the real Spotify servers.
//!
//! Usage:
//!   cargo run --example browse_check -- "daft punk"
//!
//! Logs in through librespot's own OAuth flow (opens a browser) the first time and
//! keeps reusable credentials in `BROWSE_CHECK_CACHE_DIR` (default: a temp dir), so it never
//! touches Spotiamp's real config/cache.

use std::path::PathBuf;

use librespot::core::{
    authentication::Credentials, cache::Cache, config::SessionConfig, session::Session,
};
use librespot::oauth::OAuthClientBuilder;
use librespot::protocol::playlist4_external::SelectedListContent;
use protobuf::Message;
use spotiamp_lib::browse::{self, SearchKind};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("A crypto provider");
    tauri::async_runtime::block_on(run());
}

async fn run() {
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "daft punk".to_string());
    let dir = std::env::var("BROWSE_CHECK_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("spotiamp-browse-check"));
    std::fs::create_dir_all(&dir).expect("cache dir");

    let cache = Cache::new(Some(dir.clone()), None, None, None).expect("a cache");
    let config = SessionConfig::default();
    let credentials = match cache.credentials() {
        Some(c) => c,
        None => {
            println!("No cached credentials, opening browser for Spotify login...");
            let client = OAuthClientBuilder::new(
                &config.client_id,
                "http://127.0.0.1:8898/login",
                vec!["streaming"],
            )
            .open_in_browser()
            .build()
            .expect("oauth client");
            let token = client
                .get_access_token_async()
                .await
                .expect("oauth access token");
            Credentials::with_access_token(token.access_token)
        }
    };
    let session = Session::new(config, Some(cache));
    session
        .connect(credentials, true)
        .await
        .expect("session connect");

    raw_rootlist_layout(&session).await;

    println!("=== browse::my_playlists ===");
    match browse::my_playlists(&session).await {
        Ok(items) => {
            println!("{} playlists", items.len());
            for i in items.iter().take(15) {
                println!(
                    "  {:<40} {:<20} {:>11}  {}",
                    i.name, i.subtitle, i.info, i.uri
                );
            }
        }
        Err(e) => println!("FAILED: {e} ({e:?})"),
    }

    for kind in [
        SearchKind::Tracks,
        SearchKind::Albums,
        SearchKind::Playlists,
    ] {
        println!("\n=== browse::search {kind:?} \"{query}\" ===");
        match browse::search(&session, kind, &query, 0).await {
            Ok(page) => {
                println!(
                    "{} items, next_offset {:?}",
                    page.items.len(),
                    page.next_offset
                );
                for i in page.items.iter().take(5) {
                    println!(
                        "  {:<40} {:<25} {:>6} {}  {}",
                        i.name,
                        i.subtitle,
                        i.info,
                        if i.playable { " " } else { "x" },
                        i.uri
                    );
                }
                if let Some(next) = page.next_offset {
                    match browse::search(&session, kind, &query, next).await {
                        Ok(p2) => println!(
                            "  page 2: {} items, first: {:?}",
                            p2.items.len(),
                            p2.items.first().map(|i| &i.name)
                        ),
                        Err(e) => println!("  page 2 FAILED: {e}"),
                    }
                }
            }
            Err(e) => println!("FAILED: {e} ({e:?})"),
        }
    }
}

/// Shows how rootlist `items` and `meta_items` line up (folders included).
async fn raw_rootlist_layout(session: &Session) {
    println!("=== raw rootlist layout ===");
    let bytes = session
        .spclient()
        .get_rootlist(0, Some(500))
        .await
        .expect("rootlist");
    let msg = SelectedListContent::parse_from_bytes(&bytes).expect("protobuf");
    let c = msg.contents.get_or_default();
    println!(
        "total length {}, items {}, meta_items {}",
        msg.length(),
        c.items.len(),
        c.meta_items.len()
    );
    for (i, item) in c.items.iter().enumerate() {
        if !item.uri().starts_with("spotify:playlist:")
            || c.meta_items
                .get(i)
                .is_none_or(|m| m.attributes.name().is_empty())
        {
            println!(
                "  [{i}] {} -> meta name {:?}",
                item.uri(),
                c.meta_items.get(i).map(|m| m.attributes.name())
            );
        }
    }
    println!();
}

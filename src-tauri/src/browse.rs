//! Browsing Spotify (search + the user's own playlists) through the librespot
//! session, the same way the official clients do. No Web API client ID or extra
//! OAuth scopes are needed, and no token ever leaves the backend.
//!
//! * Search goes through Pathfinder, Spotify's internal GraphQL API, using the
//!   session's Login5 token and client token.
//! * "My playlists" is the user's rootlist, fetched over spclient.
//!
//! Pathfinder requests are identified by persisted-query hashes taken from the
//! Spotify web player. Spotify rotates these from time to time; when that happens
//! search fails with [`BrowseError::SearchOutdated`] and the hashes below need
//! updating (search for `"searchTracks","query","` in the web player's
//! `xpui-routes-search` chunk). `cargo run --example browse_check -- "<query>"`
//! exercises this module against the real servers.

use std::sync::OnceLock;

use librespot::{core::session::Session, protocol::playlist4_external::SelectedListContent};
use oauth2::reqwest;
use protobuf::Message;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

const PATHFINDER_URL: &str = "https://api-partner.spotify.com/pathfinder/v2/query";
const SEARCH_PAGE_SIZE: u32 = 30;
const ROOTLIST_PAGE_SIZE: usize = 100;
const ROOTLIST_MAX: usize = 2000;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchKind {
    Tracks,
    Albums,
    Playlists,
}

impl SearchKind {
    fn operation(self) -> (&'static str, &'static str) {
        match self {
            SearchKind::Tracks => (
                "searchTracks",
                "b02683192a98dde7966b5e6655a79eeb62713eab703eda9902c932818dd52751",
            ),
            SearchKind::Albums => (
                "searchAlbums",
                "202cb3305e31e5a0767ba7925f28bd728cf8f8b0217e6da43909056071cd70e9",
            ),
            SearchKind::Playlists => (
                "searchPlaylists",
                "d520014e748f9ea44f7707d8df1819867ac1205e8b7f3e28f22fe5fc858921b1",
            ),
        }
    }
}

/// One row in the browser: a track, album or playlist.
#[derive(Debug, Clone, Serialize)]
pub struct BrowseItem {
    pub uri: String,
    pub name: String,
    /// Artists for tracks/albums, owner for playlists.
    pub subtitle: String,
    /// Track length (tracks), release year (albums) or track count (own playlists).
    pub info: String,
    pub playable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchPage {
    pub items: Vec<BrowseItem>,
    pub next_offset: Option<u32>,
}

#[derive(Debug, Error)]
pub enum BrowseError {
    #[error("Could not get a Spotify token ({0})")]
    Token(String),
    #[error("Could not reach Spotify ({0})")]
    Request(#[from] reqwest::Error),
    #[error("Spotify search is unavailable (HTTP {status})")]
    Status { status: u16, body: String },
    #[error("Spotify changed its search API, Spotiamp needs an update")]
    SearchOutdated,
    #[error("Unexpected search response ({0})")]
    Response(String),
    #[error("Could not load your playlists ({0})")]
    Rootlist(String),
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

pub async fn search(
    session: &Session,
    kind: SearchKind,
    query: &str,
    offset: u32,
) -> Result<SearchPage, BrowseError> {
    let (operation, hash) = kind.operation();
    let variables = json!({
        "searchTerm": query,
        "offset": offset,
        "limit": SEARCH_PAGE_SIZE,
        "numberOfTopResults": 5,
        "includeAudiobooks": false,
        "includePreReleases": false,
        "includeAlbumPreReleases": false,
        "includeAuthors": false,
        "includeEpisodeContentRatingsV2": false,
    });
    let response = pathfinder(session, operation, hash, variables).await?;
    let search = &response["data"]["searchV2"];

    let (list, parse): (&Value, fn(&Value) -> Option<BrowseItem>) = match kind {
        SearchKind::Tracks => (&search["tracksV2"], parse_track),
        SearchKind::Albums => (&search["albumsV2"], parse_album),
        SearchKind::Playlists => (&search["playlists"], parse_playlist),
    };
    let Some(items) = list["items"].as_array() else {
        return Err(BrowseError::Response(format!(
            "no result list in {operation} response"
        )));
    };

    Ok(SearchPage {
        items: items.iter().filter_map(parse).collect(),
        next_offset: list["pagingInfo"]["nextOffset"].as_u64().map(|o| o as u32),
    })
}

async fn pathfinder(
    session: &Session,
    operation: &str,
    hash: &str,
    variables: Value,
) -> Result<Value, BrowseError> {
    let token = session
        .login5()
        .auth_token()
        .await
        .map_err(|e| BrowseError::Token(e.to_string()))?
        .access_token;
    let client_token = session
        .spclient()
        .client_token()
        .await
        .map_err(|e| BrowseError::Token(e.to_string()))?;

    let body = json!({
        "variables": variables,
        "operationName": operation,
        "extensions": { "persistedQuery": { "version": 1, "sha256Hash": hash } },
    });

    let response = http_client()
        .post(PATHFINDER_URL)
        .bearer_auth(token)
        .header("client-token", client_token)
        .header("content-type", "application/json;charset=UTF-8")
        .header("accept", "application/json")
        .body(body.to_string())
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;

    if text.contains("PersistedQueryNotFound") {
        log::error!("Pathfinder {operation} hash {hash} is no longer accepted by Spotify");
        return Err(BrowseError::SearchOutdated);
    }
    if !status.is_success() {
        log::error!("Pathfinder {operation} failed with {status}: {text}");
        return Err(BrowseError::Status {
            status: status.as_u16(),
            body: text,
        });
    }

    let value: Value =
        serde_json::from_str(&text).map_err(|e| BrowseError::Response(e.to_string()))?;
    if value["data"].is_null() {
        log::error!("Pathfinder {operation} returned no data: {text}");
        return Err(BrowseError::Response(
            value["errors"][0]["message"]
                .as_str()
                .unwrap_or("no data")
                .to_string(),
        ));
    }
    Ok(value)
}

fn str_of(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

fn artist_names(artists: &Value) -> String {
    artists["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| a["profile"]["name"].as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_playable(data: &Value) -> bool {
    data["playability"]["playable"].as_bool().unwrap_or(true)
}

fn parse_track(item: &Value) -> Option<BrowseItem> {
    let data = &item["item"]["data"];
    let uri = data["uri"].as_str()?;
    let ms = data["duration"]["totalMilliseconds"].as_u64().unwrap_or(0);
    Some(BrowseItem {
        uri: uri.to_string(),
        name: str_of(&data["name"]),
        subtitle: artist_names(&data["artists"]),
        info: format!("{}:{:02}", ms / 60_000, (ms / 1000) % 60),
        playable: is_playable(data),
    })
}

fn parse_album(item: &Value) -> Option<BrowseItem> {
    let data = &item["data"];
    let uri = data["uri"].as_str()?;
    Some(BrowseItem {
        uri: uri.to_string(),
        name: str_of(&data["name"]),
        subtitle: artist_names(&data["artists"]),
        info: data["date"]["year"]
            .as_u64()
            .map(|y| y.to_string())
            .unwrap_or_default(),
        playable: is_playable(data),
    })
}

fn parse_playlist(item: &Value) -> Option<BrowseItem> {
    let data = &item["data"];
    let uri = data["uri"].as_str()?;
    Some(BrowseItem {
        uri: uri.to_string(),
        name: str_of(&data["name"]),
        subtitle: str_of(&data["ownerV2"]["data"]["name"]),
        info: String::new(),
        playable: true,
    })
}

/// The playlists in the user's library ("rootlist"), in the user's order.
/// Folders are flattened.
pub async fn my_playlists(session: &Session) -> Result<Vec<BrowseItem>, BrowseError> {
    let mut playlists = Vec::new();
    let mut from = 0;
    loop {
        let bytes = session
            .spclient()
            .get_rootlist(from, Some(ROOTLIST_PAGE_SIZE))
            .await
            .map_err(|e| BrowseError::Rootlist(e.to_string()))?;
        let list = SelectedListContent::parse_from_bytes(&bytes)
            .map_err(|e| BrowseError::Rootlist(e.to_string()))?;
        let contents = list.contents.get_or_default();
        let page_len = contents.items.len();

        // `meta_items` is parallel to `items`, so pair them up before dropping
        // the folder markers (spotify:start-group / spotify:end-group).
        for (index, item) in contents.items.iter().enumerate() {
            if !item.uri().starts_with("spotify:playlist:") {
                continue;
            }
            let meta = contents.meta_items.get(index);
            let name = match meta.map(|m| m.attributes.name()).unwrap_or_default() {
                "" => "(untitled)".to_string(),
                name => name.to_string(),
            };
            playlists.push(BrowseItem {
                uri: item.uri().to_string(),
                name,
                subtitle: meta
                    .map(|m| m.owner_username().to_string())
                    .unwrap_or_default(),
                info: meta
                    .filter(|m| m.has_length())
                    .map(|m| format!("{} tracks", m.length()))
                    .unwrap_or_default(),
                playable: true,
            });
        }

        from += page_len;
        let total = list.length().max(0) as usize;
        if page_len == 0 || from >= total || from >= ROOTLIST_MAX {
            break;
        }
    }
    Ok(playlists)
}

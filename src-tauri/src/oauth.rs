use oauth2::{
    basic::BasicClient, reqwest, AuthUrl, AuthorizationCode, ClientId, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenUrl,
};
use serde::Deserialize;
use std::net::{SocketAddr, TcpListener};
use thiserror::Error;
use tokio::sync::broadcast::Sender;
use url::{ParseError, Url};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("Invalid Auth URI {uri} ({e})")]
    InvalidAuthUri { uri: String, e: ParseError },

    #[error("Invalid Token URI {uri} ({e})")]
    InvalidTokenUri { uri: String, e: ParseError },

    #[error("Invalid Redirect URI {uri} ({e})")]
    InvalidRedirectUri { uri: String, e: ParseError },

    #[error("Failed to exchange code for access token ({e})")]
    ExchangeCode { e: String },

    #[error("Failed to receive code ({e})")]
    Recv { e: String },

    #[error("Failed to setup local authentication server ({e})")]
    CouldNotStartServer { e: std::io::Error },
}
type Client = oauth2::Client<
    oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
    oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>,
    oauth2::StandardTokenIntrospectionResponse<
        oauth2::EmptyExtraTokenFields,
        oauth2::basic::BasicTokenType,
    >,
    oauth2::StandardRevocableToken,
    oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>,
    oauth2::EndpointSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointSet,
>;

pub struct OAuthFlow {
    auth_url: Url,
    socket_addr: SocketAddr,
    pub client: Client,
    pkce_verifier: oauth2::PkceCodeVerifier,
}

type TokenType =
    oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>;

impl OAuthFlow {
    pub fn new(auth_url: &str, token_url: &str, client_id: &str) -> Result<Self, OAuthError> {
        log::debug!("Creating OAuth flow");
        let auth_url =
            AuthUrl::new(auth_url.to_string()).map_err(|e| OAuthError::InvalidAuthUri {
                uri: auth_url.to_string(),
                e,
            })?;

        let token_url =
            TokenUrl::new(token_url.to_string()).map_err(|e| OAuthError::InvalidTokenUri {
                uri: token_url.to_string(),
                e,
            })?;

        let socket_addr = Self::get_available_addr();
        let redirect_uri = format!("http://{socket_addr}/login");
        let redirect_url = RedirectUrl::new(redirect_uri.to_string()).map_err(|e| {
            OAuthError::InvalidRedirectUri {
                uri: redirect_uri.to_string(),
                e,
            }
        })?;

        let client = BasicClient::new(ClientId::new(client_id.to_string()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_url);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (auth_url, _) = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(vec![Scope::new("streaming".to_string())])
            .set_pkce_challenge(pkce_challenge)
            .url();

        Ok(Self {
            auth_url,
            socket_addr,
            client,
            pkce_verifier,
        })
    }

    fn get_available_addr() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        addr
    }

    pub async fn start(self) -> Result<TokenType, OAuthError> {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<AuthorizationCode>(1);

        #[derive(Deserialize)]
        struct Params {
            #[serde(default)]
            code: String,
        }

        let app = Router::new()
            .route(
                "/login",
                get(
                    |Query(params): Query<Params>,
                     State(mut tx): State<Option<Sender<AuthorizationCode>>>| async move {
                        if let Some(tx) = tx.take() {
                            let _ = tx.send(AuthorizationCode::new(params.code));
                            Html("<body></body>").into_response()
                        } else {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Redirect URL already used",
                            )
                                .into_response()
                        }
                    },
                ),
            )
            .with_state(Some(tx.clone()));

        log::info!("Waiting for OAuth redirect server to receive callback...");
        let listener = tokio::net::TcpListener::bind(self.socket_addr)
            .await
            .map_err(|e| OAuthError::CouldNotStartServer { e })?;
        axum::serve(listener, app)
            .with_graceful_shutdown({
                let mut rx = tx.subscribe();
                async move {
                    let _ = rx.recv().await;
                }
            })
            .await
            .map_err(|e| OAuthError::CouldNotStartServer { e })?;

        let code = rx
            .recv()
            .await
            .map_err(|e| OAuthError::Recv { e: e.to_string() })?;
        log::debug!("Doing the exchange...");
        let http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Client should build");
        self.client
            .exchange_code(code)
            .set_pkce_verifier(self.pkce_verifier)
            .request_async(&http_client)
            .await
            .map_err(|e| OAuthError::Recv { e: e.to_string() })
    }

    pub(crate) fn get_auth_url(&self) -> String {
        self.auth_url.to_string()
    }
}

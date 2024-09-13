//! This module provides the entry point for interacting with the Factory+ services.
//!
//! ServiceClient holds the service interfaces, credentials, and service urls.

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::error::FetchError;
use crate::service::auth::AuthInterface;
use crate::service::cmdesc::CmdEscInterface;
use crate::service::configdb::ConfigDbInterface;
use crate::service::directory::DirectoryInterface;
use crate::service::mqtt::MQTTInterface;
use crate::service::request::{FetchOpts, HttpRequestMethod};
use crate::service::response::{FetchResponse, PingResponse, TokenStruct};
use crate::uuids;

pub mod auth;
mod cmdesc;
pub mod configdb;
pub mod directory;
pub mod discovery;
pub mod mqtt;

/// Complex type to hold tokens in flight.
pub type InFlightTokensMap =
    HashMap<String, Pin<Box<dyn Future<Output = Result<TokenStruct, FetchError>> + Send>>>;

/// Struct to hold the Factory+ service interfaces and service urls.
pub struct ServiceClient {
    tokens: Arc<Mutex<HashMap<ServiceType, TokenStruct>>>,
    http_client: Arc<reqwest::Client>,

    pub auth_interface: AuthInterface,
    pub config_db_interface: ConfigDbInterface,
    pub directory_interface: DirectoryInterface,
    pub mqtt_interface: MQTTInterface,
    pub cmd_esc_interface: CmdEscInterface,

    service_creds: ServiceCreds,
    pub root_principle: Option<String>,
    pub permission_group: Option<String>,
}

impl ServiceClient {
    /// Create a new `ServiceClient` from the given credentials and urls.
    pub async fn from(
        service_username: &str,
        service_password: &str,
        root_principle: Option<&str>,
        permission_group: Option<&str>,
        directory_url: &str,
    ) -> Self {
        let client = Arc::new(reqwest::Client::new());
        let tokens = Arc::new(Mutex::new(HashMap::new()));

        let directory_interface = DirectoryInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            String::from(directory_url),
            Arc::clone(&tokens),
        );

        let configdb_urls = directory_interface
            .service_urls(ServiceType::ConfigDb)
            .await
            .unwrap();
        let mqtt_urls = directory_interface
            .service_urls(ServiceType::MQTT)
            .await
            .unwrap();
        let auth_urls = directory_interface
            .service_urls(ServiceType::Authentication)
            .await
            .unwrap();
        let cmd_esc_urls = directory_interface
            .service_urls(ServiceType::CommandEscalation)
            .await
            .unwrap();

        let config_db_interface = ConfigDbInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            String::from(directory_url),
            configdb_urls.unwrap().first().unwrap().clone(),
            Arc::clone(&tokens),
        );

        let mqtt_interface = MQTTInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            mqtt_urls.unwrap().first().unwrap().clone(),
            Arc::clone(&tokens),
        );

        let auth_interface = AuthInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            String::from(directory_url),
            auth_urls.unwrap().first().unwrap().clone(),
            Arc::clone(&tokens),
        );

        let cmd_esc_interface = CmdEscInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            cmd_esc_urls.unwrap().first().unwrap().clone(),
            Arc::clone(&tokens),
        );

        ServiceClient {
            tokens,
            http_client: Arc::clone(&client),

            service_creds: ServiceCreds::from(service_username, service_password),
            root_principle: root_principle.map(String::from),
            permission_group: permission_group.map(String::from),

            auth_interface,
            config_db_interface,
            directory_interface,
            mqtt_interface,
            cmd_esc_interface,
        }
    }

    /// Pings the given service. If the ping was successful, you should obtain a PingResponse with
    /// http::StatusCode::OK.
    ///
    /// As a side effect, this function gets a new token for authentication against the given
    /// service.
    pub async fn ping(&self, service: ServiceType) -> Result<PingResponse, FetchError> {
        let service_url = match service {
            ServiceType::Directory => self.directory_interface.service_url.clone(),
            ServiceType::ConfigDb => self.config_db_interface.service_url.clone(),
            ServiceType::Authentication => self.auth_interface.service_url.clone(),
            ServiceType::MQTT => self.mqtt_interface.service_url.clone(),
            ServiceType::CommandEscalation => self.cmd_esc_interface.service_url.clone(),
        };

        let ping_url = format!("{}/ping", service_url);

        let fetch_opts = FetchOpts {
            url: ping_url.clone(),
            service,
            method: HttpRequestMethod::GET,
            headers: Default::default(),
            query: None,
            body: None,
        };

        match self.fetch(fetch_opts).await {
            Ok(response) => Ok(response.into()),
            Err(e) => Err(e),
        }
    }

    pub async fn fetch(&self, fetch_opts: FetchOpts) -> Result<FetchResponse, FetchError> {
        let current_service_token = self
            .get_service_token(
                Arc::clone(&self.http_client),
                fetch_opts.service,
                &self.service_creds.service_username,
                &self.service_creds.service_password,
                &self.tokens,
            )
            .await?;

        let headers =
            utils::check_correct_headers(&fetch_opts.headers, &fetch_opts.body, &fetch_opts.url)?;

        if let Ok(request) = match (fetch_opts.query, fetch_opts.body) {
            (None, None) => self
                .http_client
                .request(fetch_opts.method.to_method(), fetch_opts.url.clone())
                .headers(headers),
            (Some(query), None) => self
                .http_client
                .request(fetch_opts.method.to_method(), fetch_opts.url.clone())
                .headers(headers)
                .query(&query),
            (None, Some(body)) => self
                .http_client
                .request(fetch_opts.method.to_method(), fetch_opts.url.clone())
                .headers(headers)
                .body(body),
            (Some(query), Some(body)) => self
                .http_client
                .request(fetch_opts.method.to_method(), fetch_opts.url.clone())
                .headers(headers)
                .query(&query)
                .body(body),
        }
        .bearer_auth(current_service_token.token)
        .build()
        {
            match self.http_client.execute(request).await {
                Ok(response) => {
                    let response_status = response.status();

                    if let Ok(response_body) = response.text().await {
                        Ok(FetchResponse {
                            status: response_status,
                            content: response_body,
                        })
                    } else {
                        Err(FetchError {
                            message: String::from("Couldn't decode response body."),
                            url: fetch_opts.url,
                        })
                    }
                }
                _ => Err(FetchError {
                    message: String::from("Couldn't make request."),
                    url: fetch_opts.url,
                }),
            }
        } else {
            Err(FetchError {
                message: String::from("Couldn't build a request to fetch."),
                url: fetch_opts.url,
            })
        }
    }

    pub async fn re_auth_service(&self, service: ServiceType) -> Result<TokenStruct, FetchError> {
        let service_url = match service {
            ServiceType::Directory => self.directory_interface.service_url.clone(),
            ServiceType::ConfigDb => self.config_db_interface.service_url.clone(),
            ServiceType::Authentication => self.auth_interface.service_url.clone(),
            ServiceType::MQTT => self.mqtt_interface.service_url.clone(),
            ServiceType::CommandEscalation => self.cmd_esc_interface.service_url.clone(),
        };

        let new_token = fetch_util::get_new_token(
            Arc::clone(&self.http_client),
            service_url,
            &self.service_creds.service_username,
            &self.service_creds.service_password,
        )
        .await?;

        self.tokens.lock().await.insert(service, new_token.clone());

        Ok(new_token)
    }
    async fn get_service_token(
        &self,
        client: Arc<reqwest::Client>,
        service: ServiceType,
        username: &String,
        password: &String,
        tokens: &Arc<Mutex<HashMap<ServiceType, TokenStruct>>>,
    ) -> Result<TokenStruct, FetchError> {
        let mut locked_tokens = tokens.lock().await;
        // If we find a local token, return it. Otherwise, we request a new one.
        if let Some(token) = locked_tokens.get(&service) {
            Ok(token.clone())
        } else {
            let service_url = match service {
                ServiceType::Directory => self.directory_interface.service_url.clone(),
                ServiceType::ConfigDb => self.config_db_interface.service_url.clone(),
                ServiceType::Authentication => self.auth_interface.service_url.clone(),
                ServiceType::MQTT => self.mqtt_interface.service_url.clone(),
                ServiceType::CommandEscalation => self.cmd_esc_interface.service_url.clone(),
            };
            let new_token =
                fetch_util::get_new_token(client, service_url.clone(), username, password).await?;
            locked_tokens.insert(service, new_token.clone());
            Ok(new_token)
        }
    }

    pub async fn show_tokens(&self) {
        println!("{:?}", self.tokens.lock().await);
    }
}

pub struct ServiceCreds {
    service_username: String,
    service_password: String,
}

impl ServiceCreds {
    pub fn new() -> Self {
        ServiceCreds {
            service_username: String::new(),
            service_password: String::new(),
        }
    }

    pub fn from(user_str: &str, pass_str: &str) -> Self {
        ServiceCreds {
            service_username: String::from(user_str),
            service_password: String::from(pass_str),
        }
    }
}

pub mod utils {
    use http::header;

    use crate::error::FetchError;

    /// Checks the validity of header values for the type of request.
    /// Returns a new reqwest::header::HeaderMap with valid headers.
    pub fn check_correct_headers(
        headers: &reqwest::header::HeaderMap,
        body: &Option<String>,
        url: &String,
    ) -> Result<reqwest::header::HeaderMap, FetchError> {
        // Ensure headers are set correctly for the type of request
        let mut local_headers = headers.clone();
        local_headers.entry(header::ACCEPT).or_insert({
            let maybe_header_val = header::HeaderValue::from_str("application/json");
            if let Ok(header_val) = maybe_header_val {
                header_val
            } else {
                return Err(FetchError {
                    message: String::from("Couldn't create correct header values."),
                    url: url.clone(),
                });
            }
        });
        if body.is_some() {
            local_headers.entry(header::CONTENT_TYPE).or_insert({
                let maybe_header_val = header::HeaderValue::from_str("application/json");
                if let Ok(header_val) = maybe_header_val {
                    header_val
                } else {
                    return Err(FetchError {
                        message: String::from("Couldn't create correct header values."),
                        url: url.clone(),
                    });
                }
            });
        }

        Ok(local_headers)
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum ServiceType {
    Directory,
    ConfigDb,
    Authentication,
    MQTT,
    CommandEscalation,
}

impl ServiceType {
    pub fn to_uuid(&self) -> uuid::Uuid {
        match &self {
            ServiceType::Directory => uuids::service::DIRECTORY,
            ServiceType::ConfigDb => uuids::service::CONFIG_DB,
            ServiceType::Authentication => uuids::service::AUTHENTICATION,
            ServiceType::MQTT => uuids::service::MQTT,
            ServiceType::CommandEscalation => uuids::service::COMMAND_ESCALATION,
        }
    }
}

impl Display for ServiceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} service ({})",
            match &self {
                ServiceType::Directory => "Directory",
                ServiceType::ConfigDb => "ConfigDb",
                ServiceType::Authentication => "Authentication",
                ServiceType::MQTT => "MQTT",
                ServiceType::CommandEscalation => "CommandEscalation",
            },
            self.to_uuid()
        )
    }
}

pub mod request {
    //! Contains request representations and implementations.
    use std::collections::HashMap;

    use crate::service::ServiceType;

    #[derive(Clone)]
    pub struct FetchOpts {
        pub url: String,
        pub service: ServiceType,
        pub method: HttpRequestMethod,
        pub headers: reqwest::header::HeaderMap,
        pub query: Option<HashMap<String, String>>,
        pub body: Option<String>,
    }

    impl FetchOpts {
        pub fn new() -> Self {
            FetchOpts {
                url: String::new(),
                service: ServiceType::Directory,
                method: HttpRequestMethod::GET,
                headers: reqwest::header::HeaderMap::new(),
                query: None,
                body: None,
            }
        }
    }

    /// HttpRequestMethod defines the subset of methods supported by this implementation.
    ///
    /// to_method() converts a HttpRequestMethod to a http::Method.
    #[derive(PartialEq, Eq, Clone)]
    pub enum HttpRequestMethod {
        GET,
        POST,
        PUT,
        PATCH,
        DELETE,
        HEAD,
    }

    impl HttpRequestMethod {
        // Map to a well-defined method from http::Method
        pub fn to_method(&self) -> http::Method {
            match self {
                HttpRequestMethod::GET => http::Method::GET,
                HttpRequestMethod::POST => http::Method::POST,
                HttpRequestMethod::PUT => http::Method::PUT,
                HttpRequestMethod::PATCH => http::Method::PATCH,
                HttpRequestMethod::DELETE => http::Method::DELETE,
                HttpRequestMethod::HEAD => http::Method::HEAD,
            }
        }
    }
}

pub mod response {
    //! Contains response representations and implementations.
    use serde::Deserialize;

    #[derive(Debug)]
    pub struct FetchResponse {
        pub status: http::StatusCode,
        pub content: String,
    }

    impl FetchResponse {
        pub fn from(status: http::StatusCode, content: String) -> Self {
            FetchResponse { status, content }
        }
    }

    #[derive(Debug)]
    pub struct PingResponse {
        pub status: http::StatusCode,
        pub content: Option<String>,
    }

    impl PingResponse {
        pub fn from(status: http::StatusCode, content: String) -> Self {
            PingResponse {
                status,
                content: Some(content),
            }
        }
    }

    impl From<FetchResponse> for PingResponse {
        fn from(value: FetchResponse) -> Self {
            PingResponse::from(value.status, value.content)
        }
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct TokenStruct {
        pub token: String,
        pub expiry: u64,
    }

    impl TokenStruct {
        pub fn from(token: String, expiry: u64) -> Self {
            TokenStruct { token, expiry }
        }
    }
}

pub(in crate::service) mod fetch_util {
    //! Contains utilities used by fetch().
    use std::sync::Arc;

    use serde_json;

    use crate::error::FetchError;
    use crate::service::request::{FetchOpts, HttpRequestMethod};
    use crate::service::response::TokenStruct;

    pub(crate) async fn get_new_token(
        client: Arc<reqwest::Client>,
        service_url: String,
        username: &String,
        password: &String,
    ) -> Result<TokenStruct, FetchError> {
        let token_url = format!("{}/token", service_url);
        if let Ok(request) = client
            .post(token_url.clone())
            .basic_auth(username, Some(password))
            .build()
        {
            if let Ok(response) = client.execute(request).await {
                match response.status() {
                    http::StatusCode::OK => try_decode_token(response, token_url).await,
                    http::StatusCode::UNAUTHORIZED => Err(FetchError {
                        message: String::from("Error fetching new token: 401 Unauthorised."),
                        url: token_url,
                    }),
                    http::StatusCode::INTERNAL_SERVER_ERROR => Err(FetchError {
                        message: String::from("Error fetching new token - 500 Server error."),
                        url: token_url,
                    }),
                    http::StatusCode::NOT_FOUND => Err(FetchError {
                        message: String::from("Error fetching new token: 404 Not found."),
                        url: token_url,
                    }),
                    _ => Err(FetchError {
                        message: format!(
                            "Error fetching new token: {}",
                            response.status().as_str()
                        ),
                        url: token_url,
                    }),
                }
            } else {
                Err(FetchError {
                    message: String::from("Couldn't build token request."),
                    url: token_url,
                })
            }
        } else {
            Err(FetchError {
                message: String::from("Couldn't build a request to send for a token."),
                url: token_url,
            })
        }
    }

    async fn try_decode_token(
        response: reqwest::Response,
        token_url: String,
    ) -> Result<TokenStruct, FetchError> {
        match response.text().await {
            Ok(body) => {
                if let Ok(token) = serde_json::from_str::<TokenStruct>(&body) {
                    Ok(token)
                } else {
                    Err(FetchError {
                        message: String::from("Couldn't decode response into token"),
                        url: token_url,
                    })
                }
            }
            Err(_) => Err(FetchError {
                message: String::from("No response body."),
                url: token_url,
            }),
        }
    }

    /// Check if a request is idempotent.
    ///
    /// A request <i>cannot</i> be idempotent if it is <i>not</i> a GET request, it <i>does have</i>
    /// headers, or its body is <i>not</i> empty.
    pub(in crate::service) fn is_idempotent(opts: &FetchOpts) -> bool {
        !matches!(
            (
                opts.method == HttpRequestMethod::GET,
                &opts.headers.is_empty(),
                &opts.body.is_some()
            ),
            (false, _, _) | (_, false, _) | (_, _, false)
        )
    }
}

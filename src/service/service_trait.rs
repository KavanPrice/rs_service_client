use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use http::header;

use crate::error::{FetchError, ServiceError};
use crate::service::directory::DirectoryInterface;
use crate::service::discovery::DiscoveryInterface;
use crate::service::FetchRequest;
use crate::service::service_trait::fetch_util::do_fetch;
use crate::service::service_trait::request::ServiceOpts;
use crate::service::service_trait::response::{FetchResponse, PingResponse};
use crate::uuids;

pub trait Service {
    fn new_error(service: ServiceType, message: &str, status: &str) -> ServiceError {
        ServiceError {
            service,
            message: String::from(message),
            status: String::from(status),
        }
    }

    fn build_client(opts: &ServiceOpts) -> reqwest::Result<reqwest::Client> {
        reqwest::Client::builder()
            .default_headers(opts.headers.clone())
            .build()
    }

    /// Fetch a resource.
    async fn fetch<'a, 'b>(
        &self,
        fetch_request: &FetchRequest<'a, 'b>,
        directory_interface: &DirectoryInterface,
    ) -> Result<FetchResponse, FetchError> {
        // Set up a HeaderValue from &str "application/json"
        let json_header_val = {
            let maybe_header_val = header::HeaderValue::from_str("application/json");
            if let Ok(header_val) = maybe_header_val {
                header_val
            } else {
                return Err(FetchError {
                    message: String::from("Couldn't create correct header value."),
                    url: fetch_request.directory_url.clone(),
                });
            }
        };

        if fetch_request.directory_url == String::new() || fetch_request.directory_url == *"" {
            return Err(FetchError {
                message: String::from("Directory url is empty"),
                url: fetch_request.directory_url.clone(),
            });
        }

        let mut local_headers = fetch_request.opts.headers.clone();
        local_headers
            .entry(header::ACCEPT)
            .or_insert(json_header_val.clone());

        if fetch_request.opts.body.is_some() {
            local_headers
                .entry(header::CONTENT_TYPE)
                .or_insert(json_header_val);
        }

        // From Fetch.cs
        let mut service_urls: Vec<String> = Vec::new();
        let opts = {
            if let Some(service_uuid) = fetch_request.maybe_target_service_uuid {
                service_urls.append(
                    &mut fetch_request
                        .discovery_interface
                        .get_service_urls(service_uuid, directory_interface)
                        .await
                        .unwrap_or_default()
                        .unwrap(),
                );
                let default_string = &mut String::new();
                let amended_url = service_urls.first_mut().unwrap_or(default_string);
                amended_url.push('/');
                amended_url.push_str(&fetch_request.opts.url);
                ServiceOpts {
                    url: (*amended_url.clone()).parse().unwrap(),
                    method: fetch_request.opts.method.clone(),
                    headers: fetch_request.opts.headers.clone(),
                    query: fetch_request.opts.query.clone(),
                    body: fetch_request.opts.body.clone(),
                }
            } else {
                fetch_request.opts.clone()
            }
        };

        // TODO: Implement piggybacking for idempotent requests like in cs-serviceclient

        do_fetch(
            Arc::clone(&fetch_request.client),
            &opts,
            fetch_request.service_username.clone(),
            fetch_request.service_password.clone(),
            fetch_request.tokens,
        )
        .await
    }

    /// Attempts to ping the stack.
    async fn ping(
        &self,
        client: Arc<reqwest::Client>,
        directory_interface: &DirectoryInterface,
    ) -> Result<Option<PingResponse>, FetchError> {
        let fetch_request = FetchRequest {
            service_username: String::new(),
            service_password: String::new(),
            opts: {
                let mut opts = ServiceOpts::new();
                opts.url = String::from("/ping");
                opts
            },
            client,
            directory_url: String::from(""),
            discovery_interface: &DiscoveryInterface::new(),
            maybe_target_service_uuid: None,
            tokens: &mut Default::default(),
        };

        let response = self.fetch(&fetch_request, directory_interface).await?;

        if response.status != http::StatusCode::OK.as_u16() as i32 {
            Ok(None)
        } else {
            Ok(Some(PingResponse::from(response.content)))
        }
    }
}

#[derive(Debug)]
pub enum ServiceType {
    Directory { uuid: uuid::Uuid },
    ConfigDb { uuid: uuid::Uuid },
    Authentication { uuid: uuid::Uuid },
    CommandEscalation { uuid: uuid::Uuid },
    MQTT { uuid: uuid::Uuid },
    Git { uuid: uuid::Uuid },
    Cluster { uuid: uuid::Uuid },
}

impl ServiceType {
    pub fn new(service_id: uuid::Uuid) -> Option<ServiceType> {
        match service_id {
            uuids::service::DIRECTORY => Some(ServiceType::Directory {
                uuid: uuids::service::DIRECTORY,
            }),
            uuids::service::CONFIG_DB => Some(ServiceType::ConfigDb {
                uuid: uuids::service::CONFIG_DB,
            }),
            uuids::service::AUTHENTICATION => Some(ServiceType::Authentication {
                uuid: uuids::service::AUTHENTICATION,
            }),
            uuids::service::COMMAND_ESCALATION => Some(ServiceType::CommandEscalation {
                uuid: uuids::service::COMMAND_ESCALATION,
            }),
            uuids::service::MQTT => Some(ServiceType::MQTT {
                uuid: uuids::service::MQTT,
            }),
            uuids::service::GIT => Some(ServiceType::Git {
                uuid: uuids::service::GIT,
            }),
            uuids::service::CLUSTERS => Some(ServiceType::Cluster {
                uuid: uuids::service::CLUSTERS,
            }),
            _ => None,
        }
    }
}

impl Display for ServiceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::Directory { uuid: id } => write!(f, "Directory service ({})", id),
            ServiceType::ConfigDb { uuid: id } => write!(f, "ConfigDb service ({})", id),
            ServiceType::Authentication { uuid: id } => {
                write!(f, "Authentication service ({})", id)
            }
            ServiceType::CommandEscalation { uuid: id } => {
                write!(f, "CommandEscalation service ({})", id)
            }
            ServiceType::MQTT { uuid: id } => write!(f, "MQTT service ({})", id),
            ServiceType::Git { uuid: id } => write!(f, "Git service ({})", id),
            ServiceType::Cluster { uuid: id } => write!(f, "Cluster service ({})", id),
        }
    }
}

pub mod request {
    //! Contains request representations and implementations.
    use std::collections::HashMap;

    #[derive(Clone)]
    pub struct ServiceOpts {
        pub url: String,
        pub method: HttpRequestMethod,
        pub headers: reqwest::header::HeaderMap,
        pub query: HashMap<String, String>,
        pub body: Option<String>,
    }

    impl ServiceOpts {
        pub fn new() -> Self {
            ServiceOpts {
                url: String::new(),
                method: HttpRequestMethod::GET,
                headers: reqwest::header::HeaderMap::new(),
                query: HashMap::new(),
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

    pub struct FetchResponse {
        pub status: i32,
        pub content: String,
    }

    impl FetchResponse {
        pub fn from(status: i32, content: String) -> Self {
            FetchResponse { status, content }
        }
    }

    pub struct PingResponse {
        pub version: String,
    }

    impl PingResponse {
        pub fn from(version: String) -> Self {
            PingResponse { version }
        }
    }

    #[derive(Deserialize, Clone)]
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

pub(super) mod fetch_util {
    //! Contains utilities used by fetch().
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::sync::Arc;

    use http::{header, StatusCode};
    use serde_json;

    use crate::error::FetchError;
    use crate::service::service_trait::request::{HttpRequestMethod, ServiceOpts};
    use crate::service::service_trait::response::{FetchResponse, TokenStruct};

    pub(crate) async fn do_fetch(
        client: Arc<reqwest::Client>,
        opts: &ServiceOpts,
        username: String,
        password: String,
        tokens: &HashMap<String, TokenStruct>,
    ) -> Result<FetchResponse, FetchError> {
        let response = try_fetch(
            client.clone(),
            opts,
            username.clone(),
            password.clone(),
            tokens,
        )
        .await;
        if let Ok(resp) = &response {
            if resp.status == StatusCode::UNAUTHORIZED.as_u16() as i32 {
                try_fetch(client, opts, username.clone(), password.clone(), tokens).await
            } else {
                response
            }
        } else {
            response
        }
    }

    async fn try_fetch(
        client: Arc<reqwest::Client>,
        opts: &ServiceOpts,
        username: String,
        password: String,
        tokens: &HashMap<String, TokenStruct>,
    ) -> Result<FetchResponse, FetchError> {
        let token =
            get_service_token(client.clone(), opts.url.clone(), username, password, tokens).await?;

        // Set up a HeaderValue from &str "application/json"
        let json_header_val = {
            let maybe_header_val = header::HeaderValue::from_str("application/json");
            if let Ok(header_val) = maybe_header_val {
                header_val
            } else {
                return Err(FetchError {
                    message: String::from("Couldn't create correct header value."),
                    url: opts.url.clone(),
                });
            }
        };

        let mut local_headers = opts.headers.clone();
        local_headers
            .entry(header::ACCEPT)
            .or_insert(json_header_val.clone());

        if opts.body.is_some() {
            local_headers
                .entry(header::CONTENT_TYPE)
                .or_insert(json_header_val);
        }

        let headers_with_auth =
            add_auth_headers(&mut local_headers, String::from("Bearer"), token.token)?;

        let mut builder = client
            .request(opts.method.to_method(), opts.url.clone())
            .headers(headers_with_auth.clone());

        let request_url_result = reqwest::Url::from_str(&*opts.url);
        if let Ok(mut request_url) = request_url_result {
            let mut query_pairs = request_url.query_pairs_mut();
            for (key, value) in opts.query.clone() {
                query_pairs.append_pair(&key, &value);
            }
            let response = if let Some(body) = opts.body.clone() {
                builder.body(body).send().await
            } else {
                builder.send().await
            };

            if let Ok(resp) = response {
                Ok(FetchResponse::from(
                    resp.status().as_u16() as i32,
                    resp.text().await.unwrap(),
                ))
            } else {
                Err(FetchError {
                    message: String::from("Couldn't make request"),
                    url: opts.url.clone(),
                })
            }
        } else {
            Err(FetchError {
                message: String::from("Couldn't make request."),
                url: opts.url.clone(),
            })
        }
    }

    async fn get_service_token(
        client: Arc<reqwest::Client>,
        service_url: String,
        username: String,
        password: String,
        tokens: &HashMap<String, TokenStruct>,
    ) -> Result<TokenStruct, FetchError> {
        // If we find a local token, return it. Otherwise, we request a new one.
        if let Some(token) = tokens.get(&service_url) {
            Ok(token.clone())
        } else {
            let token_url = format!("{}/token", service_url);
            if let Ok(request) = client
                .post(service_url)
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

    /// Add authorisation headers to the request.
    ///
    /// This attempts to convert the given scheme and credentials to a HeaderValue and adds them
    /// to the given HeaderMap.
    fn add_auth_headers(
        header_map: &mut header::HeaderMap,
        scheme: String,
        credentials: String,
    ) -> Result<&header::HeaderMap, FetchError> {
        let auth_val = {
            let maybe_header_val =
                header::HeaderValue::from_str(&format!("{} {}", scheme, credentials));
            if let Ok(header_val) = maybe_header_val {
                header_val
            } else {
                return Err(FetchError {
                    message: String::from("Couldn't create correct header value from credentials."),
                    url: String::new(),
                });
            }
        };

        header_map.entry(header::AUTHORIZATION).or_insert(auth_val);
        Ok(header_map)
    }

    /// Check if a request is idempotent.
    ///
    /// A request <i>cannot</i> be idempotent if it is <i>not</i> a GET request, it <i>does have</i>
    /// headers, or its body is <i>not</i> empty.
    pub(super) fn is_idempotent(opts: &ServiceOpts) -> bool {
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

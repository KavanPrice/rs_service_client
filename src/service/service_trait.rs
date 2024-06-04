use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use http::header;

use crate::error::{FetchError, ServiceError};
use crate::service::discovery::DiscoveryInterface;
use crate::service::service_trait::fetch_util::{do_fetch, is_idempotent};
use crate::service::service_trait::request::ServiceOpts;
use crate::service::service_trait::response::{FetchResponse, PingResponse};
use crate::service::FetchRequest;
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
        fetch_request: &mut FetchRequest<'a, 'b>,
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
        if let Some(service_uuid) = fetch_request.maybe_target_service_uuid {
            service_urls.append(
                &mut fetch_request
                    .discovery_interface
                    .get_service_urls(service_uuid)
                    .await
                    .unwrap_or_default(),
            );
            let default_string = &mut String::new();
            let amended_url = service_urls.first_mut().unwrap_or(default_string);
            amended_url.push('/');
            amended_url.push_str(&fetch_request.opts.url);
            fetch_request.opts.url.clone_from(amended_url);
        }

        // TODO: Implement piggybacking for idempotent requests like in cs-serviceclient

        do_fetch(
            Arc::clone(&fetch_request.client),
            &fetch_request.opts,
            fetch_request.service_username.clone(),
            fetch_request.service_password.clone(),
            fetch_request.tokens,
            fetch_request.in_flight_tokens,
        )
        .await
    }

    /// Attempts to ping the stack.
    async fn ping(&self, client: Arc<reqwest::Client>) -> Result<Option<PingResponse>, FetchError> {
        let mut fetch_request = FetchRequest {
            service_username: String::new(),
            service_password: String::new(),
            opts: {
                let mut opts = ServiceOpts::new();
                opts.url = String::from("/ping");
                opts
            },
            client,
            directory_url: String::from(""),
            discovery_interface: Arc::new(DiscoveryInterface::new()),
            maybe_target_service_uuid: None,
            in_flight_tokens: &mut Default::default(),
            tokens: &mut Default::default(),
        };

        let response = self.fetch(&mut fetch_request).await?;

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
    #[derive(PartialEq, Eq)]
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
        pub expiry: String,
    }

    impl TokenStruct {
        pub fn from(token: String, expiry: String) -> Self {
            TokenStruct { token, expiry }
        }
    }
}

pub(super) mod fetch_util {
    //! Contains utilities used by fetch().
    use std::collections::HashMap;
    use std::future::Future;
    use std::pin::Pin;
    use std::str::FromStr;
    use std::sync::Arc;

    use http::{header, StatusCode};
    use serde_json;

    use crate::error::FetchError;
    use crate::service::service_trait::request::{HttpRequestMethod, ServiceOpts};
    use crate::service::service_trait::response::{FetchResponse, TokenStruct};

    pub(super) async fn do_fetch(
        client: Arc<reqwest::Client>,
        opts: &ServiceOpts,
        username: String,
        password: String,
        tokens: &mut HashMap<String, TokenStruct>,
        in_flight_tokens: &mut HashMap<
            String,
            Pin<Box<dyn Future<Output = Result<TokenStruct, FetchError>> + Send>>,
        >,
    ) -> Result<FetchResponse, FetchError> {
        let response = try_fetch(
            client.clone(),
            opts,
            username.clone(),
            password.clone(),
            tokens,
            in_flight_tokens,
        )
        .await;
        if let Ok(resp) = &response {
            if resp.status == StatusCode::UNAUTHORIZED.as_u16() as i32 {
                try_fetch(
                    client,
                    opts,
                    username.clone(),
                    password.clone(),
                    tokens,
                    in_flight_tokens,
                )
                .await
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
        tokens: &mut HashMap<String, TokenStruct>,
        in_flight_tokens: &mut HashMap<
            String,
            Pin<Box<dyn Future<Output = Result<TokenStruct, FetchError>> + Send>>,
        >,
    ) -> Result<FetchResponse, FetchError> {
        let empty_token = TokenStruct {
            token: String::new(),
            expiry: String::new(),
        };

        let token = service_token(
            client.clone(),
            opts.url.clone(),
            username,
            password,
            Some(&empty_token),
            tokens,
            in_flight_tokens,
        )
        .await;

        if let Ok(t) = token {
            if t.token == String::new() || t.token == String::from("") {
                return Ok(FetchResponse::from(401, String::from("")));
            } else {
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
                    add_auth_headers(&mut local_headers, String::from("Bearer"), t.token)?;

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
                    //let response = builder.send().await;

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
        } else {
            Err(FetchError {
                message: String::from(""),
                url: String::from(""),
            })
        }
    }

    async fn service_token(
        client: Arc<reqwest::Client>,
        service_url: String,
        username: String,
        password: String,
        bad_token: Option<&TokenStruct>,
        tokens: &mut HashMap<String, TokenStruct>,
        in_flight_tokens: &mut HashMap<
            String,
            Pin<Box<dyn Future<Output = Result<TokenStruct, FetchError>> + Send>>,
        >,
    ) -> Result<TokenStruct, FetchError> {
        let maybe_local_token = tokens.get(&service_url);

        // If there's no token in the local token HashMap
        if maybe_local_token.is_none() {
            // If there's a possible in-flight token
            return if let Some(in_flight_token_result) = in_flight_tokens.get_mut(&service_url) {
                // Return the possible token. The caller must unwrap the result
                in_flight_token_result.await
            } else {
                // If there's no in-flight token, request a token with fetch_token
                let token_request = fetch_token(client, service_url.clone(), username, password);

                // Add the Future to the in_flight_tokens HashMap
                in_flight_tokens.insert(service_url.clone(), Box::pin(token_request));

                // Await response
                let response_result = in_flight_tokens.get_mut(&service_url).unwrap().await;

                // If we have a token
                if let Ok(token) = response_result {
                    // Add it to the tokens HashMap and return it
                    tokens.insert(service_url.clone(), token.clone());
                    Ok(token)
                } else {
                    // Otherwise return the error
                    Err(FetchError {
                        message: String::from("Error fetching new token."),
                        url: service_url.clone(),
                    })
                }
            };
        } else {
            // If there is a local token, check against the bad token if it exists
            let token = maybe_local_token.unwrap();
            let is_bad = {
                if let Some(bad) = bad_token {
                    bad.token == token.token
                } else {
                    false
                }
            };

            // If the local token is ok, return it
            if (token.token != String::new() && token.token != String::from("")) && !is_bad {
                Ok(token.clone())
            } else {
                // Otherwise request a new one
                let token_request = fetch_token(client, service_url.clone(), username, password);

                // Add the Future to the in_flight_tokens HashMap
                in_flight_tokens.insert(service_url.clone(), Box::pin(token_request));

                // Await response
                let response_result = in_flight_tokens.get_mut(&service_url).unwrap().await;

                // If we have a token
                if let Ok(token) = response_result {
                    // Add it to the tokens HashMap and return it
                    tokens.insert(service_url.clone(), token.clone());
                    Ok(token)
                } else {
                    // Otherwise return the error
                    Err(FetchError {
                        message: String::from("Error fetching new token."),
                        url: service_url.clone(),
                    })
                }
            }
        }
    }

    async fn fetch_token(
        client: Arc<reqwest::Client>,
        service_url: String,
        username: String,
        password: String,
    ) -> Result<TokenStruct, FetchError> {
        let token_url = format!("{}/{}", service_url, "token");
        let response = gss_fetch(client, token_url.clone(), username, password).await?;

        if response.status != http::StatusCode::OK.as_u16() as i32 {
            Err(FetchError {
                message: String::from("Token fetch failed."),
                url: token_url,
            })
        } else {
            let token_result: Result<TokenStruct, serde_json::Error> =
                serde_json::from_str(&response.content);
            if let Ok(token) = token_result {
                Ok(token)
            } else {
                Err(FetchError {
                    message: String::from("Unable to deserialise response into a token."),
                    url: token_url,
                })
            }
        }
    }

    async fn gss_fetch(
        client: Arc<reqwest::Client>,
        token_url: String,
        username: String,
        password: String,
    ) -> Result<FetchResponse, FetchError> {
        let auth_string = format!("{}:{}", username, password);

        let mut empty_headers = header::HeaderMap::new();
        let headers = add_auth_headers(&mut empty_headers, String::from("Basic"), auth_string)?;

        let request = client.post(&token_url).headers(headers.clone());
        let response = request.send().await;

        match response {
            Ok(res) => {
                if res.status() == http::StatusCode::UNAUTHORIZED {
                    Err(FetchError {
                        message: String::from("Unable to authenticate with Basic auth."),
                        url: token_url,
                    })
                } else {
                    let status_code_i32 = res.status().as_u16() as i32;
                    let body_result = res.text().await;
                    if let Ok(body) = body_result {
                        Ok(FetchResponse::from(status_code_i32, body))
                    } else {
                        Ok(FetchResponse::from(
                            status_code_i32,
                            String::from("Unable to decode body."),
                        ))
                    }
                }
            }
            Err(e) => Err(FetchError {
                message: e.to_string(),
                url: token_url.clone(),
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

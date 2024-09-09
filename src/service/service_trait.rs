use std::fmt::{Debug, Display, Formatter};

use crate::uuids;

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum ServiceType {
    Directory,
    ConfigDb,
    Authentication,
    MQTT,
}

impl ServiceType {
    pub fn to_uuid(&self) -> uuid::Uuid {
        match &self {
            ServiceType::Directory => uuids::service::DIRECTORY,
            ServiceType::ConfigDb => uuids::service::CONFIG_DB,
            ServiceType::Authentication => uuids::service::AUTHENTICATION,
            ServiceType::MQTT => uuids::service::MQTT,
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
            },
            self.to_uuid()
        )
    }
}

pub mod request {
    //! Contains request representations and implementations.
    use std::collections::HashMap;

    use crate::service::service_trait::ServiceType;

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

pub(super) mod fetch_util {
    //! Contains utilities used by fetch().
    use std::sync::Arc;

    use serde_json;

    use crate::error::FetchError;
    use crate::service::service_trait::request::{FetchOpts, HttpRequestMethod};
    use crate::service::service_trait::response::TokenStruct;

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
    pub(super) fn is_idempotent(opts: &FetchOpts) -> bool {
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

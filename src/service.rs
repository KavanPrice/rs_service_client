//! This module provides the entry point for interacting with the Factory+ services.
//!
//! ServiceClient holds the service interfaces, credentials, and service urls.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use sparkplug_rs::protobuf::Message;
use tokio::sync::Mutex;

use crate::error::FetchError;
use crate::service::auth::AuthInterface;
use crate::service::configdb::ConfigDbInterface;
use crate::service::directory::DirectoryInterface;
use crate::service::mqtt::MQTTInterface;
use crate::service::service_trait::request::{FetchOpts, HttpRequestMethod};
use crate::service::service_trait::response::{FetchResponse, PingResponse, TokenStruct};
use crate::service::service_trait::ServiceType;

pub mod auth;
pub mod configdb;
pub mod directory;
pub mod discovery;
pub mod mqtt;
pub mod service_trait;

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

        let directory_interface = DirectoryInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            String::from(directory_url),
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

        let config_db_interface = ConfigDbInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            String::from(directory_url),
            configdb_urls.unwrap().first().unwrap().clone(),
        );

        let mqtt_interface = MQTTInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            mqtt_urls.unwrap().first().unwrap().clone(),
        );

        let auth_interface = AuthInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            String::from(directory_url),
            auth_urls.unwrap().first().unwrap().clone(),
        );

        ServiceClient {
            tokens: Arc::new(Mutex::new(HashMap::new())),
            http_client: Arc::clone(&client),

            service_creds: ServiceCreds::from(service_username, service_password),
            root_principle: root_principle.map(String::from),
            permission_group: permission_group.map(String::from),

            auth_interface,
            config_db_interface,
            directory_interface,
            mqtt_interface,
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
        };

        let new_token = service_trait::fetch_util::get_new_token(
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
            };
            let new_token = service_trait::fetch_util::get_new_token(
                client,
                service_url.clone(),
                username,
                password,
            )
            .await?;
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

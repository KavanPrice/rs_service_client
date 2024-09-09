//! This module provides an implementation of DirectoryInterface for interacting with the Factory+
//! Directory service.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::error::FetchError;
use crate::service;
use crate::service::directory::service_provider::ServiceProvider;
use crate::service::request::{FetchOpts, HttpRequestMethod};
use crate::service::response::{FetchResponse, TokenStruct};
use crate::service::ServiceType;
use crate::service::utils;

/// The interface for the Factory+ Directory service.
///
/// DirectoryInterface holds a hashmap from service URLS to tokens.
pub struct DirectoryInterface {
    pub service_type: ServiceType,
    service_username: String,
    service_password: String,
    http_client: Arc<reqwest::Client>,
    pub service_url: String,
    tokens: Arc<Mutex<HashMap<ServiceType, TokenStruct>>>,
}

impl DirectoryInterface {
    /// Create a new `DirectoryInterface` from a username, password, HTTP client, and directory url.
    pub fn from(
        service_username: String,
        service_password: String,
        http_client: Arc<reqwest::Client>,
        service_url: String,
    ) -> Self {
        DirectoryInterface {
            service_type: ServiceType::Directory,
            service_username,
            service_password,
            http_client,
            service_url,
            tokens: Default::default(),
        }
    }

    /// Gets a vector of URLs that point to a service.
    pub async fn service_urls(
        &self,
        service: ServiceType,
    ) -> Result<Option<Vec<String>>, FetchError> {
        let fetch_opts = FetchOpts {
            url: format!("{}/v1/service/{}", self.service_url, service.to_uuid()),
            service: ServiceType::Directory,
            method: HttpRequestMethod::GET,
            headers: reqwest::header::HeaderMap::new(),
            query: None,
            body: None,
        };

        let response = self.fetch(fetch_opts).await?;

        match response.status {
            http::status::StatusCode::OK => {
                let service_providers_result: Result<Vec<ServiceProvider>, serde_json::Error> =
                    serde_json::from_str(&response.content);
                match service_providers_result {
                    Ok(service_providers_vec) => Ok(Some(
                        service_providers_vec
                            .iter()
                            .filter_map(|x| {
                                x.url.as_ref().map(|url| {
                                    let mut url = url.clone();
                                    if url.ends_with('/') {
                                        url = url.strip_suffix('/').unwrap().to_string();
                                    }
                                    url
                                })
                            })
                            .collect(),
                    )),
                    Err(_) => Err(FetchError {
                        message: String::from("Couldn't decode service response."),
                        url: self.service_url.clone(),
                    }),
                }
            }
            _ => Ok(None),
        }
    }

    /// Register a service url against a service name in the directory.
    pub async fn register_service_url(
        &mut self,
        service_name: String,
        url: String,
    ) -> Result<FetchResponse, FetchError> {
        let opts = FetchOpts {
            url: format!(
                "{}/v1/service/{}/advertisement",
                self.service_url, service_name
            ),
            service: ServiceType::Directory,
            method: HttpRequestMethod::PUT,
            headers: reqwest::header::HeaderMap::new(),
            query: None,
            body: Some(format!("{{\"url\": \"{}\"}}", url)),
        };

        self.fetch(opts).await
    }
    async fn fetch(&self, fetch_opts: FetchOpts) -> Result<FetchResponse, FetchError> {
        let current_directory_token = self.get_directory_token().await?;

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
        .bearer_auth(current_directory_token.token)
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

    async fn get_directory_token(&self) -> Result<TokenStruct, FetchError> {
        let mut locked_tokens = self.tokens.lock().await;
        // If we find a local token, return it. Otherwise, we request a new one.
        if let Some(token) = locked_tokens.get(&ServiceType::Directory) {
            Ok(token.clone())
        } else {
            let new_token = service::fetch_util::get_new_token(
                Arc::clone(&self.http_client),
                self.service_url.clone(),
                &self.service_username,
                &self.service_password,
            )
            .await?;
            locked_tokens.insert(ServiceType::Directory, new_token.clone());
            Ok(new_token)
        }
    }
}

pub mod service_provider {
    //! Contains structs and implementations for representations of service providers.

    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct ServiceProvider {
        pub device: Option<uuid::Uuid>,
        pub url: Option<String>,
    }

    impl ServiceProvider {
        pub fn from(device: uuid::Uuid, url: String) -> Self {
            ServiceProvider {
                device: Some(device),
                url: Some(url),
            }
        }
    }

    pub struct ServiceProviderList {
        pub list: Vec<ServiceProvider>,
    }

    impl ServiceProviderList {
        pub fn from(list: Vec<ServiceProvider>) -> Self {
            ServiceProviderList { list }
        }
    }
}

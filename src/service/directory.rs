//! This module provides an implementation of DirectoryInterface for interacting with the Factory+
//! Directory service.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use http::header;

use crate::error::FetchError;
use crate::service::directory::service_provider::ServiceProvider;
use crate::service::discovery::DiscoveryInterface;
use crate::service::FetchRequest;
use crate::service::service_trait::{Service, ServiceType};
use crate::service::service_trait::request::{HttpRequestMethod, ServiceOpts};
use crate::service::service_trait::response::{FetchResponse, TokenStruct};

/// The interface for the Factory+ Directory service.
///
/// DirectoryInterface holds a hashmap from service URLS to tokens.
pub struct DirectoryInterface {
    service_type: ServiceType,
    service_username: String,
    service_password: String,
    http_client: Arc<reqwest::Client>,
    directory_url: String,
    tokens: HashMap<String, TokenStruct>,
}

impl DirectoryInterface {
    /// Create a new `DirectoryInterface` from a username, password, HTTP client, and directory url.
    pub fn from(
        service_username: String,
        service_password: String,
        http_client: Arc<reqwest::Client>,
        directory_url: String,
    ) -> Self {
        DirectoryInterface {
            service_type: ServiceType::Directory {
                uuid: crate::uuids::service::DIRECTORY,
            },
            service_username,
            service_password,
            http_client,
            directory_url,
            tokens: HashMap::new(),
        }
    }

    /// Gets a vector of URLs that point to a service.
    pub async fn service_urls(
        &self,
        service_uuid: uuid::Uuid,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<Option<Vec<String>>, FetchError> {
        let request = FetchRequest {
            service_username: self.service_username.clone(),
            service_password: self.service_password.clone(),
            opts: ServiceOpts {
                url: format!("/v1/service/{}", service_uuid),
                method: HttpRequestMethod::GET,
                headers: reqwest::header::HeaderMap::new(),
                query: HashMap::new(),
                body: None,
            },
            client: Arc::clone(&self.http_client),
            directory_url: self.directory_url.clone(),
            discovery_interface: &discovery_interface,
            maybe_target_service_uuid: Some(service_uuid),
            tokens: &self.tokens,
        };

        let response = self.fetch(&request).await?;

        match http::status::StatusCode::from_u16(response.status as u16) {
            Err(_) => Err(FetchError {
                message: String::from("Invalid status code was returned from the service."),
                url: self.directory_url.clone(),
            }),
            Ok(http::status::StatusCode::OK) => {
                let service_providers_result: Result<Vec<ServiceProvider>, serde_json::Error> =
                    serde_json::from_str(&response.content);
                match service_providers_result {
                    Ok(service_providers_vec) => Ok(service_providers_vec
                        .iter()
                        .filter(|&x| x.url.is_some())
                        .map(|x| x.url.clone())
                        .collect()),
                    Err(_) => Err(FetchError {
                        message: String::from("Couldn't decode service response."),
                        url: self.directory_url.clone(),
                    }),
                }
            }
            Ok(_) => Ok(None),
        }
    }

    /// Register a service url against a service name in the directory.
    pub async fn register_service_url(
        &mut self,
        service_name: String,
        url: String,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<FetchResponse, FetchError> {
        let service_username = self.service_username.clone();
        let service_password = self.service_password.clone();
        let client = Arc::clone(&self.http_client);
        let directory_url = self.directory_url.clone();

        let opts = ServiceOpts {
            url: format!("/v1/service/{}/advertisement", service_name),
            method: HttpRequestMethod::PUT,
            headers: reqwest::header::HeaderMap::new(),
            query: HashMap::new(),
            body: Some(format!("{{\"url\": \"{}\"}}", url)),
        };

        let tokens = &mut self.tokens.clone();

        let request = FetchRequest {
            service_username,
            service_password,
            opts,
            client,
            directory_url,
            discovery_interface,
            maybe_target_service_uuid: None,
            tokens,
        };

        self.fetch(&request).await
    }

    async fn fetch<'a, 'b>(
        &self,
        fetch_request: &FetchRequest<'a, 'b>,
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

        // Once we are in the Directory service, we know we don't need to involve the Discovery
        // service so we should just try to fetch a response.

        crate::service::service_trait::fetch_util::do_fetch(
            Arc::clone(&fetch_request.client),
            &fetch_request.opts,
            fetch_request.service_username.clone(),
            fetch_request.service_password.clone(),
            fetch_request.tokens,
        )
        .await
    }
}

impl Service for DirectoryInterface {}

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

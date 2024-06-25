//! This module provides the entry point for interacting with the Factory+ services.
//!
//! ServiceClient holds the service interfaces, credentials, and service urls.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::FetchError;
use crate::service::auth::AuthInterface;
use crate::service::configdb::ConfigDbInterface;
use crate::service::directory::DirectoryInterface;
use crate::service::discovery::DiscoveryInterface;
use crate::service::mqtt::MQTTInterface;
use crate::service::service_trait::request::ServiceOpts;
use crate::service::service_trait::response::TokenStruct;

pub mod auth;
pub mod cmdesc;
pub mod configdb;
pub mod configdb_watcher;
pub mod directory;
pub mod discovery;
pub mod git;
pub mod mqtt;
pub mod service_trait;

/// Complex type to hold tokens in flight.
pub type InFlightTokensMap = HashMap<String, Pin<Box<dyn Future<Output = Result<TokenStruct, FetchError>> + Send>>>;

/// Struct to hold the Factory+ service interfaces and service urls.
pub struct ServiceClient {
    tokens: HashMap<String, TokenStruct>,
    http_client: Arc<reqwest::Client>,

    pub auth_interface: AuthInterface,
    pub config_db_interface: ConfigDbInterface,
    pub directory_interface: DirectoryInterface,
    pub discovery_interface: DiscoveryInterface,
    pub mqtt_interface: MQTTInterface,

    service_creds: ServiceCreds,
    pub root_principle: Option<String>,
    pub permission_group: Option<String>,
    pub auth_url: Option<String>,
    pub config_db_url: Option<String>,
    pub directory_url: String,
    pub mqtt_url: Option<String>,
}

impl ServiceClient {
    /// Create a new `ServiceClient` from the given credentials and urls.
    pub fn from(
        service_username: &str,
        service_password: &str,
        root_principle: Option<&str>,
        permission_group: Option<&str>,
        auth_url: Option<&str>,
        config_db_url: Option<&str>,
        directory_url: &str,
        mqtt_url: Option<&str>,
    ) -> Self {
        let client = Arc::new(reqwest::Client::new());

        let directory_interface = DirectoryInterface::from(
            String::from(service_username),
            String::from(service_password),
            Arc::clone(&client),
            String::from(directory_url),
        );

        let discovery_interface = DiscoveryInterface::from(
            auth_url.map(String::from),
            config_db_url.map(String::from),
            Some(String::from(directory_url)),
            mqtt_url.map(String::from),
        );

        ServiceClient {
            tokens: HashMap::new(),
            http_client: Arc::clone(&client),

            service_creds: ServiceCreds::from(service_username, service_password),
            root_principle: root_principle.map(String::from),
            permission_group: permission_group.map(String::from),
            auth_url: auth_url.map(String::from),
            config_db_url: config_db_url.map(String::from),
            directory_url: String::from(directory_url),
            mqtt_url: mqtt_url.map(String::from),

            auth_interface: AuthInterface::new(),
            config_db_interface: ConfigDbInterface::new(),
            directory_interface,
            discovery_interface,
            mqtt_interface: MQTTInterface::new(),
        }
    }

    /// Build a new `FetchRequest` using `&mut self`. This requires a `ServiceOpts` struct
    /// containing the service options for the request and an optional service UUID.
    ///
    /// This is used to make fetch requests to the Factory+ stack using
    /// `rs_service_client::service::service_trait::Service::fetch()`.
    pub fn new_fetch_request(
        &self,
        opts: ServiceOpts,
        maybe_target_service_uuid: Option<uuid::Uuid>,
    ) -> FetchRequest {
        FetchRequest {
            service_username: self.service_creds.service_username.clone(),
            service_password: self.service_creds.service_password.clone(),
            opts,
            client: Arc::clone(&self.http_client),
            directory_url: self.directory_url.clone(),
            discovery_interface: &self.discovery_interface,
            maybe_target_service_uuid,
            tokens: &self.tokens,
        }
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

pub struct FetchRequest<'a, 'b> {
    service_username: String,
    service_password: String,
    opts: ServiceOpts,
    client: Arc<reqwest::Client>,
    directory_url: String,
    discovery_interface: &'a DiscoveryInterface,
    maybe_target_service_uuid: Option<uuid::Uuid>,
    tokens: &'b HashMap<String, TokenStruct>,
}

//! This module provides the entry point for interacting with the Factory+ services.
//!
//! ServiceClient holds the service interfaces, credentials, and service urls.

use std::sync::Arc;

use crate::service::auth::AuthInterface;
use crate::service::configdb::ConfigDbInterface;
use crate::service::directory::DirectoryInterface;
use crate::service::discovery::DiscoveryInterface;
use crate::service::fetch::FetchInterface;
use crate::service::mqtt::MQTTInterface;

pub mod auth;
pub mod cmdesc;
pub mod configdb;
pub mod configdb_watcher;
pub mod directory;
pub mod discovery;
pub mod fetch;
pub mod git;
pub mod mqtt;
pub mod service_trait;

/// Struct to hold the Factory+ service interfaces and service urls.
pub struct ServiceClient {
    pub auth_interface: AuthInterface,
    pub config_db_interface: ConfigDbInterface,
    pub directory_interface: Arc<DirectoryInterface>,
    pub discovery_interface: DiscoveryInterface,
    pub fetch_interface: FetchInterface,
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
    pub fn new(
        service_username: &str,
        service_password: &str,
        root_principle: Option<&str>,
        permission_group: Option<&str>,
        auth_url: Option<&str>,
        config_db_url: Option<&str>,
        directory_url: &str,
        mqtt_url: Option<&str>,
    ) -> Self {
        let directory_interface = Arc::new(DirectoryInterface::new());

        ServiceClient {
            service_creds: ServiceCreds::from(service_username, service_password),
            root_principle: root_principle.map(String::from),
            permission_group: permission_group.map(String::from),
            auth_url: auth_url.map(String::from),
            config_db_url: config_db_url.map(String::from),
            directory_url: String::from(directory_url),
            mqtt_url: mqtt_url.map(String::from),

            auth_interface: AuthInterface::new(),
            config_db_interface: ConfigDbInterface::new(),
            directory_interface: Arc::clone(&directory_interface),
            discovery_interface: DiscoveryInterface::from(
                auth_url.map(String::from),
                config_db_url.map(String::from),
                Some(String::from(directory_url)),
                mqtt_url.map(String::from),
                Arc::clone(&directory_interface),
            ),
            fetch_interface: FetchInterface::new(),
            mqtt_interface: MQTTInterface::new(),
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

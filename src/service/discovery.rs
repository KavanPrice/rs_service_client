//! This module provides an implementation of DiscoveryInterface for interacting with the Factory+
//! Discovery service.

use std::collections::HashMap;
use std::sync::Arc;

use crate::service::directory::DirectoryInterface;
use crate::service::service_trait::Service;
use crate::uuids;

/// The interface for the Factory+ Discovery service.
///
/// DiscoveryInterface holds a hashmap from service UUIDs to service URLs. These can be queried
/// locally and can use the Directory service if not found locally.
pub struct DiscoveryInterface {
    urls: HashMap<uuid::Uuid, Vec<String>>,
    directory_interface: Arc<DirectoryInterface>,
}

impl DiscoveryInterface {
    ///Create a new empty DiscoveryInterface.
    pub fn new() -> Self {
        DiscoveryInterface {
            urls: HashMap::new(),
            directory_interface: Arc::new(DirectoryInterface::new()),
        }
    }

    /// Create a new DiscoveryInterface from the service URLs:
    /// Authentication, ConfigDB, Directory, MQTT.
    ///
    /// Any of these can be None, in which case there will be no entry in the URL hashmap for the
    /// service. The URL can be found later with the Directory service.
    pub fn from(
        auth_url: Option<String>,
        config_db_url: Option<String>,
        directory_url: Option<String>,
        mqtt_url: Option<String>,
        directory_interface: Arc<DirectoryInterface>,
    ) -> Self {
        let mut urls_map: HashMap<uuid::Uuid, Vec<String>> = HashMap::new();

        // Closure to handle inserting the optional urls into urls_map.
        // This will insert a new vector if the key doesn't exist or push to the already existing
        // vector if it does.
        let insert_maybe_url = |(service_uuid, maybe_service_url): (uuid::Uuid, Option<String>)| {
            if let Some(url) = maybe_service_url {
                urls_map.entry(service_uuid).or_default().push(url);
            }
        };

        vec![
            (uuids::service::AUTHENTICATION, auth_url),
            (uuids::service::CONFIG_DB, config_db_url),
            (uuids::service::DIRECTORY, directory_url),
            (uuids::service::MQTT, mqtt_url),
        ]
        .into_iter()
        .for_each(insert_maybe_url);
        DiscoveryInterface {
            urls: urls_map,
            directory_interface,
        }
    }

    /// Inserts a (uuid, url) pair into the urls map. This overwrites the current vector or urls.
    /// This requires a mutable reference to the DiscoveryInterface.
    ///
    /// If the key was already present in the map, the old value is returned.
    /// Otherwise None is returned.
    pub(crate) fn set_service_url(
        &mut self,
        service_uuid: uuid::Uuid,
        service_url: String,
    ) -> Option<Vec<String>> {
        self.urls.insert(service_uuid, vec![service_url])
    }

    /// Inserts a (uuid, url) pair into the urls map. This adds to the current vector value assigned
    /// to the url key if the key already exists.
    /// This requires a mutable reference to the DiscoveryInterface.
    pub(crate) fn add_service_url(&mut self, service_uuid: uuid::Uuid, service_url: String) {
        self.urls.entry(service_uuid).or_default().push(service_url);
    }

    /// Gets all known URLS that point to a service with the given UUID.
    /// The preconfigured URLS are queried first. If the service is not found, the Directory service
    /// is queried.
    pub async fn get_service_urls(&self, service_uuid: uuid::Uuid) -> Option<Vec<String>> {
        if let Some(url) = self.urls.get(&service_uuid).cloned() {
            Some(url)
        } else {
            self.find_service_urls(service_uuid).await
        }
    }

    /// Use the given Directory service to find the urls for the given service.
    pub async fn find_service_urls(&self, service_uuid: uuid::Uuid) -> Option<Vec<String>> {
        self.directory_interface.service_urls(service_uuid).await
    }
}

impl Service for DiscoveryInterface {}

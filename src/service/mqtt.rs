use std::sync::Arc;

use crate::service::service_trait::{Service, ServiceType};
use crate::uuids;

pub struct MQTTInterface {
    service_type: ServiceType,
    service_username: String,
    service_password: String,
    http_client: Arc<reqwest::Client>,
}

impl MQTTInterface {
    pub fn from(
        service_username: String,
        service_password: String,
        http_client: Arc<reqwest::Client>,
    ) -> Self {
        MQTTInterface {
            service_type: ServiceType::MQTT {
                uuid: uuids::service::MQTT,
            },
            service_username,
            service_password,
            http_client,
        }
    }

    pub fn get_mqtt_client(host_url: Option<String>) -> paho_mqtt::AsyncClient {
        todo!()
    }

    pub fn basic_async_client(
        url: String,
        username: String,
        password: String,
    ) -> paho_mqtt::AsyncClient {
        todo!()
    }
}

impl Service for MQTTInterface {}

//! This module provides an implementation of MQTTInterface for interacting with the Factory+
//! MQTT service.

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use paho_mqtt::ReasonCode;
use sparkplug_rs;
use sparkplug_rs::protobuf::Message as ProtobufMessage;
use tokio::sync::Mutex;

use crate::error::MqttError;
use crate::service::mqtt::protocol::MqttProtocol;
use crate::service::response::TokenStruct;
use crate::service::ServiceType;

/// The interface for the Factory+ MQTT service.
pub struct MQTTInterface {
    service_type: ServiceType,
    service_username: String,
    service_password: String,
    http_client: Arc<reqwest::Client>,
    pub service_url: String,
    tokens: Arc<Mutex<HashMap<ServiceType, TokenStruct>>>,
}

impl MQTTInterface {
    pub fn from(
        service_username: String,
        service_password: String,
        http_client: Arc<reqwest::Client>,
        service_url: String,
        tokens: Arc<Mutex<HashMap<ServiceType, TokenStruct>>>,
    ) -> Self {
        MQTTInterface {
            service_type: ServiceType::MQTT,
            service_username,
            service_password,
            http_client,
            service_url,
            tokens,
        }
    }

    /// Attempt to obtain a paho_mqtt::AsyncClient connected to the host at the uri specified by the
    /// passed components. If this is successful, the client will be returned along with the
    /// receiving half of mpsc::channel for receiving the deserialised Sparkplug payloads. These are
    /// deserialised as sparkplug_rs::Payload structs by the client message callback.
    pub async fn get_mqtt_client(
        &self,
        protocol: MqttProtocol,
        port: u16,
        client_id: &str,
    ) -> Result<
        (
            paho_mqtt::AsyncClient,
            mpsc::Receiver<sparkplug_rs::Payload>,
        ),
        MqttError,
    > {
        match self
            .basic_async_client(
                format!("{}:{}", &self.service_url, port),
                client_id,
                self.service_username.clone(),
                self.service_password.clone(),
            )
            .await
        {
            Ok(client_receiver) => Ok(client_receiver),
            Err(paho_mqtt::Error::ReasonCode(ReasonCode::UnspecifiedError)) => Err(MqttError {
                message: String::from("No response from the MQTT service."),
            }),
            Err(resp) => Err(MqttError {
                message: resp.to_string(),
            }),
        }
    }

    async fn basic_async_client(
        &self,
        uri: String,
        client_id: &str,
        username: String,
        password: String,
    ) -> Result<
        (
            paho_mqtt::AsyncClient,
            mpsc::Receiver<sparkplug_rs::Payload>,
        ),
        paho_mqtt::Error,
    > {
        let client = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(uri)
            .client_id(client_id)
            .create_client()?;

        let ssl_options = paho_mqtt::SslOptionsBuilder::new()
            .enable_server_cert_auth(false)
            .finalize();

        let connect_options = paho_mqtt::ConnectOptionsBuilder::new()
            .user_name(username)
            .password(password)
            .clean_start(true)
            .clean_session(true)
            .keep_alive_interval(Duration::from_secs(20))
            .ssl_options(ssl_options)
            .finalize();

        let (sender, receiver) = mpsc::channel::<sparkplug_rs::Payload>();

        client.set_message_callback(move |_client, maybe_message: Option<paho_mqtt::Message>| {
            if let Some(message) = maybe_message {
                match sparkplug_rs::Payload::parse_from_bytes(message.payload()) {
                    Ok(payload) => {
                        if let Err(returned_payload) = sender.send(payload) {
                            eprintln!(
                                "Failed to send payload through channel: {}",
                                returned_payload
                            )
                        }
                    }
                    Err(e) => eprintln!("Failed to parse payload: {}", e),
                }
            }
        });

        match client.connect(connect_options).await {
            Ok(resp) => {
                if resp.connect_response().is_some() {
                    Ok((client, receiver))
                } else {
                    Err(paho_mqtt::Error::ReasonCode(ReasonCode::UnspecifiedError))
                }
            }
            Err(resp) => Err(resp),
        }
    }
}

pub mod protocol {
    //! Contains MqttProtocol and its implementations for describing the protocol to use with the
    //! MQTT service.

    use std::str::FromStr;

    use crate::error::MqttError;

    pub enum MqttProtocol {
        TCP,
        SSL,
        TLS,
    }

    impl MqttProtocol {
        pub fn to_str(&self) -> &str {
            match &self {
                MqttProtocol::TCP => "tcp",
                MqttProtocol::SSL => "ssl",
                MqttProtocol::TLS => "mqtts",
            }
        }
    }

    impl FromStr for MqttProtocol {
        type Err = MqttError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "tcp" => Ok(MqttProtocol::TCP),
                "TCP" => Ok(MqttProtocol::TCP),
                "ssl" => Ok(MqttProtocol::SSL),
                "SSL" => Ok(MqttProtocol::SSL),
                "mqtts" => Ok(MqttProtocol::TLS),
                "MQTTS" => Ok(MqttProtocol::TLS),
                _ => Err(MqttError {
                    message: String::from("Couldn't determine protocol."),
                }),
            }
        }
    }
}

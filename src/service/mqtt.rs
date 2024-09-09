//! This module provides an implementation of MQTTInterface for interacting with the Factory+
//! MQTT service.

use std::sync::Arc;
use std::sync::mpsc;

use paho_mqtt::ReasonCode;
use sparkplug_rs;
use sparkplug_rs::protobuf::Message as ProtobufMessage;

use crate::error::MqttError;
use crate::service::mqtt::protocol::MqttProtocol;
use crate::service::ServiceType;

/// The interface for the Factory+ MQTT service.
pub struct MQTTInterface {
    service_type: ServiceType,
    service_username: String,
    service_password: String,
    http_client: Arc<reqwest::Client>,
    pub service_url: String,
}

impl MQTTInterface {
    pub fn from(
        service_username: String,
        service_password: String,
        http_client: Arc<reqwest::Client>,
        service_url: String,
    ) -> Self {
        MQTTInterface {
            service_type: ServiceType::MQTT,
            service_username,
            service_password,
            http_client,
            service_url,
        }
    }

    /// Attempt to obtain a paho_mqtt::AsyncClient connected to the host at the uri specified by the
    /// passed components. If this is successful, the client will be returned along with the
    /// receiving half of mpsc::channel for receiving the deserialised Sparkplug payloads. These are
    /// deserialised as sparkplug_rs::Payload structs by the client message callback.
    ///
    /// If a None host address is given, the Discovery and Directory services will be used to obtain
    /// an address. If one is not found, an error will be returned.
    pub async fn get_mqtt_client(
        &self,
        protocol: MqttProtocol,
        port: String,
        client_id: String,
    ) -> Result<
        (
            paho_mqtt::AsyncClient,
            mpsc::Receiver<sparkplug_rs::Payload>,
        ),
        MqttError,
    > {
        let full_uri = format!("{}://{}:{}", protocol.to_str(), self.service_url, port);

        match self
            .basic_async_client(
                full_uri,
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
        client_id: String,
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

        let connect_options = paho_mqtt::ConnectOptionsBuilder::new_v5()
            .user_name(username)
            .password(password)
            .clean_start(true)
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
    pub enum MqttProtocol {
        TCP,
        SSL,
    }

    impl MqttProtocol {
        pub fn to_str(&self) -> &str {
            match &self {
                MqttProtocol::TCP => "tcp",
                MqttProtocol::SSL => "ssl",
            }
        }

        pub fn from_str(str: &str) -> Option<Self> {
            match str {
                "tcp" => Some(MqttProtocol::TCP),
                "TCP" => Some(MqttProtocol::TCP),
                "ssl" => Some(MqttProtocol::SSL),
                "SSL" => Some(MqttProtocol::SSL),
                _ => None,
            }
        }
    }
}

//! Contains error types and implementations.
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::service::ServiceType;

#[derive(Debug)]
pub struct ServiceError {
    pub service: ServiceType,
    pub message: String,
    pub status: String,
}

impl Display for ServiceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Service error in service {}. Message: {} ({}).",
            self.service, self.message, self.status
        )
    }
}

impl Error for ServiceError {}

#[derive(Debug, Clone)]
pub struct FetchError {
    pub message: String,
    pub url: String,
}

impl Display for FetchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error fetching from {}. Message: {}.",
            self.url, self.message
        )
    }
}

impl Error for FetchError {}

#[derive(Debug)]
pub struct MqttError {
    pub message: String,
}

impl Display for MqttError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MQTT error: {}.", self.message,)
    }
}

impl From<FetchError> for MqttError {
    fn from(fetch_error: FetchError) -> Self {
        MqttError {
            message: fetch_error.message,
        }
    }
}

impl Error for MqttError {}

//! Contains error types and implementations.
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::service::service_trait::ServiceType;

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

#[derive(Debug)]
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

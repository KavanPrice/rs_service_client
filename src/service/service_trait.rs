use crate::uuids;
use http;
use reqwest;
use reqwest::Client;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

#[derive(Debug)]
pub struct ServiceError {
    service: ServiceType,
    message: String,
    status: String,
}

impl Display for ServiceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Service error in service {}. Message: {} ({})",
            self.service, self.message, self.status
        )
    }
}

impl Error for ServiceError {}

pub trait Service {
    fn new_error(service: ServiceType, message: &str, status: &str) -> ServiceError {
        ServiceError {
            service,
            message: String::from(message),
            status: String::from(status),
        }
    }

    fn build_client(opts: &ServiceOpts) -> reqwest::Result<Client> {
        Client::builder()
            .default_headers(opts.headers.clone())
            .build()
    }

    async fn fetch(
        opts: Arc<ServiceOpts>,
        client: Client,
    ) -> reqwest::Result<(http::status::StatusCode, Option<String>, Option<String>)> {
        // Check we're expecting application/json type returned
        assert_eq!(
            opts.headers.get("accept"),
            Some(&reqwest::header::HeaderValue::from_bytes("application/json".as_ref()).unwrap())
        );
        if let Some(_) = &opts.body {
            assert_eq!(
                opts.headers.get("content-type"),
                Some(
                    &reqwest::header::HeaderValue::from_bytes("application/json".as_ref()).unwrap()
                )
            );
        };

        let req = client.request(opts.method.to_method(), &opts.url).build()?;
        let res = client.execute(req).await?;

        // Must be done in this order for borrowing semantics on reqwest::async_impl::response::Response
        let status = res.status();
        let etag = res
            .headers()
            .get("etag")
            .map(|header_val| header_val.to_str().unwrap())
            .map(String::from);
        let body = res.text().await.ok();

        Ok((status, body, etag))
    }

    async fn ping(
        &self,
        opts: Arc<ServiceOpts>,
        client: Client,
    ) -> (http::status::StatusCode, Option<String>) {
        let mut ping_opts = (*opts).clone();
        ping_opts.url = format!("{}/ping", opts.url);
        let (status, maybe_body, _) = Self::fetch(Arc::new(ping_opts), client)
            .await
            .expect("Couldn't ping service.");

        return (
            status,
            if status.is_success() {
                maybe_body
            } else {
                None
            },
        );
    }
}

#[derive(Debug)]
pub enum ServiceType {
    Directory { uuid: uuid::Uuid },
    ConfigDb { uuid: uuid::Uuid },
    Authentication { uuid: uuid::Uuid },
    CommandEscalation { uuid: uuid::Uuid },
    MQTT { uuid: uuid::Uuid },
    Git { uuid: uuid::Uuid },
    Cluster { uuid: uuid::Uuid },
}

impl ServiceType {
    pub fn new(service_id: uuid::Uuid) -> Option<ServiceType> {
        match service_id {
            uuids::service::DIRECTORY => Some(ServiceType::Directory {
                uuid: uuids::service::DIRECTORY,
            }),
            uuids::service::CONFIG_DB => Some(ServiceType::ConfigDb {
                uuid: uuids::service::CONFIG_DB,
            }),
            uuids::service::AUTHENTICATION => Some(ServiceType::Authentication {
                uuid: uuids::service::AUTHENTICATION,
            }),
            uuids::service::COMMAND_ESCALATION => Some(ServiceType::CommandEscalation {
                uuid: uuids::service::COMMAND_ESCALATION,
            }),
            uuids::service::MQTT => Some(ServiceType::MQTT {
                uuid: uuids::service::MQTT,
            }),
            uuids::service::GIT => Some(ServiceType::Git {
                uuid: uuids::service::GIT,
            }),
            uuids::service::CLUSTERS => Some(ServiceType::Cluster {
                uuid: uuids::service::CLUSTERS,
            }),
            _ => None,
        }
    }
}

impl Display for ServiceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::Directory { uuid: id } => write!(f, "Directory service ({})", id),
            ServiceType::ConfigDb { uuid: id } => write!(f, "ConfigDb service ({})", id),
            ServiceType::Authentication { uuid: id } => {
                write!(f, "Authentication service ({})", id)
            }
            ServiceType::CommandEscalation { uuid: id } => {
                write!(f, "CommandEscalation service ({})", id)
            }
            ServiceType::MQTT { uuid: id } => write!(f, "MQTT service ({})", id),
            ServiceType::Git { uuid: id } => write!(f, "Git service ({})", id),
            ServiceType::Cluster { uuid: id } => write!(f, "Cluster service ({})", id),
        }
    }
}

#[derive(Clone)]
pub struct ServiceOpts {
    url: String,
    method: HttpRequestMethod,
    headers: reqwest::header::HeaderMap,
    body: Option<String>,
}

// Pick only the supported methods
#[derive(Clone)]
pub enum HttpRequestMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
}

impl HttpRequestMethod {
    // Map to a well-defined method from http::Method
    pub fn to_method(&self) -> http::Method {
        match self {
            HttpRequestMethod::Get => http::Method::GET,
            HttpRequestMethod::Post => http::Method::POST,
            HttpRequestMethod::Put => http::Method::PUT,
            HttpRequestMethod::Patch => http::Method::PATCH,
            HttpRequestMethod::Delete => http::Method::DELETE,
            HttpRequestMethod::Head => http::Method::HEAD,
        }
    }
}

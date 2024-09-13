//! This module provides an implementation of CmdEscInterface for interacting with the Factory+
//! Command Escalation service.

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::error::FetchError;
use crate::service;
use crate::service::request::{FetchOpts, HttpRequestMethod};
use crate::service::response::{FetchResponse, TokenStruct};
use crate::service::{utils, ServiceType};
use crate::sparkplug::util::address::Address;

/// The interface for the Factory+ Command Escalation service.
pub struct CmdEscInterface {
    pub service_type: ServiceType,
    service_username: String,
    service_password: String,
    http_client: Arc<reqwest::Client>,
    pub service_url: String,
    tokens: Arc<Mutex<HashMap<ServiceType, TokenStruct>>>,
}

impl CmdEscInterface {
    /// Create a new `CmdEscInterface` from a username, password, HTTP client, service url, and a
    /// tokens HashMap.
    pub fn from(
        service_username: String,
        service_password: String,
        http_client: Arc<reqwest::Client>,
        service_url: String,
        tokens: Arc<Mutex<HashMap<ServiceType, TokenStruct>>>,
    ) -> Self {
        CmdEscInterface {
            service_type: ServiceType::CommandEscalation,
            service_username,
            service_password,
            http_client,
            service_url,
            tokens,
        }
    }

    pub async fn request_cmd(
        &self,
        address: Address,
        name: &str,
        r#type: &str,
        value: CmdValue,
    ) -> Result<FetchResponse, FetchError> {
        let fetch_opts = FetchOpts {
            url: format!("{}/v1/address/{}", self.service_url, address),
            service: ServiceType::CommandEscalation,
            method: HttpRequestMethod::POST,
            headers: Default::default(),
            query: None,
            body: Some(format!(
                r#"{{"name":"{}","type":"{}","value":{}}}"#,
                name, r#type, value
            )),
        };

        self.fetch(fetch_opts).await
    }

    pub async fn rebirth(&self, address: Address) -> Result<FetchResponse, FetchError> {
        let ctrl_string = if address.is_device() {
            "Device Control"
        } else {
            "Node Control"
        };

        self.request_cmd(
            address,
            &format!("{}/Rebirth", ctrl_string),
            "Boolean",
            CmdValue::Bool(true),
        )
        .await
    }

    async fn fetch(&self, fetch_opts: FetchOpts) -> Result<FetchResponse, FetchError> {
        let current_cmdesc_token = self.get_cmdesc_token().await?;

        let headers =
            utils::check_correct_headers(&fetch_opts.headers, &fetch_opts.body, &fetch_opts.url)?;

        if let Ok(request) = match (fetch_opts.query, fetch_opts.body) {
            (None, None) => self
                .http_client
                .request(fetch_opts.method.to_method(), fetch_opts.url.clone())
                .headers(headers),
            (Some(query), None) => self
                .http_client
                .request(fetch_opts.method.to_method(), fetch_opts.url.clone())
                .headers(headers)
                .query(&query),
            (None, Some(body)) => self
                .http_client
                .request(fetch_opts.method.to_method(), fetch_opts.url.clone())
                .headers(headers)
                .body(body),
            (Some(query), Some(body)) => self
                .http_client
                .request(fetch_opts.method.to_method(), fetch_opts.url.clone())
                .headers(headers)
                .query(&query)
                .body(body),
        }
        .bearer_auth(current_cmdesc_token.token)
        .build()
        {
            match self.http_client.execute(request).await {
                Ok(response) => {
                    let response_status = response.status();

                    if let Ok(response_body) = response.text().await {
                        Ok(FetchResponse {
                            status: response_status,
                            content: response_body,
                        })
                    } else {
                        Err(FetchError {
                            message: String::from("Couldn't decode response body."),
                            url: fetch_opts.url,
                        })
                    }
                }
                _ => Err(FetchError {
                    message: String::from("Couldn't make request."),
                    url: fetch_opts.url,
                }),
            }
        } else {
            Err(FetchError {
                message: String::from("Couldn't build a request to fetch."),
                url: fetch_opts.url,
            })
        }
    }

    async fn get_cmdesc_token(&self) -> Result<TokenStruct, FetchError> {
        let mut locked_tokens = self.tokens.lock().await;
        // If we find a local token, return it. Otherwise, we request a new one.
        if let Some(token) = locked_tokens.get(&ServiceType::CommandEscalation) {
            Ok(token.clone())
        } else {
            let new_token = service::fetch_util::get_new_token(
                Arc::clone(&self.http_client),
                self.service_url.clone(),
                &self.service_username,
                &self.service_password,
            )
            .await?;
            locked_tokens.insert(ServiceType::Directory, new_token.clone());
            Ok(new_token)
        }
    }
}

pub enum CmdValue {
    String(String),
    Bool(bool),
}

impl Display for CmdValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdValue::String(value) => write!(f, r#""{}""#, value),
            CmdValue::Bool(value) => write!(f, r#"{}"#, value),
        }
    }
}

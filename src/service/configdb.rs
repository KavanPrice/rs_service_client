use std::collections::HashMap;
use std::sync::Arc;

use http::header;
use tokio::sync::Mutex;

use crate::error::FetchError;
use crate::service;
use crate::service::configdb::configdb_models::{ObjectRegistration, PrincipalConfig};
use crate::service::request::{FetchOpts, HttpRequestMethod};
use crate::service::response::{FetchResponse, TokenStruct};
use crate::service::utils;
use crate::service::ServiceType;

pub struct ConfigDbInterface {
    service_type: ServiceType,
    service_username: String,
    service_password: String,
    http_client: Arc<reqwest::Client>,
    directory_url: String,
    pub service_url: String,
    tokens: Arc<Mutex<HashMap<ServiceType, TokenStruct>>>,
}

impl ConfigDbInterface {
    pub fn from(
        service_username: String,
        service_password: String,
        http_client: Arc<reqwest::Client>,
        directory_url: String,
        service_url: String,
        tokens: Arc<Mutex<HashMap<ServiceType, TokenStruct>>>,
    ) -> Self {
        ConfigDbInterface {
            service_type: ServiceType::ConfigDb,
            service_username,
            service_password,
            http_client: Arc::clone(&http_client),
            directory_url,
            service_url,
            tokens,
        }
    }

    pub async fn get_config(
        &self,
        app: uuid::Uuid,
        obj: uuid::Uuid,
    ) -> Result<Option<PrincipalConfig>, FetchError> {
        let target_url = format!("{}/v1/app/{}/object/{}", self.service_url, app, obj);

        let opts = FetchOpts {
            url: target_url.clone(),
            service: ServiceType::ConfigDb,
            method: HttpRequestMethod::GET,
            headers: Default::default(),
            query: Default::default(),
            body: None,
        };

        let res = self.fetch(opts).await?;

        match res.status {
            http::status::StatusCode::OK => {
                let principal_config_result: Result<PrincipalConfig, serde_json::Error> =
                    serde_json::from_str(&res.content);
                if let Ok(principal_config) = principal_config_result {
                    Ok(Some(principal_config))
                } else {
                    Err(FetchError {
                        message: String::from("Couldn't parse response into a principal config."),
                        url: target_url,
                    })
                }
            }
            http::status::StatusCode::NOT_FOUND => Ok(None),
            _ => Err(FetchError {
                message: String::from("Can't get object."),
                url: target_url,
            }),
        }
    }

    pub async fn put_config(
        &self,
        app: uuid::Uuid,
        obj: uuid::Uuid,
        json_body: String,
    ) -> Result<FetchResponse, FetchError> {
        let opts = FetchOpts {
            url: format!("{}/v1/app/{}/object/{}", self.service_url, app, obj),
            service: ServiceType::ConfigDb,
            method: HttpRequestMethod::PUT,
            headers: Default::default(),
            query: Default::default(),
            body: Some(json_body),
        };

        self.fetch(opts).await
    }

    pub async fn delete_config(
        &self,
        app: uuid::Uuid,
        obj: uuid::Uuid,
    ) -> Result<FetchResponse, FetchError> {
        let opts = FetchOpts {
            url: format!("{}/v1/app/{}/object/{}", self.service_url, app, obj),
            service: ServiceType::ConfigDb,
            method: HttpRequestMethod::DELETE,
            headers: Default::default(),
            query: Default::default(),
            body: None,
        };

        self.fetch(opts).await
    }

    pub async fn patch_config(
        &self,
        app: uuid::Uuid,
        obj: uuid::Uuid,
        patch: String,
    ) -> Result<FetchResponse, FetchError> {
        let target_url = format!("{}/v1/app/{}/object/{}", self.service_url, app, obj);

        let header_val = {
            let maybe_header_val = header::HeaderValue::from_str("application/merge-patch+json");
            if let Ok(header_val) = maybe_header_val {
                header_val
            } else {
                return Err(FetchError {
                    message: String::from("Couldn't create correct header value."),
                    url: target_url.clone(),
                });
            }
        };

        let opts = FetchOpts {
            url: format!("/v1/app/{}/object/{}", app, obj),
            service: ServiceType::ConfigDb,
            method: HttpRequestMethod::PATCH,
            headers: {
                let mut headers: reqwest::header::HeaderMap = Default::default();
                headers.insert(header::CONTENT_TYPE, header_val);
                headers
            },
            query: Default::default(),
            body: Some(patch),
        };

        self.fetch(opts).await
    }

    pub async fn create_object(
        &self,
        class: uuid::Uuid,
        maybe_obj_uuid: Option<uuid::Uuid>,
        is_exclusive: bool,
    ) -> Result<uuid::Uuid, FetchError> {
        let obj_uuid = maybe_obj_uuid.unwrap_or(uuid::Uuid::nil());
        let maybe_req_body: Result<String, serde_json::Error> =
            serde_json::ser::to_string(&ObjectRegistration::from(obj_uuid, class));

        let target_url = format!("{}/v1/object", self.service_url);

        if let Ok(req_body) = maybe_req_body {
            let opts = FetchOpts {
                url: target_url.clone(),
                service: ServiceType::ConfigDb,
                method: HttpRequestMethod::POST,
                headers: Default::default(),
                query: Default::default(),
                body: Some(req_body),
            };

            match self.fetch(opts).await {
                Ok(res) if res.status == 200 && is_exclusive => Err(FetchError {
                    message: format!("Exclusive create of {} failed", obj_uuid),
                    url: target_url.clone(),
                }),
                Ok(res) if res.status == 200 || res.status == 201 => {
                    let object_reg_result: Result<ObjectRegistration, serde_json::Error> =
                        serde_json::from_str(&res.content);
                    if let Ok(object_reg) = object_reg_result {
                        Ok(object_reg.uuid)
                    } else {
                        Err(FetchError {
                            message: String::from(
                                "Couldn't parse response into an object registration.",
                            ),
                            url: target_url.clone(),
                        })
                    }
                }
                Ok(res) if maybe_obj_uuid.is_some() => Err(FetchError {
                    message: format!("{}: Creating {} failed", res.status, obj_uuid),
                    url: target_url.clone(),
                }),
                Ok(res) => Err(FetchError {
                    message: format!("{}: Creating new {} failed", res.status, class),
                    url: target_url.clone(),
                }),
                Err(fetch_error) => Err(fetch_error),
            }
        } else {
            Err(FetchError {
                message: String::from("Couldn't create an object registration."),
                url: String::from("/v1/object"),
            })
        }
    }

    pub async fn delete_object(&self, obj: uuid::Uuid) -> Result<FetchResponse, FetchError> {
        let opts = FetchOpts {
            url: format!("{}/v1/object/{}", self.service_url, obj),
            service: ServiceType::ConfigDb,
            method: HttpRequestMethod::DELETE,
            headers: Default::default(),
            query: Default::default(),
            body: None,
        };

        self.fetch(opts).await
    }

    pub async fn search(
        &self,
        app: uuid::Uuid,
        query: &HashMap<String, String>,
        results: &HashMap<String, String>,
        class: Option<String>,
    ) -> Result<Option<Vec<uuid::Uuid>>, FetchError> {
        let new_query: HashMap<String, String> = query
            .into_iter()
            .chain(results)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let url = format!(
            "{}/v1/app/{}{}/search",
            self.service_url,
            app,
            class.unwrap_or_default()
        );

        let opts = FetchOpts {
            url: url.clone(),
            service: ServiceType::ConfigDb,
            method: HttpRequestMethod::GET,
            headers: Default::default(),
            query: Some(new_query),
            body: None,
        };

        let res = self.fetch(opts).await?;

        match res.status {
            http::status::StatusCode::OK => {
                let uuids_result: Result<Vec<uuid::Uuid>, serde_json::Error> =
                    serde_json::from_str(&res.content);
                if let Ok(uuids) = uuids_result {
                    Ok(Some(uuids))
                } else {
                    Err(FetchError {
                        message: String::from("Failed to parse a UUID from response."),
                        url: url.clone(),
                    })
                }
            }
            http::status::StatusCode::NOT_FOUND => Ok(None),
            _ => Err(FetchError {
                message: String::from("ConfigDB search failed."),
                url: url.clone(),
            }),
        }
    }

    pub async fn resolve(
        &self,
        app: uuid::Uuid,
        query: &HashMap<String, String>,
        results: &HashMap<String, String>,
        class: Option<String>,
    ) -> Result<Option<uuid::Uuid>, FetchError> {
        let maybe_uuids = self.search(app, query, results, class.clone()).await?;

        match maybe_uuids.as_deref() {
            Some([uuid]) => Ok(Some(*uuid)),
            Some([_, _, ..]) => Err(FetchError {
                message: format!("Returned more than once result: {} with {:?}", app, query),
                url: format!("/v1/app/{}{}/search", app, class.unwrap_or_default()),
            }),
            // If the returned option is None or the Vec is somehow empty
            _ => Ok(None),
        }
    }
    async fn fetch(&self, fetch_opts: FetchOpts) -> Result<FetchResponse, FetchError> {
        let current_configdb_token = self.get_configdb_token().await?;

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
        .bearer_auth(current_configdb_token.token)
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

    async fn get_configdb_token(&self) -> Result<TokenStruct, FetchError> {
        let mut locked_tokens = self.tokens.lock().await;
        // If we find a local token, return it. Otherwise, we request a new one.
        if let Some(token) = locked_tokens.get(&ServiceType::Directory) {
            Ok(token.clone())
        } else {
            let new_token = service::fetch_util::get_new_token(
                Arc::clone(&self.http_client),
                self.service_url.clone(),
                &self.service_username,
                &self.service_password,
            )
            .await?;
            locked_tokens.insert(ServiceType::ConfigDb, new_token.clone());
            Ok(new_token)
        }
    }
}

pub mod configdb_models {
    //! Contains structs and implementations for representations of Config elements.

    pub struct PutConfigBody {
        pub name: String,
        pub deleted: Option<bool>,
    }

    impl PutConfigBody {
        pub fn from(name: String, deleted: Option<bool>) -> Self {
            PutConfigBody { name, deleted }
        }
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct ObjectRegistration {
        pub uuid: uuid::Uuid,
        pub class: uuid::Uuid,
    }

    impl ObjectRegistration {
        pub fn from(uuid: uuid::Uuid, class: uuid::Uuid) -> Self {
            ObjectRegistration { uuid, class }
        }
    }

    #[derive(serde::Deserialize)]
    pub struct PrincipalConfig {
        pub group_id: String,
        pub node_id: String,
    }

    impl PrincipalConfig {
        pub fn from(group_id: String, node_id: String) -> Self {
            PrincipalConfig { group_id, node_id }
        }
    }
}

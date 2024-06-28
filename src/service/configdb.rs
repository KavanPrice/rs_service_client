use std::collections::HashMap;
use std::sync::Arc;

use http::header;

use crate::error::FetchError;
use crate::service::configdb::configdb_models::{ObjectRegistration, PrincipalConfig};
use crate::service::directory::DirectoryInterface;
use crate::service::discovery::DiscoveryInterface;
use crate::service::FetchRequest;
use crate::service::service_trait::{Service, ServiceType};
use crate::service::service_trait::request::{HttpRequestMethod, ServiceOpts};
use crate::service::service_trait::response::{FetchResponse, TokenStruct};
use crate::uuids;

pub struct ConfigDbInterface {
    service_type: ServiceType,
    service_username: String,
    service_password: String,
    http_client: Arc<reqwest::Client>,
    directory_url: String,
    tokens: HashMap<String, TokenStruct>,
}

impl ConfigDbInterface {
    pub fn from(
        service_username: String,
        service_password: String,
        http_client: Arc<reqwest::Client>,
        directory_url: String,
    ) -> Self {
        ConfigDbInterface {
            service_type: ServiceType::ConfigDb {
                uuid: uuids::service::CONFIG_DB,
            },
            service_username,
            service_password,
            http_client: Arc::clone(&http_client),
            directory_url,
            tokens: Default::default(),
        }
    }

    pub async fn get_config(
        &self,
        app: uuid::Uuid,
        obj: uuid::Uuid,
        directory_interface: &DirectoryInterface,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<Option<PrincipalConfig>, FetchError> {
        let opts = ServiceOpts {
            url: format!("/v1/app/{}/object/{}", app, obj),
            method: HttpRequestMethod::GET,
            headers: Default::default(),
            query: Default::default(),
            body: None,
        };
        let req = FetchRequest {
            service_username: self.service_username.clone(),
            service_password: self.service_password.clone(),
            opts,
            client: Arc::clone(&self.http_client),
            directory_url: self.directory_url.clone(),
            discovery_interface,
            maybe_target_service_uuid: Some(uuids::service::CONFIG_DB),
            tokens: &self.tokens,
        };
        let res = self.fetch(&req, directory_interface).await?;

        match http::status::StatusCode::from_u16(res.status as u16) {
            Ok(http::status::StatusCode::OK) => {
                let principal_config_result: Result<PrincipalConfig, serde_json::Error> =
                    serde_json::from_str(&res.content);
                if let Ok(principal_config) = principal_config_result {
                    Ok(Some(principal_config))
                } else {
                    Err(FetchError {
                        message: String::from("Couldn't parse response into a principal config."),
                        url: req.opts.url,
                    })
                }
            }
            Ok(http::status::StatusCode::NOT_FOUND) => Ok(None),
            Ok(_) => Err(FetchError {
                message: String::from("Can't get object."),
                url: req.opts.url,
            }),
            Err(_) => Err(FetchError {
                message: String::from("Invalid status code was returned from the service."),
                url: req.opts.url,
            }),
        }
    }

    pub async fn put_config(
        &self,
        app: uuid::Uuid,
        obj: uuid::Uuid,
        json_body: String,
        directory_interface: &DirectoryInterface,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<FetchResponse, FetchError> {
        let opts = ServiceOpts {
            url: format!("/v1/app/{}/object/{}", app, obj),
            method: HttpRequestMethod::PUT,
            headers: Default::default(),
            query: Default::default(),
            body: Some(json_body),
        };
        let req = FetchRequest {
            service_username: self.service_username.clone(),
            service_password: self.service_password.clone(),
            opts,
            client: Arc::clone(&self.http_client),
            directory_url: self.directory_url.clone(),
            discovery_interface,
            maybe_target_service_uuid: Some(uuids::service::CONFIG_DB),
            tokens: &self.tokens,
        };
        self.fetch(&req, directory_interface).await
    }

    pub async fn delete_config(
        &self,
        app: uuid::Uuid,
        obj: uuid::Uuid,
        directory_interface: &DirectoryInterface,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<FetchResponse, FetchError> {
        let opts = ServiceOpts {
            url: format!("/v1/app/{}/object/{}", app, obj),
            method: HttpRequestMethod::DELETE,
            headers: Default::default(),
            query: Default::default(),
            body: None,
        };
        let req = FetchRequest {
            service_username: self.service_username.clone(),
            service_password: self.service_password.clone(),
            opts,
            client: Arc::clone(&self.http_client),
            directory_url: self.directory_url.clone(),
            discovery_interface,
            maybe_target_service_uuid: Some(uuids::service::CONFIG_DB),
            tokens: &self.tokens,
        };
        self.fetch(&req, directory_interface).await
    }

    pub async fn patch_config(
        &self,
        app: uuid::Uuid,
        obj: uuid::Uuid,
        patch: String,
        directory_interface: &DirectoryInterface,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<FetchResponse, FetchError> {
        let header_val = {
            let maybe_header_val = header::HeaderValue::from_str("application/merge-patch+json");
            if let Ok(header_val) = maybe_header_val {
                header_val
            } else {
                return Err(FetchError {
                    message: String::from("Couldn't create correct header value."),
                    url: format!("/v1/app/{}/object/{}", app, obj),
                });
            }
        };

        let opts = ServiceOpts {
            url: format!("/v1/app/{}/object/{}", app, obj),
            method: HttpRequestMethod::PATCH,
            headers: {
                let mut headers: reqwest::header::HeaderMap = Default::default();
                headers.insert(header::CONTENT_TYPE, header_val);
                headers
            },
            query: Default::default(),
            body: Some(patch),
        };
        let req = FetchRequest {
            service_username: self.service_username.clone(),
            service_password: self.service_password.clone(),
            opts,
            client: Arc::clone(&self.http_client),
            directory_url: self.directory_url.clone(),
            discovery_interface,
            maybe_target_service_uuid: Some(uuids::service::CONFIG_DB),
            tokens: &self.tokens,
        };
        self.fetch(&req, directory_interface).await
    }

    pub async fn create_object(
        &self,
        class: uuid::Uuid,
        maybe_obj_uuid: Option<uuid::Uuid>,
        is_exclusive: bool,
        directory_interface: &DirectoryInterface,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<uuid::Uuid, FetchError> {
        let obj_uuid = maybe_obj_uuid.unwrap_or(uuid::Uuid::nil());
        let maybe_req_body: Result<String, serde_json::Error> =
            serde_json::ser::to_string(&ObjectRegistration::from(obj_uuid, class));

        if let Ok(req_body) = maybe_req_body {
            let opts = ServiceOpts {
                url: String::from("/v1/object"),
                method: HttpRequestMethod::POST,
                headers: Default::default(),
                query: Default::default(),
                body: Some(req_body),
            };
            let req = FetchRequest {
                service_username: self.service_username.clone(),
                service_password: self.service_password.clone(),
                opts,
                client: Arc::clone(&self.http_client),
                directory_url: self.directory_url.clone(),
                discovery_interface,
                maybe_target_service_uuid: Some(uuids::service::CONFIG_DB),
                tokens: &self.tokens,
            };
            match self.fetch(&req, directory_interface).await {
                Ok(res) if res.status == 200 && is_exclusive => Err(FetchError {
                    message: format!("Exclusive create of {} failed", obj_uuid),
                    url: req.opts.url,
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
                            url: req.opts.url,
                        })
                    }
                }
                Ok(res) if maybe_obj_uuid.is_some() => Err(FetchError {
                    message: format!("{}: Creating {} failed", res.status, obj_uuid),
                    url: req.opts.url,
                }),
                Ok(res) => Err(FetchError {
                    message: format!("{}: Creating new {} failed", res.status, class),
                    url: req.opts.url,
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

    pub async fn delete_object(
        &self,
        obj: uuid::Uuid,
        directory_interface: &DirectoryInterface,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<FetchResponse, FetchError> {
        let opts = ServiceOpts {
            url: format!("/v1/object/{}", obj),
            method: HttpRequestMethod::DELETE,
            headers: Default::default(),
            query: Default::default(),
            body: None,
        };
        let req = FetchRequest {
            service_username: self.service_username.clone(),
            service_password: self.service_password.clone(),
            opts,
            client: Arc::clone(&self.http_client),
            directory_url: self.directory_url.clone(),
            discovery_interface,
            maybe_target_service_uuid: Some(uuids::service::CONFIG_DB),
            tokens: &self.tokens,
        };

        self.fetch(&req, directory_interface).await
    }

    pub async fn search(
        &self,
        app: uuid::Uuid,
        query: &HashMap<String, String>,
        results: &HashMap<String, String>,
        class: Option<String>,
        directory_interface: &DirectoryInterface,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<Option<Vec<uuid::Uuid>>, FetchError> {
        let new_query: HashMap<String, String> = query
            .into_iter()
            .chain(results)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let url = format!("/v1/app/{}{}/search", app, class.unwrap_or_default());

        let opts = ServiceOpts {
            url: url.clone(),
            method: HttpRequestMethod::GET,
            headers: Default::default(),
            query: new_query,
            body: None,
        };
        let req = FetchRequest {
            service_username: self.service_username.clone(),
            service_password: self.service_password.clone(),
            opts,
            client: Arc::clone(&self.http_client),
            directory_url: self.directory_url.clone(),
            discovery_interface,
            maybe_target_service_uuid: Some(uuids::service::CONFIG_DB),
            tokens: &self.tokens,
        };

        let res = self.fetch(&req, directory_interface).await?;

        match http::status::StatusCode::from_u16(res.status as u16) {
            Ok(http::status::StatusCode::OK) => {
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
            Ok(http::status::StatusCode::NOT_FOUND) => Ok(None),
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
        directory_interface: &DirectoryInterface,
        discovery_interface: &DiscoveryInterface,
    ) -> Result<Option<uuid::Uuid>, FetchError> {
        let maybe_uuids = self
            .search(
                app,
                query,
                results,
                class.clone(),
                directory_interface,
                discovery_interface,
            )
            .await?;

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
}

impl Service for ConfigDbInterface {}

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

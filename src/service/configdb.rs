use std::collections::HashMap;

use crate::error::FetchError;
use crate::service::configdb::configdb_models::PrincipalConfig;
use crate::service::service_trait::{Service, ServiceType};
use crate::service::service_trait::response::FetchResponse;
use crate::uuids;

pub struct ConfigDbInterface {
    service_type: ServiceType,
}

impl ConfigDbInterface {
    pub fn new() -> Self {
        ConfigDbInterface {
            service_type: ServiceType::ConfigDb {
                uuid: uuids::service::CONFIG_DB,
            },
        }
    }

    pub async fn get_config(
        app: uuid::Uuid,
        obj: uuid::Uuid,
    ) -> Result<PrincipalConfig, FetchError> {
        todo!()
    }

    pub async fn put_config(
        app: uuid::Uuid,
        obj: uuid::Uuid,
        json_body: String,
    ) -> Result<FetchResponse, FetchError> {
        todo!()
    }

    pub async fn delete_config(
        app: uuid::Uuid,
        obj: uuid::Uuid,
    ) -> Result<FetchResponse, FetchError> {
        todo!()
    }

    pub async fn patch_config(
        app: uuid::Uuid,
        obj: uuid::Uuid,
        patch: String,
    ) -> Result<FetchResponse, FetchError> {
        todo!()
    }

    pub async fn create_object(
        class: uuid::Uuid,
        obj_uuid_nullable: Option<bool>,
        is_exclusive: bool,
    ) -> Result<FetchResponse, FetchError> {
        todo!()
    }

    pub async fn delete_object(obj: uuid::Uuid) -> Result<FetchResponse, FetchError> {
        todo!()
    }

    pub async fn search(
        app: uuid::Uuid,
        query: &HashMap<String, String>,
        results: &HashMap<String, String>,
        class: Option<String>,
    ) -> Result<Option<uuid::Uuid>, FetchError> {
        todo!()
    }

    pub async fn resolve(
        app: uuid::Uuid,
        query: &HashMap<String, String>,
        results: &HashMap<String, String>,
        class: Option<String>,
    ) -> Result<Option<uuid::Uuid>, FetchError> {
        todo!()
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

    pub struct ObjectRegistration {
        pub uuid: uuid::Uuid,
        pub class: uuid::Uuid,
    }

    impl ObjectRegistration {
        pub fn from(uuid: uuid::Uuid, class: uuid::Uuid) -> Self {
            ObjectRegistration { uuid, class }
        }
    }

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

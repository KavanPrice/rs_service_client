//! This module provides an implementation of AuthInterface for interacting with the Factory+
//! Auth service.

use std::sync::Arc;

use crate::service::ServiceType;

pub struct AuthInterface {
    service_type: ServiceType,
    service_username: String,
    service_password: String,
    http_client: Arc<reqwest::Client>,
    directory_url: String,
    pub service_url: String,
}

impl AuthInterface {
    pub fn from(
        service_username: String,
        service_password: String,
        http_client: Arc<reqwest::Client>,
        directory_url: String,
        service_url: String,
    ) -> Self {
        AuthInterface {
            service_type: ServiceType::Authentication,
            service_username,
            service_password,
            http_client: Arc::clone(&http_client),
            directory_url,
            service_url,
        }
    }

    pub fn check_acl(&self) {
        todo!()
    }

    pub fn fetch_acl(&self) {
        todo!()
    }

    pub fn resolve_principal(&self) {
        todo!()
    }

    pub fn find_principal(&self) {
        todo!()
    }

    pub fn add_principal(&self) {
        todo!()
    }

    pub fn create_principal(&self) {
        todo!()
    }

    pub fn add_ace(&self) {
        todo!()
    }

    pub fn delete_ace(&self) {
        todo!()
    }

    pub fn add_to_group(&self) {
        todo!()
    }

    pub fn remove_from_group(&self) {
        todo!()
    }

    fn resolve_principal_by_address(&self) {
        todo!()
    }

    fn edit_ace(&self) {
        todo!()
    }
}

pub mod auth_models {
    //! Contains structs and implementations for modelling Auth requests and responses.

    use crate::sparkplug::util::Address;

    pub struct PostAceBody {
        pub permission: uuid::Uuid,
        pub target: uuid::Uuid,
        pub ace_action: AceAction,
        pub principal: uuid::Uuid,
        pub kerberos: String,
    }

    pub struct PrincipalMapping {
        pub uuid: uuid::Uuid,
        pub kerberos: String,
        pub sparkplug: Address,
    }

    pub struct FetchAclQuery {
        principal: String,
        permission: String,
        by_uuid: bool,
    }

    pub struct Ace {
        permission: uuid::Uuid,
        target: uuid::Uuid,
        principal: uuid::Uuid,
        kerberos: String,
    }

    pub struct Acl {
        acl_vec: Vec<Acl>,
    }

    pub enum AceAction {
        Add,
        Delete,
    }
}

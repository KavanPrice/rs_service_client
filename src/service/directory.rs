use crate::service::service_trait::Service;

pub struct DirectoryInterface {}

impl DirectoryInterface {
    pub fn new() -> Self {
        todo!()
    }

    pub async fn service_urls(&self, service_uuid: uuid::Uuid) -> Option<Vec<String>> {
        todo!()
    }
}

impl Service for DirectoryInterface {}

mod list;
mod upload;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_actix_web::service_config::ServiceConfig;

// Book metadata for filtering
#[derive(Default, Debug, ToSchema, Deserialize, Serialize)]
struct BookMetadata {
    author: String,
    tags: Vec<String>,
}

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
    |config: &mut ServiceConfig| {
        config.service(upload::upload).service(list::list);
    }
}

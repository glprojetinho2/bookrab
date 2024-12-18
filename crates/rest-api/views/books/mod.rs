pub mod list;
pub mod search;
pub mod upload;
use utoipa_actix_web::service_config::ServiceConfig;

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
    |config: &mut ServiceConfig| {
        config
            .service(upload::upload)
            .service(list::list)
            .service(search::search);
    }
}

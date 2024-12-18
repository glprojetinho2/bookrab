use crate::{
    config::ensure_confy_works,
    errors::{ApiError, Bookrab400},
};
use actix_web::{get, HttpResponse, Responder};
use bookrab_core::{books::RootBookDir, config::BookrabConfig};

/// Lists all books with their metadata.
#[utoipa::path(responses((status = 404, body = Bookrab400)))]
#[get("/list")]
pub async fn list() -> impl Responder {
    _list(ensure_confy_works())
}

pub fn _list(config: BookrabConfig) -> HttpResponse {
    let book_dir = RootBookDir::new(config);
    let listing = match book_dir.list() {
        Ok(v) => v,
        Err(e) => return ApiError(e).into(),
    };
    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&listing).unwrap())
}

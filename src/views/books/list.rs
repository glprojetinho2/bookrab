use super::BookListElement;
use super::RootBookDir;
use crate::{
    config::{get_config, BookrabConfig},
    errors::{CouldntReadChild, CouldntReadFile, InvalidMetadata},
};
use actix_web::{get, HttpResponse, Responder};
use utoipa::{ToResponse, ToSchema};

#[derive(ToSchema, ToResponse)]
#[allow(dead_code)]
enum ListError {
    CouldntReadChild(#[content("application/json")] CouldntReadChild),
    CouldntReadFile(#[content("application/json")] CouldntReadFile),
    InvalidMetadata(#[content("application/json")] InvalidMetadata),
}

/// Lists all books with their metadata.
#[utoipa::path(
    responses (
        (status = 200, description = "Success", body = [BookListElement]),
        (status = 500, content((ListError))),
    )
)]
#[get("/list")]
pub async fn list() -> impl Responder {
    _list(get_config())
}

pub fn _list(config: BookrabConfig) -> HttpResponse {
    let book_dir = RootBookDir::new(config.book_path);
    let listing = match book_dir.list() {
        Ok(v) => v,
        Err(e) => return e.to_res(),
    };
    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&listing).unwrap())
}

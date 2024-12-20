use crate::{
    config::ensure_confy_works,
    database::DB,
    errors::{ApiError, Bookrab400},
};
use actix_web::{get, HttpResponse, Responder};
use bookrab_core::{books::RootBookDir, config::BookrabConfig, database::PgPooledConnection};

/// Lists all books with their metadata.
#[utoipa::path(responses((status = 404, body = Bookrab400)))]
#[get("/list")]
pub async fn list(db: DB) -> impl Responder {
    _list(ensure_confy_works(), db.connection)
}

pub fn _list(config: BookrabConfig, mut connection: PgPooledConnection) -> HttpResponse {
    let book_dir = RootBookDir::new(config, &mut connection);
    let listing = match book_dir.list() {
        Ok(v) => v,
        Err(e) => return ApiError(e).into(),
    };
    HttpResponse::Ok()
        .content_type("application/json")
        .body(serde_json::to_string(&listing).unwrap())
}

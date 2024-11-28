use std::fs;

use super::BookMetadata;
use crate::{
    config::get_config,
    errors::{CouldntReadChild, InvalidMetadata},
};
use actix_web::{get, HttpResponse, Responder};
use log::error;
use serde::Serialize;
use utoipa::{ToResponse, ToSchema};

/// Represents elements returned by the listing
/// route.
#[derive(Debug, Serialize, ToSchema)]
pub struct BookListElement {
    /// Book title
    book: String,
    /// Book metadata for filtering
    metadata: BookMetadata,
}

#[derive(ToSchema, ToResponse)]
#[allow(dead_code)]
enum ListError {
    CouldntReadChild(#[content("application/json")] CouldntReadChild),
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
    _list().await
}

pub async fn _list() -> HttpResponse {
    let config = get_config();
    let books_dir = fs::read_dir(config.book_path.clone()).expect("book path coudnt be read");
    let mut result = vec![];
    for book_dir_res in books_dir {
        let book_dir = match book_dir_res {
            Ok(v) => v,
            Err(e) => {
                return {
                    error!("{:#?}", e);
                    CouldntReadChild::new(
                        config
                            .book_path
                            .to_str()
                            .unwrap_or("path is not even valid unicode"),
                    )
                    .to_res()
                }
            }
        };
        let book_title = book_dir.file_name().to_str().unwrap().to_string();

        // extract metadata
        let metadata_path = book_dir.path().join("metadata.json");
        let metadata_contents;
        if metadata_path.exists() {
            metadata_contents =
                fs::read_to_string(metadata_path).expect("metadata.json couldnt be read");
        } else {
            metadata_contents = serde_json::to_string(&BookMetadata::default())
                .expect("default metadata couldnt be parsed.");
            fs::write(metadata_path, metadata_contents.clone())
                .expect("couldnt supply default metadata for entry lacking a metadata.")
        }
        let metadata_json: BookMetadata = match serde_json::from_str(metadata_contents.as_str()) {
            Ok(v) => v,
            Err(e) => {
                return {
                    error!("{:#?}", e);
                    InvalidMetadata::new(metadata_contents.as_str()).to_res()
                }
            }
        };

        result.push(BookListElement {
            book: book_title,
            metadata: metadata_json,
        });
    }

    HttpResponse::Ok().json(result)
}

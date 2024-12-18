use std::{collections::HashSet, io::Read, path::PathBuf};

use actix_multipart::form::{json::Json, tempfile::TempFile, MultipartForm};
use actix_web::{post, HttpResponse, Responder};
use bookrab_core::{books::RootBookDir, errors::BookrabError};
use utoipa::ToSchema;

use crate::{
    config::ensure_confy_works,
    errors::{ApiError, Bookrab400, Bookrab500},
};

/// Represents a form for book uploading.
/// The books currently have to be .txt files.
#[derive(Debug, MultipartForm, ToSchema)]
struct BookForm {
    /// Book in the .txt format
    #[schema(value_type = String, format = "binary")]
    book: TempFile,
    /// Book tags
    #[schema(value_type = Vec<String>)]
    tags: Json<Vec<String>>,
}

/// Uploads a book to be searched later.
#[utoipa::path(
    request_body(content_type = "multipart/form-data", content = BookForm),
    responses (
        (status = 200, description = "Success (without response body)"),
        (status = 400, body = Bookrab400),
        (status = 500, body = Bookrab500),
    )
)]
#[post("/upload")]
pub async fn upload(MultipartForm(form): MultipartForm<BookForm>) -> impl Responder {
    let config = ensure_confy_works();
    let book_dir = RootBookDir::new(config);

    let mut file = form.book;
    if let Some(v) = file.content_type {
        if v != "text/plain" {
            return ApiError(BookrabError::ShouldBeTextPlain {
                error: (),
                filename: file.file_name.unwrap_or("".to_string()),
            })
            .into();
        }
    };
    let file_name = PathBuf::from(file.file_name.unwrap());
    let mut txt = String::new();
    if let Err(e) = file.file.read_to_string(&mut txt) {
        return ApiError(BookrabError::CouldntReadFile {
            error: (),
            path: file_name,
            err: e,
        })
        .into();
    };
    let mut tags = HashSet::new();
    for tag in form.tags.iter() {
        tags.insert(tag.to_string());
    }
    let title = match file_name.to_str() {
        Some(v) => v,
        None => {
            return ApiError(BookrabError::NotUnicode {
                error: (),
                what: file_name.to_string_lossy().to_string(),
            })
            .into()
        }
    };

    if let Err(e) = book_dir.upload(title, txt.as_str(), tags) {
        return ApiError(e).into();
    };
    HttpResponse::Ok().finish()
}

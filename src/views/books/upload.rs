use crate::{
    books::RootBookDir,
    errors::{BadRequestError, CouldntReadFile, InternalServerErrors, NotUnicode},
};
use std::{collections::HashSet, io::Read, path::PathBuf};

use actix_multipart::form::{json::Json, tempfile::TempFile, MultipartForm};
use actix_web::{post, HttpResponse, Responder};
use log::error;
use utoipa::ToSchema;

use crate::{config::get_config, errors::ShouldBeTextPlain};

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
        (status = 400, content((BadRequestError))),
        (status = 500, content((InternalServerErrors))),
    )
)]
#[post("/upload")]
pub async fn upload(MultipartForm(form): MultipartForm<BookForm>) -> impl Responder {
    let config = get_config();
    let book_dir = RootBookDir::new(config.book_path);

    let mut file = form.book;
    if let Some(v) = file.content_type {
        if v != "text/plain" {
            return ShouldBeTextPlain::new(file.file_name.unwrap_or("".to_string()).as_str())
                .to_res();
        }
    };
    let file_name = PathBuf::from(file.file_name.unwrap());
    let mut txt = String::new();
    if let Err(e) = file.file.read_to_string(&mut txt) {
        error!("{e:#?}");
        return CouldntReadFile::new(&file_name).to_res();
    };
    let mut tags = HashSet::new();
    for tag in form.tags.iter() {
        tags.insert(tag.to_string());
    }
    let title = match file_name.to_str() {
        Some(v) => v,
        None => return NotUnicode::new(file_name.to_string_lossy().to_string()).to_res(),
    };

    if let Err(e) = book_dir.upload(title, txt.as_str(), tags) {
        return e.to_res();
    };
    HttpResponse::Ok().finish()
}

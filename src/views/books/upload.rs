use crate::{errors::CouldntReadFile, views::books::RootBookDir};
use std::{io::Read, path::PathBuf};

use actix_multipart::form::{json::Json, tempfile::TempFile, MultipartForm};
use actix_web::{post, Responder};
use log::error;
use utoipa::{ToResponse, ToSchema};

use crate::{
    config::get_config,
    errors::{CouldntCreateDir, CouldntSaveFile, CouldntWriteFile, ShouldBeTextPlain},
};

use crate::views::books::list::_list;
use crate::views::books::BookListElement;

/// Represents a form for book uploading.
/// The books currently have to be .txt files.
#[derive(Debug, MultipartForm, ToSchema)]
struct BookForm {
    /// Book in the .txt format
    #[schema(value_type = String, format = "binary")]
    book: TempFile,
    /// Book metadata
    #[schema(value_type = Vec<String>)]
    tags: Json<Vec<String>>,
}
/// Represents internal server errors that could be returned from the
/// book uploading endpoint.
#[derive(ToSchema, ToResponse)]
#[allow(dead_code)]
enum UploadError {
    CouldntCreateDir(#[content("application/json")] CouldntCreateDir),
    CouldntWriteMetadata(#[content("application/json")] CouldntWriteFile),
    CouldntSaveFile(#[content("application/json")] CouldntSaveFile),
}

/// Uploads a book to be searched later.
#[utoipa::path(
    request_body(content_type = "multipart/form-data", content = BookForm),
    responses (
        (status = 200, description = "Success", body = [BookListElement]),
        (status = 400, content((ShouldBeTextPlain = "application/json"))),
        (status = 500, content((UploadError)))
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

    _list(get_config()).await
}

use std::{fs, path::PathBuf};

use actix_multipart::form::{json::Json, tempfile::TempFile, MultipartForm};
use actix_web::{post, Responder};
use log::error;
use utoipa::{ToResponse, ToSchema};

use crate::{
    config::get_config,
    errors::{CouldntCreateDir, CouldntSaveFile, CouldntWriteFile, ShouldBeTextPlain},
};

use super::BookMetadata;
use crate::views::books::list::{BookListElement, _list};

/// Represents a form for book uploading.
/// The books currently have to be .txt files.
#[derive(Debug, MultipartForm, ToSchema)]
struct BookForm {
    /// Book in the .txt format
    #[schema(value_type = String, format = "binary")]
    book: TempFile,
    /// Book metadata
    #[schema(value_type = BookMetadata)]
    metadata: Json<BookMetadata>,
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
    let file = form.book;
    if let Some(v) = file.content_type {
        if v != "text/plain" {
            return ShouldBeTextPlain::new(file.file_name.unwrap_or("".to_string()).as_str())
                .to_res();
        }
    };
    let file_name = PathBuf::from(file.file_name.unwrap());
    let mut file_path = config.book_path.clone();
    file_path.push(file_name);

    // create book directory if it doesn't exist
    match fs::create_dir_all(file_path.clone()) {
        Ok(v) => v,
        Err(e) => match e.kind() {
            std::io::ErrorKind::AlreadyExists => (),
            _ => {
                return {
                    error!("{:#?}", e);
                    CouldntCreateDir::new(
                        file_path
                            .to_str()
                            .unwrap_or("path is not even valid unicode"),
                    )
                    .to_res()
                }
            }
        },
    }

    // save text of the book
    file_path.push("txt");
    if let Err(e) = file.file.persist(file_path.clone()) {
        return {
            error!("{:#?}", e);
            CouldntSaveFile::new(
                file_path
                    .to_str()
                    .unwrap_or("path is not even valid unicode"),
            )
            .to_res()
        };
    };

    // save metadata of the book
    file_path.pop();
    file_path.push("metadata.json");
    let metadata = serde_json::to_string(&*form.metadata)
        .expect("couldnt convert metadata do a string (bruh)");
    if let Err(e) = fs::write(file_path.clone(), metadata) {
        return {
            error!("{:#?}", e);
            CouldntWriteFile::new(
                file_path
                    .to_str()
                    .unwrap_or("path is not even valid unicode"),
            )
            .to_res()
        };
    };

    _list().await
}

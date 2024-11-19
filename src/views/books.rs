use crate::{api_error, config::get_config};
use actix_multipart::form::{json::Json as MpJson, tempfile::TempFile, MultipartForm};
use actix_web::get;
use actix_web::post;
use actix_web::HttpResponse;
use actix_web::Responder;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use utoipa::ToSchema;
use utoipa_actix_web::service_config::ServiceConfig;

#[derive(Default, Debug, ToSchema, Deserialize, Serialize)]
struct BookMetadata {
    author: String,
    tags: Vec<String>,
}

/// Represents a form for book uploading.
/// The books currently have to be .txt files.
#[derive(Debug, MultipartForm)]
struct BookForm {
    /// text file
    book: TempFile,
    metadata: MpJson<BookMetadata>,
}

/// Utoipa likes Vec<u8> and doesnt like TempFile. :(
#[derive(ToSchema)]
struct BookFormForUtoipa {
    #[schema(value_type = String, format = "binary")]
    book: actix_multipart::form::bytes::Bytes,
    metadata: BookMetadata,
}

/// Represents elements returned by the listing
/// route.
#[derive(Debug, Serialize, ToSchema)]
struct BookListElement {
    book: String,
    metadata: BookMetadata,
}

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
    |config: &mut ServiceConfig| {
        config.service(upload).service(list);
    }
}

/// Uploads a book
#[utoipa::path(
    request_body(content_type = "multipart/form-data", content = BookFormForUtoipa),
    responses (
        (status = 200, description = "Success", body = [BookListElement])
    )
)]
#[post("/upload")]
pub async fn upload(MultipartForm(form): MultipartForm<BookForm>) -> impl Responder {
    let config = get_config();
    let file = form.book;
    if let Some(v) = file.content_type {
        if v != "text/plain" {
            return api_error!(3, file.file_name.unwrap());
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
            _ => return api_error!(2, file_path.to_str().unwrap(), e),
        },
    }

    // save text of the book
    file_path.push("txt");
    if let Err(e) = file.file.persist(file_path.clone()) {
        return api_error!(1, file_path.to_str().unwrap(), e);
    };

    // save metadata of the book
    file_path.pop();
    file_path.push("metadata.json");
    let metadata = serde_json::to_string(&*form.metadata)
        .expect("couldnt convert metadata do a string (bruh)");
    if let Err(e) = fs::write(file_path.clone(), metadata) {
        return api_error!(4, file_path.to_str().unwrap(), e);
    };

    _list().await
}

/// Lists all books with their metadata.
#[utoipa::path(
    responses (
        (status = 200, description = "Success", body = [BookListElement])
    )
)]
#[get("/list")]
pub async fn list() -> impl Responder {
    _list().await
}

pub async fn _list() -> HttpResponse {
    let config = get_config();
    let books_dir = fs::read_dir(config.book_path).expect("book path coudnt be read");
    let mut result = vec![];
    for book_dir_res in books_dir {
        let book_dir = match book_dir_res {
            Ok(v) => v,
            Err(e) => return api_error!(6, e),
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
            Err(_) => return api_error!(7, metadata_contents),
        };

        result.push(BookListElement {
            book: book_title,
            metadata: metadata_json,
        });
    }

    HttpResponse::Ok().json(result)
}

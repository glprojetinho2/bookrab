use std::{borrow::Cow, io, path::PathBuf};

use actix_web::{
    http::{header::ContentType, StatusCode},
    HttpResponse, HttpResponseBuilder,
};
use bookrab_core::errors::BookrabError;
use grep_searcher::SinkError;
use serde::{de::Error, Serialize};
use utoipa::{
    openapi::{ObjectBuilder, OneOfBuilder, RefOr, Schema},
    PartialSchema, ToSchema,
};

#[derive(Serialize)]
pub struct ApiError(pub BookrabError);

impl Into<HttpResponse> for ApiError {
    fn into(self) -> HttpResponse {
        HttpResponseBuilder::new(self.status())
            .content_type(ContentType::json())
            .body(serde_json::to_string(&self.0).unwrap())
    }
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self.0 {
            BookrabError::CouldntSaveFile { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::CouldntCreateDir { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::CouldntWriteFile { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::MessedUpBookFolder { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::CouldntReadChild { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::InvalidTags { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::CouldntReadFile { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::CouldntReadDir { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::GrepSearchError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::DatabaseError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            BookrabError::InexistentBook { .. } => StatusCode::BAD_REQUEST,
            BookrabError::ShouldBeTextPlain { .. } => StatusCode::BAD_REQUEST,
            BookrabError::NotUnicode { .. } => StatusCode::BAD_REQUEST,
            BookrabError::RegexProblem { .. } => StatusCode::BAD_REQUEST,
        }
    }
    fn examples() -> Vec<Self> {
        vec![
            BookrabError::CouldntSaveFile {
                error: (),
                path: PathBuf::from("path/to/file"),
                err: io::Error::error_message("Cool Rust io error."),
            },
            BookrabError::CouldntSaveFile {
                error: (),
                path: PathBuf::from("path/to/file"),
                err: io::Error::error_message("Cool Rust io error."),
            },
            BookrabError::CouldntCreateDir {
                error: (),
                path: PathBuf::from("path/to/file"),
                err: io::Error::error_message("Cool Rust io error."),
            },
            BookrabError::CouldntWriteFile {
                error: (),
                path: PathBuf::from("path/to/file"),
                err: io::Error::error_message("Cool Rust io error."),
            },
            BookrabError::MessedUpBookFolder {
                error: (),
                path: PathBuf::from("path/to/file"),
            },
            BookrabError::CouldntReadChild {
                error: (),
                parent: PathBuf::from("path/to/file"),
                err: io::Error::error_message("Cool Rust io error."),
            },
            BookrabError::InvalidTags {
                error: (),
                tags: "messed up tags (not valid JSON string list)".into(),
                path: PathBuf::from("path/to/file"),
                err: serde_json::Error::custom("Cool serde error"),
            },
            BookrabError::CouldntReadFile {
                error: (),
                path: PathBuf::from("path/to/file"),
                err: io::Error::error_message("Cool Rust io error."),
            },
            BookrabError::CouldntReadDir {
                error: (),
                path: PathBuf::from("path/to/file"),
                err: io::Error::error_message("Cool Rust io error."),
            },
            BookrabError::GrepSearchError {
                error: (),
                path: PathBuf::from("path/to/file"),
                err: io::Error::error_message("Cool Rust io error."),
            },
            BookrabError::DatabaseError {
                error: (),
                err: diesel::result::Error::NotFound,
            },
            BookrabError::InexistentBook {
                error: (),
                path: PathBuf::from("path/to/file"),
            },
            BookrabError::ShouldBeTextPlain {
                error: (),
                filename: String::from("filename"),
            },
            BookrabError::NotUnicode {
                error: (),
                what: "ugly string ���������".into(),
            },
            BookrabError::RegexProblem {
                error: (),
                err: grep_regex::RegexMatcher::new("(").unwrap_err(),
            },
        ]
        .into_iter()
        .map(ApiError)
        .collect()
    }
    fn examples_with_status(status: StatusCode) -> Vec<Self> {
        Self::examples()
            .into_iter()
            .filter(|x| x.status() == status)
            .collect()
    }
}

/// Used to convert api errors to utoipa schemas.
/// It groups errors based on their status.
fn api_errors_to_schema(status: StatusCode) -> RefOr<Schema> {
    let examples = ApiError::examples_with_status(status);
    let mut one_of = OneOfBuilder::new();
    for example in examples {
        let example_json = serde_json::to_value(example).unwrap();
        let mut utoipa_object = ObjectBuilder::new();

        let (description, actual_object) = example_json.as_object().unwrap().iter().next().unwrap();
        utoipa_object = utoipa_object.examples(vec![actual_object.clone()]);
        for (key, value) in actual_object.as_object().unwrap() {
            utoipa_object = utoipa_object.property(key, value.to_owned());
        }

        utoipa_object = utoipa_object.title(Some(description));
        one_of = one_of.item(utoipa_object.build());
    }
    RefOr::T(Schema::OneOf(one_of.build()))
}

pub struct Bookrab400;
impl ToSchema for Bookrab400 {
    fn name() -> Cow<'static, str> {
        std::borrow::Cow::Borrowed("Bookrab404")
    }
}
impl PartialSchema for Bookrab400 {
    fn schema() -> RefOr<Schema> {
        api_errors_to_schema(StatusCode::BAD_REQUEST)
    }
}

pub struct Bookrab500;
impl ToSchema for Bookrab500 {
    fn name() -> Cow<'static, str> {
        std::borrow::Cow::Borrowed("Bookrab500")
    }
}
impl PartialSchema for Bookrab500 {
    fn schema() -> RefOr<Schema> {
        api_errors_to_schema(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

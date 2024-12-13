use anyhow::anyhow;
use log::error;
use serde::Serialize;
use std::path::PathBuf;
use utoipa::{openapi::content, ToResponse, ToSchema};

use actix_web::{
    body::BoxBody,
    http::{header::ContentType, StatusCode},
    HttpResponse, Responder,
};
use serde_json::json;
use thiserror::Error;
pub const E0001_MSG: &str = "E0001: could not save file permanently.";
pub const E0002_MSG: &str = "E0002: could not create directory.";
pub const E0003_MSG: &str = "E0003: file should have 'text/plain' content type.";
pub const E0004_MSG: &str = "E0004: could not write tags.";
pub const E0005_MSG: &str = "E0005: one of your book folders is messed up.";
pub const E0006_MSG: &str = "E0006: couldnt read child of your book folder.";
pub const E0007_MSG: &str = "E0007: invalid tags.";
pub const E0008_MSG: &str = "E0008: couldnt read file.";
pub const E0009_MSG: &str = "E0009: couldnt read dir.";
pub const E0010_MSG: &str = "E0010: not valid unicode.";
pub const E0011_MSG: &str = "E0011: book doesnt exist.";
pub const E0012_MSG: &str = "E0012: problematic regex pattern.";
pub const E0013_MSG: &str = "E0013: couldn't search file (even though it exists).";
pub const E0014_MSG: &str = "E0014: invalid history entry.";
pub const E0015_MSG: &str = "E0015: database error.";

macro_rules! impl_responder {
    ($struct: ident, $status: expr, $msg: expr) => {
        impl $struct {
            /// Converts value to [`HttpResponse`]
            pub fn to_res(self) -> HttpResponse<BoxBody>
            where
                Self: Serialize + Sized,
            {
                let mut body = serde_json::to_value(&self).unwrap();
                body["error"] = json!($msg);

                HttpResponse::Ok()
                    .status($status)
                    .content_type(ContentType::json())
                    .body(body.to_string())
            }
        }

        #[allow(clippy::from_over_into)]
        impl Into<HttpResponse> for $struct {
            fn into(self) -> HttpResponse {
                self.to_res()
            }
        }
    };
}

/// Responds with [`E0001_MSG`]
/// Server couldn't turn a temporary file into a permanent file.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct CouldntSaveFile {
    #[schema(default = json!(E0001_MSG))]
    pub error: String,
    pub path: String,
}

impl CouldntSaveFile {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            error: E0001_MSG.to_string(),
            path: path
                .to_str()
                .expect("path is not valid unicode")
                .to_string(),
        }
    }
}

impl_responder!(
    CouldntSaveFile,
    StatusCode::INTERNAL_SERVER_ERROR,
    E0001_MSG
);

/// Responds with [`E0002_MSG`]
/// Server couldn't create a folder.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct CouldntCreateDir {
    #[schema(default = json!(E0002_MSG))]
    pub error: String,
    pub path: String,
}

impl CouldntCreateDir {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            error: E0002_MSG.to_string(),
            path: path
                .to_str()
                .expect("path is not valid unicode")
                .to_string(),
        }
    }
}

impl_responder!(
    CouldntCreateDir,
    StatusCode::INTERNAL_SERVER_ERROR,
    E0002_MSG
);

/// Responds with [`E0003_MSG`]
/// You shoud've inputed a text file.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct ShouldBeTextPlain {
    #[schema(default = json!(E0003_MSG))]
    pub error: String,
    pub filename: String,
}

impl ShouldBeTextPlain {
    pub fn new(filename: &str) -> Self {
        Self {
            error: E0003_MSG.to_string(),
            filename: filename.to_string(),
        }
    }
}

impl_responder!(ShouldBeTextPlain, StatusCode::BAD_REQUEST, E0003_MSG);

/// Responds with [`E0004_MSG`]
/// Server couldn't write file.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct CouldntWriteFile {
    #[schema(default = json!(E0004_MSG))]
    pub error: String,
    pub path: String,
}

impl CouldntWriteFile {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            error: E0004_MSG.to_string(),
            path: path
                .to_str()
                .expect("path is not valid unicode")
                .to_string(),
        }
    }
}

impl_responder!(
    CouldntWriteFile,
    StatusCode::INTERNAL_SERVER_ERROR,
    E0004_MSG
);

/// Responds with [`E0005_MSG`]
/// Your book folder is messed up. Check it out.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct MessedUpBookFolder {
    #[schema(default = json!(E0005_MSG))]
    pub error: String,
    pub path: String,
}

impl MessedUpBookFolder {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            error: E0005_MSG.to_string(),
            path: path
                .to_str()
                .expect("path is not valid unicode")
                .to_string(),
        }
    }
}

impl_responder!(
    MessedUpBookFolder,
    StatusCode::INTERNAL_SERVER_ERROR,
    E0005_MSG
);

/// Responds with [`E0006_MSG`]
/// Couldnt read folder inside parent.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct CouldntReadChild {
    #[schema(default = json!(E0006_MSG))]
    pub error: String,
    pub parent: String,
}

impl CouldntReadChild {
    pub fn new(parent: &str) -> Self {
        Self {
            error: E0006_MSG.to_string(),
            parent: parent.to_string(),
        }
    }
}

impl_responder!(
    CouldntReadChild,
    StatusCode::INTERNAL_SERVER_ERROR,
    E0006_MSG
);

/// Responds with [`E0007_MSG`]
/// Invalid tags inside book folder.
#[derive(
    serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug, PartialEq,
)]
pub struct InvalidTags {
    #[schema(default = json!(E0007_MSG))]
    pub error: String,
    pub tags: String,
    pub path: String,
}

impl InvalidTags {
    pub fn new(tags: &str, path: &PathBuf) -> Self {
        Self {
            error: E0007_MSG.to_string(),
            tags: tags.to_string(),
            path: path
                .to_str()
                .expect("path is not valid unicode")
                .to_string(),
        }
    }
}

impl_responder!(InvalidTags, StatusCode::INTERNAL_SERVER_ERROR, E0007_MSG);

/// Responds with [`E0008_MSG`]
/// Couldnt read folder inside parent.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct CouldntReadFile {
    #[schema(default = json!(E0008_MSG))]
    pub error: String,
    pub path: String,
}

impl CouldntReadFile {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            error: E0008_MSG.to_string(),
            path: path.to_str().unwrap_or("path is not unicode").to_string(),
        }
    }
}

impl_responder!(
    CouldntReadFile,
    StatusCode::INTERNAL_SERVER_ERROR,
    E0008_MSG
);

/// Responds with [`E0009_MSG`]
/// Couldnt read folder inside parent.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct CouldntReadDir {
    #[schema(default = json!(E0009_MSG))]
    pub error: String,
    pub path: String,
}

impl CouldntReadDir {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            error: E0009_MSG.to_string(),
            path: path.to_str().unwrap_or("path is not unicode").to_string(),
        }
    }
}

impl_responder!(CouldntReadDir, StatusCode::INTERNAL_SERVER_ERROR, E0009_MSG);

/// Responds with [`E0010_MSG`]
/// Something is not Unicode.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct NotUnicode {
    #[schema(default = json!(E0010_MSG))]
    pub error: String,
    pub what: String,
}

impl NotUnicode {
    pub fn new(what: String) -> Self {
        Self {
            error: E0010_MSG.to_string(),
            what,
        }
    }
}

impl_responder!(NotUnicode, StatusCode::BAD_REQUEST, E0010_MSG);

/// Responds with [`E0011_MSG`]
/// Book doesn't exist
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct InexistentBook {
    #[schema(default = json!(E0011_MSG))]
    pub error: String,
    pub path: String,
}

impl InexistentBook {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            error: E0011_MSG.to_string(),
            path: path.to_str().unwrap_or("path is not unicode").to_string(),
        }
    }
}

impl_responder!(InexistentBook, StatusCode::BAD_REQUEST, E0011_MSG);

/// Responds with [`E0012_MSG`]
/// Check your regex.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct RegexProblem {
    #[schema(default = json!(E0012_MSG))]
    pub error: String,
    pub cause: String,
}

impl RegexProblem {
    pub fn new(regex_error: grep_regex::Error) -> Self {
        Self {
            error: E0012_MSG.to_string(),
            cause: format!("{:?}", regex_error),
        }
    }
}

impl_responder!(RegexProblem, StatusCode::BAD_REQUEST, E0012_MSG);

/// Responds with [`E0013_MSG`]
/// Book doesn't exist
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug)]
pub struct GrepSearchError {
    #[schema(default = json!(E0013_MSG))]
    pub error: String,
    pub path: String,
}

impl GrepSearchError {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            error: E0013_MSG.to_string(),
            path: path.to_str().unwrap_or("path is not unicode").to_string(),
        }
    }
}

impl_responder!(
    GrepSearchError,
    StatusCode::INTERNAL_SERVER_ERROR,
    E0013_MSG
);

/// Responds with [`E0014_MSG`]
/// Invalid history inside book folder.
#[derive(
    serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug, PartialEq,
)]
pub struct InvalidHistory {
    #[schema(default = json!(E0014_MSG))]
    pub error: String,
    pub history: String,
    pub path: String,
}

impl InvalidHistory {
    pub fn new(history: &str, path: &PathBuf) -> Self {
        Self {
            error: E0014_MSG.to_string(),
            history: history.to_string(),
            path: path
                .to_str()
                .expect("path is not valid unicode")
                .to_string(),
        }
    }
}

impl_responder!(InvalidHistory, StatusCode::INTERNAL_SERVER_ERROR, E0014_MSG);

/// Responds with [`E0015_MSG`]
/// Database error.
#[derive(
    serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug, PartialEq,
)]
pub struct DatabaseError {
    #[schema(default = json!(E0015_MSG))]
    pub error: String,
    pub cause: String,
}

impl DatabaseError {
    pub fn new(cause: String) -> Self {
        Self {
            error: E0015_MSG.to_string(),
            cause,
        }
    }
}

impl_responder!(DatabaseError, StatusCode::INTERNAL_SERVER_ERROR, E0015_MSG);

/// Api errors that can be used outside of actix handlers.
/// You should always be using this.
#[derive(Error, Debug)]
pub enum BookrabError {
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    CouldntSaveFile(CouldntSaveFile, anyhow::Error),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    CouldntCreateDir(CouldntCreateDir, anyhow::Error),
    #[error("{}", serde_json::to_string(.0).unwrap())]
    ShouldBeTextPlain(ShouldBeTextPlain),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    CouldntWriteFile(CouldntWriteFile, anyhow::Error),
    #[error("{}", serde_json::to_string(.0).unwrap())]
    MessedUpBookFolder(MessedUpBookFolder),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    CouldntReadChild(CouldntReadChild, anyhow::Error),
    #[error("{}", serde_json::to_string(.0).unwrap())]
    InvalidTags(InvalidTags),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    CouldntReadFile(CouldntReadFile, anyhow::Error),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    CouldntReadDir(CouldntReadDir, anyhow::Error),
    #[error("{}\n", serde_json::to_string(.0).unwrap())]
    NotUnicode(NotUnicode),
    #[error("{}\n", serde_json::to_string(.0).unwrap())]
    InexistentBook(InexistentBook),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    RegexProblem(RegexProblem, anyhow::Error),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    GrepSearchError(GrepSearchError, anyhow::Error),
    #[error("{}", serde_json::to_string(.0).unwrap())]
    InvalidHistory(InvalidHistory),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    DatabaseError(DatabaseError, anyhow::Error),
}

impl BookrabError {
    /// Converts value to [`HttpResponse`]
    pub fn to_res(self) -> HttpResponse<BoxBody> {
        match self {
            Self::CouldntReadDir(err, e) => {
                error!("{e:#?}");
                err.to_res()
            }
            Self::CouldntReadFile(err, e) => {
                error!("{e:#?}");
                err.to_res()
            }
            Self::InvalidTags(err) => err.to_res(),
            Self::CouldntReadChild(err, e) => {
                error!("{e:#?}");
                err.to_res()
            }
            Self::MessedUpBookFolder(err) => err.to_res(),
            Self::ShouldBeTextPlain(err) => err.to_res(),
            Self::CouldntWriteFile(err, e) => {
                error!("{e:#?}");
                err.to_res()
            }
            Self::CouldntCreateDir(err, e) => {
                error!("{e:#?}");
                err.to_res()
            }
            Self::CouldntSaveFile(err, e) => {
                error!("{e:#?}");
                err.to_res()
            }
            Self::NotUnicode(err) => err.to_res(),
            Self::InexistentBook(err) => err.to_res(),
            Self::RegexProblem(err, e) => {
                error!("{e:#?}");
                err.to_res()
            }
            Self::GrepSearchError(err, e) => {
                error!("{e:#?}");
                err.to_res()
            }
            Self::InvalidHistory(err) => err.to_res(),
            Self::DatabaseError(err, e) => {
                error!("{e:#?}");
                err.to_res()
            }
        }
    }
}

impl From<grep_regex::Error> for BookrabError {
    fn from(err: grep_regex::Error) -> Self {
        let bookrab_error = RegexProblem::new(err.clone());
        BookrabError::RegexProblem(bookrab_error, anyhow!(err))
    }
}
impl From<diesel::result::Error> for BookrabError {
    fn from(err: diesel::result::Error) -> Self {
        let bookrab_error = DatabaseError::new(format!("{:#?}", err));
        BookrabError::DatabaseError(bookrab_error, anyhow!(err))
    }
}

impl Into<HttpResponse> for BookrabError {
    fn into(self) -> HttpResponse {
        self.to_res()
    }
}

#[derive(ToSchema, ToResponse)]
#[allow(dead_code)]
pub enum InternalServerErrors {
    CouldntSaveFile(#[content("application/json")] CouldntSaveFile),
    CouldntCreateDir(#[content("application/json")] CouldntCreateDir),
    CouldntWriteFile(#[content("application/json")] CouldntWriteFile),
    MessedUpBookFolder(#[content("application/json")] MessedUpBookFolder),
    CouldntReadChild(#[content("application/json")] CouldntReadChild),
    InvalidTags(#[content("application/json")] InvalidTags),
    CouldntReadFile(#[content("application/json")] CouldntReadFile),
    CouldntReadDir(#[content("application/json")] CouldntReadDir),
    GrepSearchError(#[content("application/json")] GrepSearchError),
    InvalidHistory(#[content("application/json")] InvalidHistory),
    DatabaseError(#[content("application/json")] DatabaseError),
}

#[derive(ToSchema, ToResponse)]
#[allow(dead_code)]
pub enum BadRequestError {
    ShouldBeTextPlain(#[content("application/json")] ShouldBeTextPlain),
    NotUnicode(#[content("application/json")] NotUnicode),
    InexistentBook(#[content("application/json")] InexistentBook),
    RegexProblem(#[content("application/json")] RegexProblem),
}

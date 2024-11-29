use log::error;
use serde::Serialize;
use std::path::PathBuf;

use actix_web::{
    body::BoxBody,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use serde_json::json;
use thiserror::Error;
pub const E0001_MSG: &str = "E0001: could not save file permanently.";
pub const E0002_MSG: &str = "E0002: could not create directory.";
pub const E0003_MSG: &str = "E0003: file should have 'text/plain' content type.";
pub const E0004_MSG: &str = "E0004: could not write metadata.";
pub const E0005_MSG: &str = "E0005: one of your book folders is messed up.";
pub const E0006_MSG: &str = "E0006: couldnt read child of your book folder.";
pub const E0007_MSG: &str = "E0007: invalid metadata.";
pub const E0008_MSG: &str = "E0008: couldnt read file.";
pub const E0009_MSG: &str = "E0009: couldnt read dir.";

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
/// Invalid metadata inside book folder.
#[derive(
    serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::ToResponse, Debug, PartialEq,
)]
pub struct InvalidMetadata {
    #[schema(default = json!(E0007_MSG))]
    pub error: String,
    pub metadata: String,
    pub path: String,
}

impl InvalidMetadata {
    pub fn new(metadata: &str, path: &PathBuf) -> Self {
        Self {
            error: E0007_MSG.to_string(),
            metadata: metadata.to_string(),
            path: path
                .to_str()
                .expect("path is not valid unicode")
                .to_string(),
        }
    }
}

impl_responder!(
    InvalidMetadata,
    StatusCode::INTERNAL_SERVER_ERROR,
    E0007_MSG
);

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
    InvalidMetadata(InvalidMetadata),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    CouldntReadFile(CouldntReadFile, anyhow::Error),
    #[error("{}\ncause: {:#?}", serde_json::to_string(.0).unwrap(), .1)]
    CouldntReadDir(CouldntReadDir, anyhow::Error),
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
            Self::InvalidMetadata(err) => err.to_res(),
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
        }
    }
}

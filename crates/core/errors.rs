use std::{fmt::Debug, path::PathBuf};

use serde::{Serialize, Serializer};

macro_rules! edddd {
    ($name: ident, $msg: expr) => {
        fn $name<S: Serializer>(_: &(), s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str($msg)
        }
    };
}
edddd!(e0001, "E0001: could not save file permanently.");
edddd!(e0002, "E0002: could not create directory.");
edddd!(e0003, "E0003: file should have 'text/plain' content type.");
edddd!(e0004, "E0004: could not write tags.");
edddd!(e0005, "E0005: one of your book folders is messed up.");
edddd!(e0006, "E0006: couldnt read child of your book folder.");
edddd!(e0007, "E0007: invalid tags.");
edddd!(e0008, "E0008: couldnt read file.");
edddd!(e0009, "E0009: couldnt read dir.");
edddd!(e0010, "E0010: not valid unicode.");
edddd!(e0011, "E0011: book doesnt exist.");
edddd!(e0012, "E0012: problematic regex pattern.");
edddd!(
    e0013,
    "E0013: couldn't search file (even though it exists)."
);
edddd!(e0015, "E0015: database error.");

fn format_error<S: Serializer, D: Debug>(err: &D, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(format!("{:#?}", err).as_str())
}

#[derive(Debug, Serialize)]
pub enum BookrabError {
    /// Responds with [`E0001_MSG`]
    /// Server couldn't turn a temporary file into a permanent file.
    CouldntSaveFile {
        #[serde(serialize_with = "e0001")]
        error: (),
        path: PathBuf,
        #[serde(serialize_with = "format_error")]
        err: std::io::Error,
    },

    /// Responds with [`E0002_MSG`]
    /// Server couldn't create a folder.
    CouldntCreateDir {
        #[serde(serialize_with = "e0002")]
        error: (),
        path: PathBuf,
        #[serde(serialize_with = "format_error")]
        err: std::io::Error,
    },

    /// Responds with [`E0003_MSG`]
    /// You shoud've inputed a text file.
    ShouldBeTextPlain {
        #[serde(serialize_with = "e0003")]
        error: (),
        filename: String,
    },

    /// Responds with [`E0004_MSG`]
    /// Server couldn't write file.
    CouldntWriteFile {
        #[serde(serialize_with = "e0004")]
        error: (),
        path: PathBuf,
        #[serde(serialize_with = "format_error")]
        err: std::io::Error,
    },

    /// Responds with [`E0005_MSG`]
    /// Your book folder is messed up. Check it out.
    MessedUpBookFolder {
        #[serde(serialize_with = "e0005")]
        error: (),
        path: PathBuf,
    },

    /// Responds with [`E0006_MSG`]
    /// Couldnt read folder inside parent.
    CouldntReadChild {
        #[serde(serialize_with = "e0006")]
        error: (),
        parent: PathBuf,
        #[serde(serialize_with = "format_error")]
        err: std::io::Error,
    },

    /// Responds with [`E0007_MSG`]
    /// Invalid tags inside book folder.
    InvalidTags {
        #[serde(serialize_with = "e0007")]
        error: (),
        tags: String,
        path: PathBuf,
        #[serde(serialize_with = "format_error")]
        err: serde_json::error::Error,
    },

    /// Responds with [`E0008_MSG`]
    /// Couldnt read folder inside parent.
    CouldntReadFile {
        #[serde(serialize_with = "e0008")]
        error: (),
        path: PathBuf,
        #[serde(serialize_with = "format_error")]
        err: std::io::Error,
    },

    /// Responds with [`E0009_MSG`]
    /// Couldnt read folder inside parent.
    CouldntReadDir {
        #[serde(serialize_with = "e0009")]
        error: (),
        path: PathBuf,
        #[serde(serialize_with = "format_error")]
        err: std::io::Error,
    },

    /// Responds with [`E0010_MSG`]
    /// Something is not Unicode.
    NotUnicode {
        #[serde(serialize_with = "e0010")]
        error: (),
        what: String,
    },

    /// Responds with [`E0011_MSG`]
    /// Book doesn't exist
    InexistentBook {
        #[serde(serialize_with = "e0011")]
        error: (),
        path: PathBuf,
    },

    /// Responds with [`E0012_MSG`]
    /// Check your regex.
    RegexProblem {
        #[serde(serialize_with = "e0012")]
        error: (),
        #[serde(serialize_with = "format_error")]
        err: grep_regex::Error,
    },

    /// Responds with [`E0013_MSG`]
    /// Book doesn't exist
    GrepSearchError {
        #[serde(serialize_with = "e0013")]
        error: (),
        path: PathBuf,
        #[serde(serialize_with = "format_error")]
        err: std::io::Error,
    },

    /// Responds with [`E0015_MSG`]
    /// Database error.
    DatabaseError {
        #[serde(serialize_with = "e0015")]
        error: (),
        #[serde(serialize_with = "format_error")]
        err: diesel::result::Error,
    },
}
impl From<grep_regex::Error> for BookrabError {
    fn from(err: grep_regex::Error) -> Self {
        BookrabError::RegexProblem { error: (), err }
    }
}
impl From<diesel::result::Error> for BookrabError {
    fn from(err: diesel::result::Error) -> Self {
        BookrabError::DatabaseError { error: (), err }
    }
}

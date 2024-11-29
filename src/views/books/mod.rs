mod list;
mod upload;
use anyhow::anyhow;
use log::error;
use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_actix_web::service_config::ServiceConfig;

use crate::errors::{
    BookrabError, CouldntCreateDir, CouldntReadChild, CouldntReadDir, CouldntReadFile,
    CouldntWriteFile, InvalidMetadata,
};

/// Represents elements returned by the listing
/// route.
#[derive(Debug, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct BookListElement {
    /// Book title
    book: String,
    /// Book metadata for filtering
    metadata: BookMetadata,
}

/// Represents a root book folder.
/// In this folder we are going to store texts and metadata
/// in the way explained bellow:
/// ```
/// path/to/root_book_dir/ <= this is the `path` we use in this struct
/// ├─ book_title1/ <= folder with the book's title as its name
/// │  ├─ txt <= full text of the book
/// │  ├─ metadata.json <= metadata of the book (see [`BookMetadata`])
/// ├─ book_title2/
/// │  ├─ txt
/// │  ├─ metadata.json
/// ```
pub struct RootBookDir {
    path: PathBuf,
}

impl RootBookDir {
    pub fn new(path: PathBuf) -> Self {
        RootBookDir { path }
    }
    /// Creates folder to store books.
    /// It ignores `std::io::ErrorKind::AlreadyExists`
    pub fn create(&self) -> io::Result<()> {
        if let Err(e) = fs::create_dir_all(&self.path) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(e);
            }
        }
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<BookListElement>, BookrabError> {
        let books_dir = match fs::read_dir(&self.path.clone()) {
            Ok(v) => v,
            Err(e) => {
                error!("{e:#?}");
                return Err(BookrabError::CouldntReadDir(
                    CouldntReadDir::new(&self.path),
                    anyhow!(e),
                ));
            }
        };
        let mut result = vec![];
        for book_dir_res in books_dir {
            let book_dir = match book_dir_res {
                Ok(v) => v,
                Err(e) => {
                    return {
                        error!("{:#?}", e);
                        Err(BookrabError::CouldntReadChild(
                            CouldntReadChild::new(
                                &self
                                    .path
                                    .to_str()
                                    .unwrap_or("path is not even valid unicode"),
                            ),
                            anyhow!(e),
                        ))
                    }
                }
            };
            let book_title = book_dir.file_name().to_str().unwrap().to_string();

            // extract metadata
            let metadata_path = book_dir.path().join("metadata.json");
            let metadata_contents;
            if metadata_path.exists() {
                metadata_contents = match fs::read_to_string(&metadata_path) {
                    Ok(v) => v,
                    Err(e) => {
                        return {
                            error!("{e:#?}");
                            Err(BookrabError::CouldntReadFile(
                                CouldntReadFile::new(&metadata_path),
                                anyhow!(e),
                            ))
                        }
                    }
                }
            } else {
                //TODO: there must be a better way of doing this
                metadata_contents = serde_json::to_string(&BookMetadata::default())
                    .expect("default metadata couldnt be parsed.");
                match fs::write(&metadata_path, metadata_contents.clone()) {
                    Ok(v) => v,
                    Err(e) => {
                        return {
                            error!("{e:#?}");
                            Err(BookrabError::CouldntWriteFile(
                                CouldntWriteFile::new(&metadata_path),
                                anyhow!(e),
                            ))
                        }
                    }
                };
            }
            let metadata_json: BookMetadata = match serde_json::from_str(metadata_contents.as_str())
            {
                Ok(v) => v,
                Err(e) => {
                    return {
                        error!("{:#?}", e);
                        Err(BookrabError::InvalidMetadata(InvalidMetadata::new(
                            metadata_contents.as_str(),
                            &metadata_path,
                        )))
                    }
                }
            };

            result.push(BookListElement {
                book: book_title,
                metadata: metadata_json,
            });
        }

        Ok(result)
    }

    /// Uploads a single book.
    pub fn upload(
        &self,
        book_name: &str,
        txt: &str,
        metadata: BookMetadata,
    ) -> Result<(), BookrabError> {
        // create book directory if it doesn't exist
        let book_path = &self.path.join(book_name);
        if let Err(e) = fs::create_dir_all(book_path) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(BookrabError::CouldntCreateDir(
                    CouldntCreateDir::new(book_path),
                    anyhow!(e),
                ));
            }
        }
        // write text
        let txt_path = book_path.join("txt");
        if let Err(e) = fs::write(&txt_path, txt) {
            return Err(BookrabError::CouldntWriteFile(
                CouldntWriteFile::new(&txt_path),
                anyhow!(e),
            ));
        };

        // write metadata
        let metadata_str = serde_json::to_string(&metadata)
            .expect("BookMetadata could not be converted to string");
        let metadata_path = book_path.join("metadata.json");
        if let Err(e) = fs::write(&metadata_path, metadata_str) {
            return Err(BookrabError::CouldntWriteFile(
                CouldntWriteFile::new(&metadata_path),
                anyhow!(e),
            ));
        };
        Ok(())
    }
}

// Book metadata for filtering
#[derive(Default, Debug, ToSchema, Deserialize, Serialize, PartialEq)]
pub struct BookMetadata {
    author: String,
    tags: Vec<String>,
}

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
    |config: &mut ServiceConfig| {
        config.service(upload::upload).service(list::list);
    }
}
#[cfg(test)]
mod tests {
    use crate::{
        config::BookrabConfig,
        views::books::{BookMetadata, RootBookDir},
    };
    use rand::{distributions::Alphanumeric, Rng};
    use std::{env::temp_dir, fs};

    use super::*;

    fn create_book_dir() -> RootBookDir {
        let random_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();

        let book_dir = temp_dir().to_path_buf().join(random_name);
        let root = RootBookDir::new(book_dir);
        root.create().expect("couldnt create root dir");
        root
    }
    fn basic_metadata() -> BookMetadata {
        BookMetadata {
            author: "Camões".to_string(),
            tags: vec!["Literatura Portuguesa".to_string()],
        }
    }
    #[actix_web::test]
    async fn basic_listing() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        let body = book_dir.list().await.unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(
            body[0],
            BookListElement {
                book: "lusiadas".to_string(),
                metadata: BookMetadata {
                    author: "Camões".to_string(),
                    tags: vec!["Literatura Portuguesa".to_string()],
                },
            }
        );
        Ok(())
    }

    #[actix_web::test]
    async fn list_two_items() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        book_dir.upload("sonetos", "", basic_metadata()).unwrap();

        let body = book_dir.list().await.unwrap();
        assert_eq!(body.len(), 2);
        assert_eq!(
            body[0],
            BookListElement {
                book: "lusiadas".to_string(),
                metadata: BookMetadata {
                    author: "Camões".to_string(),
                    tags: vec!["Literatura Portuguesa".to_string()],
                },
            }
        );
        assert_eq!(
            body[1],
            BookListElement {
                book: "sonetos".to_string(),
                metadata: BookMetadata {
                    author: "Camões".to_string(),
                    tags: vec!["Literatura Portuguesa".to_string()],
                },
            }
        );
        Ok(())
    }

    #[actix_web::test]
    async fn list_invalid_metadata() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        let metadata_path = book_dir.path.join("lusiadas").join("metadata.json");
        fs::write(&metadata_path, "meeeeeeeeeeeeeeeeeeeessed up").unwrap();

        match book_dir.list().await.unwrap_err() {
            BookrabError::InvalidMetadata(err) => {
                assert_eq!(err.metadata, "meeeeeeeeeeeeeeeeeeeessed up");
                assert_eq!(err.path, metadata_path.to_string_lossy());
            }
            _ => return Err(anyhow!("isnt invalid metadata")),
        }
        Ok(())
    }
}

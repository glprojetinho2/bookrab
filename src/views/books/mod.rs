mod list;
mod upload;
use anyhow::anyhow;
use log::error;
use std::{collections::HashSet, fs, io, path::PathBuf};

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
    title: String,
    /// Book metadata for filtering
    tags: HashSet<String>,
}

/// Manages the way that books will be filtered by tags.
pub enum FilterMode {
    /// Grabs books that have all of the tags.
    All,
    /// Grabs books that have any of the tags.
    Any,
}

/// Represents a tag filter.
pub enum TagFilter {
    Exclude(Exclude),
    Include(Include),
}
/// Excludes matched books
pub struct Exclude {
    mode: FilterMode,
    tags: HashSet<String>,
}
/// Include matched books
pub struct Include {
    mode: FilterMode,
    tags: HashSet<String>,
}
/// Represents a root book folder.
/// In this folder we are going to store texts and metadata
/// in the way explained bellow:
/// ```
/// path/to/root_book_dir/ <= this is the `path` we use in this struct
/// ├─ book_title1/ <= folder with the book's title as its name
/// │  ├─ txt <= full text of the book
/// │  ├─ tags.json <= tags
/// ├─ book_title2/
/// │  ├─ txt
/// │  ├─ tags.json
/// ```
pub struct RootBookDir {
    path: PathBuf,
}

impl RootBookDir {
    const INFO_PATH: &str = "tags.json";
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

    /// Gets book according to its title.
    pub async fn get_by_title(
        &self,
        title: String,
    ) -> Result<Option<BookListElement>, BookrabError> {
        let list = self.list().await?;
        let result: Vec<BookListElement> = list
            .into_iter()
            .filter(|book| book.title == title)
            .collect();
        // there are not going to be any duplicates
        Ok(result.into_iter().next())
    }

    /// Lists books according to their tags.
    /// No included tags = include all tags.
    /// No excluded tags = exclude no tags.
    /// These apply regardless of the mode of the inclusion/exclusion.
    pub async fn list_by_tags(
        &self,
        include: Include,
        exclude: Exclude,
    ) -> Result<Vec<BookListElement>, BookrabError> {
        let list = self.list().await?;
        let result = list
            .into_iter()
            .filter(|book| {
                let includes = if include.tags.len() > 0 {
                    match include.mode {
                        FilterMode::Any => {
                            include
                                .tags
                                .intersection(&book.tags)
                                .collect::<Vec<&String>>()
                                .len()
                                > 0
                        }
                        FilterMode::All => {
                            include.tags.union(&book.tags).collect::<Vec<_>>().len()
                                == book.tags.len()
                        }
                    }
                } else {
                    true
                };
                let excludes = if exclude.tags.len() > 0 {
                    match exclude.mode {
                        FilterMode::Any => {
                            exclude
                                .tags
                                .intersection(&book.tags)
                                .collect::<Vec<&String>>()
                                .len()
                                > 0
                        }
                        FilterMode::All => {
                            exclude.tags.union(&book.tags).collect::<Vec<_>>().len()
                                == book.tags.len()
                        }
                    }
                } else {
                    false
                };
                includes && !excludes
            })
            .collect();
        Ok(result)
    }

    /// Lists all books in the form of [BookListElement]
    pub async fn list(&self) -> Result<Vec<BookListElement>, BookrabError> {
        let books_dir = match fs::read_dir(self.path.clone()) {
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
                                self.path
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
            let tags_path = book_dir.path().join(Self::INFO_PATH);
            let tags_contents = if tags_path.exists() {
                match fs::read_to_string(&tags_path) {
                    Ok(v) => v,
                    Err(e) => {
                        return {
                            error!("{e:#?}");
                            Err(BookrabError::CouldntReadFile(
                                CouldntReadFile::new(&tags_path),
                                anyhow!(e),
                            ))
                        }
                    }
                }
            } else {
                let _ = fs::write(&tags_path, "[]");
                "[]".to_string()
            };
            let tags: HashSet<String> = match serde_json::from_str(tags_contents.as_str()) {
                Ok(v) => v,
                Err(e) => {
                    return {
                        error!("{:#?}", e);
                        Err(BookrabError::InvalidMetadata(InvalidMetadata::new(
                            tags_contents.as_str(),
                            &tags_path,
                        )))
                    }
                }
            };

            result.push(BookListElement {
                title: book_title,
                tags,
            });
        }

        Ok(result)
    }

    /// Uploads a single book.
    pub fn upload(
        &self,
        book_name: &str,
        txt: &str,
        tags: HashSet<String>,
    ) -> Result<&Self, BookrabError> {
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
        let tags_str =
            serde_json::to_string(&tags).expect("BookMetadata could not be converted to string");
        let tags_path = book_path.join(Self::INFO_PATH);
        if let Err(e) = fs::write(&tags_path, tags_str) {
            return Err(BookrabError::CouldntWriteFile(
                CouldntWriteFile::new(&tags_path),
                anyhow!(e),
            ));
        };
        Ok(self)
    }
}

pub fn configure() -> impl FnOnce(&mut ServiceConfig) {
    |config: &mut ServiceConfig| {
        config.service(upload::upload).service(list::list);
    }
}
#[cfg(test)]
mod tests {
    use crate::views::books::RootBookDir;
    use rand::{distributions::Alphanumeric, Rng};
    use std::{env::temp_dir, fs};

    use super::*;

    fn s(v: Vec<&str>) -> HashSet<String> {
        v.into_iter().map(|v| v.to_string()).collect()
    }
    fn root_for_tag_tests() -> RootBookDir {
        let book_dir = temp_dir().to_path_buf().join("tag_testing_bookrab");
        let root = RootBookDir::new(book_dir);
        if root.path.exists() {
            return root;
        }
        root.create().expect("couldnt create root dir");
        root.upload("1", "", s(vec!["a", "b", "c", "d"]))
            .unwrap()
            .upload("2", "", s(vec!["a", "b", "c"]))
            .unwrap()
            .upload("3", "", s(vec!["a", "b"]))
            .unwrap()
            .upload("4", "", s(vec!["a"]))
            .unwrap();
        root
    }
    fn create_book_dir() -> RootBookDir {
        let random_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();

        let book_dir = temp_dir()
            .to_path_buf()
            .join("bookrab-test".to_string() + &random_name);
        let root = RootBookDir::new(book_dir);
        root.create().expect("couldnt create root dir");
        root
    }
    fn basic_metadata() -> HashSet<String> {
        vec!["Camões".to_string(), "Literatura Portuguesa".to_string()]
            .into_iter()
            .collect()
    }

    #[actix_web::test]
    async fn basic_uploading() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir
            .upload(
                "lusiadas",
                "As armas e os barões assinalados",
                basic_metadata(),
            )
            .unwrap();
        let txt = fs::read_to_string(book_dir.path.join("lusiadas").join("txt"))
            .expect("couldnt read txt (file not created?)");
        let tags_txt =
            fs::read_to_string(book_dir.path.join("lusiadas").join(RootBookDir::INFO_PATH))
                .expect("couldnt read info (file not created?)");
        let tags: HashSet<String> = serde_json::from_str(&tags_txt).unwrap();
        assert_eq!(txt, "As armas e os barões assinalados");
        assert_eq!(tags, basic_metadata());
        Ok(())
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
                title: "lusiadas".to_string(),
                tags: basic_metadata(),
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
                title: "lusiadas".to_string(),
                tags: basic_metadata(),
            }
        );
        assert_eq!(
            body[1],
            BookListElement {
                title: "sonetos".to_string(),
                tags: basic_metadata(),
            }
        );
        Ok(())
    }

    #[actix_web::test]
    async fn list_invalid_metadata() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        let metadata_path = book_dir.path.join("lusiadas").join(RootBookDir::INFO_PATH);
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
    macro_rules! test_filter {
        ($name:ident, $include:expr, $exclude: expr, $expected: expr) => {
            #[actix_web::test]
            async fn $name() -> Result<(), anyhow::Error> {
                let book_dir = root_for_tag_tests();
                let include = $include;
                let exclude = $exclude;
                let books = book_dir.list_by_tags(include, exclude).await.unwrap();

                let expected = $expected;
                assert_eq!(books.len(), expected.len());
                assert_eq!(
                    books
                        .into_iter()
                        .map(|book| book.title)
                        .collect::<HashSet<_>>(),
                    expected
                );
                Ok(())
            }
        };
    }
    // here we go
    test_filter!(
        filter_include_all,
        Include {
            mode: FilterMode::All,
            tags: s(vec!["d", "c"])
        },
        Exclude {
            mode: FilterMode::All,
            tags: s(vec![]),
        },
        HashSet::from(s(vec!["1"]))
    );
    test_filter!(
        filter_include_any,
        Include {
            mode: FilterMode::Any,
            tags: s(vec!["d", "c"])
        },
        Exclude {
            mode: FilterMode::All,
            tags: s(vec![]),
        },
        HashSet::from(s(vec!["1", "2"]))
    );
    test_filter!(
        filter_exclude_all,
        Include {
            mode: FilterMode::Any,
            tags: s(vec![])
        },
        Exclude {
            mode: FilterMode::All,
            tags: s(vec!["d", "c"]),
        },
        HashSet::from(s(vec!["2", "3", "4"]))
    );
    test_filter!(
        filter_exclude_any,
        Include {
            mode: FilterMode::Any,
            tags: s(vec![])
        },
        Exclude {
            mode: FilterMode::Any,
            tags: s(vec!["d", "c"]),
        },
        HashSet::from(s(vec!["3", "4"]))
    );
    test_filter!(
        filter_include_any_exclude_any,
        Include {
            mode: FilterMode::Any,
            tags: s(vec!["b"])
        },
        Exclude {
            mode: FilterMode::Any,
            tags: s(vec!["d", "c"]),
        },
        HashSet::from(s(vec!["3"]))
    );
    test_filter!(
        filter_include_all_exclude_all,
        Include {
            mode: FilterMode::All,
            tags: s(vec!["b"])
        },
        Exclude {
            mode: FilterMode::All,
            tags: s(vec!["b", "d"]),
        },
        HashSet::from(s(vec!["2", "3"]))
    );
    test_filter!(
        filter_include_any_exclude_all,
        Include {
            mode: FilterMode::Any,
            tags: s(vec!["c", "d", "b"])
        },
        Exclude {
            mode: FilterMode::All,
            tags: s(vec!["a", "d"]),
        },
        HashSet::from(s(vec!["2", "3"]))
    );
    test_filter!(
        filter_include_all_exclude_any,
        Include {
            mode: FilterMode::All,
            tags: s(vec!["c", "d", "b"])
        },
        Exclude {
            mode: FilterMode::Any,
            tags: s(vec!["a", "d"]),
        },
        HashSet::from(s(vec![]))
    );

    #[actix_web::test]
    async fn get_by_title() -> Result<(), anyhow::Error> {
        let book_dir = create_book_dir();
        book_dir.upload("lusiadas", "", basic_metadata()).unwrap();
        let book = book_dir
            .get_by_title("lusiadas".to_string())
            .await?
            .unwrap();
        assert_eq!(
            book,
            BookListElement {
                title: "lusiadas".to_string(),
                tags: basic_metadata(),
            }
        );
        Ok(())
    }
}

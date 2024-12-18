use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookrabConfig {
    /// Folder that stores books
    pub book_path: PathBuf,
    /// URL of the database
    pub database_url: String,
}
impl std::default::Default for BookrabConfig {
    fn default() -> Self {
        let base = directories::BaseDirs::new();
        let mut book_path = PathBuf::from(".bookrab/books/");
        if base.is_some() {
            let data_dir = base.unwrap().data_local_dir().to_path_buf();
            book_path = data_dir.join("bookrab").join("books");
        };
        Self {
            book_path,
            database_url: String::from("postgres://bookrab:bookStrongPass@localhost/bookrab_db"),
        }
    }
}
/// Makes sure a config works.
pub fn ensure_config_works(config: &BookrabConfig) -> &BookrabConfig {
    //TODO: remove unwrap.
    if !config.book_path.exists() {
        fs::create_dir_all(&config.book_path).unwrap();
    };
    config
}

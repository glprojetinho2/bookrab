use std::{fs, path::Path, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BookrabConfig {
    /// Folder that stores books
    pub book_path: PathBuf,
    /// History JSON file
    pub history_path: PathBuf,
}
impl std::default::Default for BookrabConfig {
    fn default() -> Self {
        let base = directories::BaseDirs::new();
        let mut book_path = PathBuf::from(".bookrab/books/");
        let mut history_path = PathBuf::from(".bookrab/history.json");
        if base.is_some() {
            let data_dir = base.unwrap().data_local_dir().to_path_buf();
            book_path = data_dir.join("bookrab").join("books");
            history_path = data_dir.join("bookrab").join("history.json")
        };
        Self {
            book_path,
            history_path,
        }
    }
}
/// Makes sure a config works.
pub fn ensure_config_works(config: BookrabConfig) -> BookrabConfig {
    if !config.book_path.exists() {
        fs::create_dir_all(&config.book_path);
    };
    let root = &Path::new("/");
    let history_parent = config.history_path.parent().unwrap_or(root);
    if !history_parent.exists() {
        fs::create_dir_all(&history_parent);
    };
    if !config.history_path.exists() {
        fs::write(&config.history_path, "[]");
    }
    config
}
/// Loads the configuration file and makes sure it works.
pub fn ensure_confy_works() -> BookrabConfig {
    let config: BookrabConfig = confy::load("bookrab", None).unwrap();
    ensure_config_works(config)
}

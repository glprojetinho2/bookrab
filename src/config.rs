use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BookrabConfig {
    pub book_path: PathBuf,
}
impl std::default::Default for BookrabConfig {
    fn default() -> Self {
        let base = directories::BaseDirs::new();
        let mut path = PathBuf::from(".books/");
        if base.is_some() {
            path = base.unwrap().data_local_dir().to_path_buf().join("bookrab");
        };
        Self { book_path: path }
    }
}

pub fn get_config() -> BookrabConfig {
    confy::load("bookrab", None).unwrap()
}

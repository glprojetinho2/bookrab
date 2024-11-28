use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BookrabConfig {
    pub book_path: PathBuf,
}
impl std::default::Default for BookrabConfig {
    fn default() -> Self {
        Self {
            book_path: PathBuf::from(".books/"),
        }
    }
}

pub fn get_config() -> BookrabConfig {
    confy::load("bookrab", None).unwrap()
}

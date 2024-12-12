use std::fs;

use anyhow::anyhow;
use chrono::{DateTime, Utc};

use crate::{
    config::{ensure_config_works, BookrabConfig},
    errors::{BookrabError, CouldntReadFile, CouldntWriteFile, InvalidHistory},
};

use super::SearchResults;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SearchHistoryEntry {
    pub title: String,
    pub results: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

pub struct SearchHistory {
    pub config: BookrabConfig,
}
impl SearchHistory {
    pub fn new(config: BookrabConfig) -> SearchHistory {
        SearchHistory { config }
    }
    /// Gets the whole history.
    pub fn history(&self) -> Result<Vec<SearchHistoryEntry>, BookrabError> {
        let history_str = match fs::read_to_string(&self.config.history_path) {
            Ok(v) => v,
            Err(e) => {
                return Err(BookrabError::CouldntReadFile(
                    CouldntReadFile::new(&self.config.history_path),
                    anyhow!(e),
                ))
            }
        };

        match serde_json::from_str(history_str.as_str()) {
            Ok(v) => Ok(v),
            Err(_) => Err(BookrabError::InvalidHistory(InvalidHistory::new(
                history_str.as_str(),
                &self.config.history_path,
            ))),
        }
    }
    /// Appends a history entry to a JSON file.
    /// It returns ownership of the results.
    pub fn register_history_json(
        &self,
        results: Vec<SearchResults>,
    ) -> Result<Vec<SearchResults>, BookrabError> {
        let config = ensure_config_works(self.config.clone());
        let mut history = self.history()?;
        history.extend(results.clone().into_iter().map(|v| v.into()));
        match fs::write(
            dbg!(&config.history_path),
            dbg!(serde_json::to_string(&history).unwrap()),
        ) {
            Ok(v) => v,
            Err(e) => {
                return Err(BookrabError::CouldntWriteFile(
                    CouldntWriteFile::new(&config.history_path),
                    anyhow!(e),
                ))
            }
        };
        Ok(results)
    }

    /// Saves history entry somewhere.
    /// It returns ownership of the results.
    pub fn register_history(
        &self,
        results: Vec<SearchResults>,
    ) -> Result<Vec<SearchResults>, BookrabError> {
        self.register_history_json(results)
    }
}
impl From<SearchResults> for SearchHistoryEntry {
    fn from(value: SearchResults) -> Self {
        SearchHistoryEntry {
            title: value.title,
            results: value.results,
            timestamp: Utc::now(),
        }
    }
}

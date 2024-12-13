use std::fs;

use anyhow::anyhow;
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;

use crate::{
    config::{ensure_config_works, BookrabConfig},
    database::history::{NewResult, NewSearchHistoryEntry},
    errors::{BookrabError, CouldntReadFile, CouldntWriteFile, InvalidHistory},
};

use super::SearchResults;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SearchHistoryEntryJSON {
    pub title: String,
    pub pattern: String,
    pub results: Vec<String>,
    pub date: NaiveDateTime,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name=crate::schema::search_history)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchHistoryEntryPG {
    pub id: i32,
    pub title: String,
    pub pattern: String,
    pub date: NaiveDateTime,
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name=crate::schema::search_results)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchResult {
    pub id: i32,
    pub search_history_id: i32,
    pub result: String,
}

pub struct SearchHistory {
    pub config: BookrabConfig,
    /// Connection to Postgresql (this disables the JSON history).
    pub postgres_connection: Option<PgConnection>,
}

impl SearchHistory {
    pub fn new(config: BookrabConfig, postgres_connection: Option<PgConnection>) -> SearchHistory {
        SearchHistory {
            config,
            postgres_connection,
        }
    }

    /// Gets the whole history.
    pub fn history(&self) -> Result<Vec<SearchHistoryEntryJSON>, BookrabError> {
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
    pub fn register_history_json<'a>(
        &'a self,
        pattern: String,
        results: &'a Vec<SearchResults>,
    ) -> Result<&'a Vec<SearchResults>, BookrabError> {
        let config = ensure_config_works(self.config.clone());
        let mut history = self.history()?;
        history.extend(results.clone().into_iter().map(|v| SearchHistoryEntryJSON {
            pattern: pattern.clone(),
            results: v.results,
            title: v.title,
            date: Utc::now().naive_utc(),
        }));
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

    /// Appends a history entry to Postgresql table.
    /// It returns ownership of the results.
    pub fn register_history_postgres<'a>(
        &'a mut self,
        pattern: String,
        results: &'a Vec<SearchResults>,
    ) -> Result<&'a Vec<SearchResults>, BookrabError> {
        let connection = self.postgres_connection.as_mut().unwrap();
        for search_result in results {
            let in_db_history = diesel::insert_into(crate::schema::search_history::table)
                .values(NewSearchHistoryEntry {
                    pattern: &pattern,
                    title: &search_result.title,
                })
                .returning(SearchHistoryEntryPG::as_returning())
                .get_result(connection)?;

            let mut search_result_vec = vec![];
            for single_result in search_result.results.iter() {
                search_result_vec.push(NewResult {
                    search_history_id: in_db_history.id,
                    result: single_result.as_str(),
                })
            }
            diesel::insert_into(crate::schema::search_results::table)
                .values(search_result_vec)
                .execute(connection)?;
        }
        Ok(results)
    }

    /// Saves history entry somewhere.
    /// It returns ownership of the results.
    pub fn register_history<'a>(
        &'a mut self,
        pattern: String,
        results: &'a Vec<SearchResults>,
    ) -> Result<&'a Vec<SearchResults>, BookrabError> {
        match self.config.history_type {
            crate::config::HistoryType::ALL => {
                self.register_history_postgres(pattern.clone(), results)?;
                self.register_history_json(pattern, results)?;
            }
            crate::config::HistoryType::JSON => {
                self.register_history_json(pattern, results)?;
            }
            crate::config::HistoryType::POSTGRES => {
                self.register_history_postgres(pattern, results)?;
            }
        }
        Ok(results)
    }
}

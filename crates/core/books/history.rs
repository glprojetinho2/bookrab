use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::{
    config::BookrabConfig,
    database::history::{NewResult, NewSearchHistoryEntry},
    errors::BookrabError,
};

use super::SearchResults;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SearchHistoryEntryJSON<'a> {
    pub title: String,
    pub pattern: &'a str,
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
    pub postgres_connection: PgConnection,
}

impl SearchHistory {
    pub fn new(config: BookrabConfig, postgres_connection: PgConnection) -> SearchHistory {
        SearchHistory {
            config,
            postgres_connection,
        }
    }

    /// Appends a history entry to Postgresql table.
    /// It returns ownership of the results.
    pub fn register_history<'a>(
        &'a mut self,
        pattern: String,
        results: &'a Vec<SearchResults>,
    ) -> Result<&'a Vec<SearchResults>, BookrabError> {
        let connection = &mut self.postgres_connection;
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
}

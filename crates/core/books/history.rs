use diesel::prelude::*;

use crate::{
    config::BookrabConfig,
    database::{
        history::{NewResult, NewSearchHistoryEntry, SearchHistoryEntry},
        PgPooledConnection,
    },
    errors::BookrabError,
    schema,
};

use super::SearchResults;

pub struct SearchHistory<'a> {
    pub config: BookrabConfig,
    /// Connection to Postgresql
    pub connection: &'a mut PgPooledConnection,
}

impl<'a> SearchHistory<'a> {
    pub fn new(config: BookrabConfig, connection: &mut PgPooledConnection) -> SearchHistory {
        SearchHistory { config, connection }
    }

    /// Returns entire history.
    pub fn get_entire_history(self) -> Result<Vec<SearchHistoryEntry>, BookrabError> {
        match schema::search_history::table
            .order(schema::search_history::columns::date.asc())
            .load::<SearchHistoryEntry>(self.connection)
        {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into()),
        }
    }

    /// Appends a history entry to Postgresql table.
    /// It returns ownership of the results.
    pub fn register_history(
        self,
        pattern: String,
        results: &'a Vec<SearchResults>,
    ) -> Result<&'a Vec<SearchResults>, BookrabError> {
        let connection = self.connection;
        for search_result in results {
            let in_db_history = diesel::insert_into(crate::schema::search_history::table)
                .values(NewSearchHistoryEntry {
                    pattern: &pattern,
                    title: &search_result.title,
                })
                .returning(SearchHistoryEntry::as_returning())
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

#[cfg(test)]
mod tests {
    use super::SearchHistory;
    use crate::books::test_utils::create_book_dir;
    use crate::books::test_utils::DBCONNECTION;
    #[test]
    fn get_entire_history() {
        //TODO: actually test this
        let connection = &mut DBCONNECTION.get().unwrap();
        let config = create_book_dir(connection).config;
        let connection = &mut DBCONNECTION.get().unwrap();
        let history = SearchHistory::new(config, connection);
        history.get_entire_history().unwrap();
    }
}

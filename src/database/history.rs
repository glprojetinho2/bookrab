use diesel::prelude::Insertable;

use crate::schema::{search_history, search_results};

#[derive(Insertable)]
#[diesel(table_name = search_history)]
pub struct NewSearchHistoryEntry<'a> {
    pub title: &'a str,
    pub pattern: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = search_results)]
pub struct NewResult<'a> {
    pub search_history_id: i32,
    pub result: &'a str,
}

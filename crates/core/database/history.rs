use chrono::NaiveDateTime;
use diesel::{
    prelude::{Insertable, Queryable},
    Selectable,
};

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

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name=crate::schema::search_history)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchHistoryEntry {
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

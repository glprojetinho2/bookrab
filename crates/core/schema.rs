// @generated automatically by Diesel CLI.

diesel::table! {
    search_history (id) {
        id -> Int4,
        title -> Varchar,
        pattern -> Varchar,
        date -> Timestamp,
    }
}

diesel::table! {
    search_results (id) {
        id -> Int4,
        search_history_id -> Int4,
        result -> Text,
    }
}

diesel::joinable!(search_results -> search_history (search_history_id));

diesel::allow_tables_to_appear_in_same_query!(search_history, search_results,);

use bookrab_core::database::{PgPool, PgPooledConnection};
use diesel::r2d2::ConnectionManager;
use lazy_static::lazy_static;

use crate::config::ensure_confy_works;

lazy_static! {
    pub static ref DBCONNECTION: PgPool = {
        let config = ensure_confy_works();
        PgPool::builder()
            .max_size(8)
            .build(ConnectionManager::new(config.database_url))
            .expect("could not create db connection pool")
    };
}
pub struct DB {
    pub connection: PgPooledConnection,
}

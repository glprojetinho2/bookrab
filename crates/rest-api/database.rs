use actix_web::error::ErrorServiceUnavailable;
use actix_web::FromRequest;
use bookrab_core::database::{PgPool, PgPooledConnection};
use diesel::r2d2::ConnectionManager;
use futures::future::{err, ok, Ready};
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

impl FromRequest for DB {
    type Error = actix_web::Error;
    type Future = Ready<Result<DB, actix_web::Error>>;

    fn from_request(_: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        match DBCONNECTION.get() {
            Ok(connection) => ok(DB { connection }),
            Err(_) => err(ErrorServiceUnavailable("couldnt make connection to the db")),
        }
    }
}

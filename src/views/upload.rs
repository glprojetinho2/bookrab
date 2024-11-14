use actix_web::{post, Responder};

#[post("/v1/upload")]
pub async fn upload() -> impl Responder {
    String::from("bruh")
}

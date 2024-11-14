mod views;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};

async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .message_body("<h1>bruh</h1>")
        .expect("body could not be served")
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .service(views::upload::upload)
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await?;
    Ok(())
}

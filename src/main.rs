pub mod config;
pub mod errors;
mod views;
use actix_multipart::form::tempfile::TempFileConfig;
use actix_web::{middleware::Logger, App, HttpServer};
use config::get_config;
use utoipa::OpenApi;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("src/log4rs.yml", Default::default()).expect("logger didnt initialize");

    #[derive(OpenApi)]
    #[openapi(
        tags(
            (name = "bookrab", description = "Search stuff in books.")
        ),
    )]
    struct ApiDoc;

    HttpServer::new(move || {
        let config = get_config();
        App::new()
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .map(|app| app.wrap(Logger::default()))
            .service(utoipa_actix_web::scope("/v1/books").configure(views::books::configure()))
            .app_data(TempFileConfig::default().directory(&config.book_path))
            .openapi_service(|api| {
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", api)
            })
            .into_app()
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await?;
    Ok(())
}

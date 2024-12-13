use actix_files::Files;
use actix_web::dev::Service;
use books::FilterMode;
use futures_util::FutureExt;
use std::fs;
use utoipa_redoc::{Redoc, Servable};
mod books;
pub mod config;
pub mod database;
pub mod errors;
pub mod schema;
mod views;
use actix_multipart::form::tempfile::TempFileConfig;
use actix_web::{middleware::Logger, App, HttpServer};
use config::ensure_confy_works;
use utoipa::{
    openapi::{self},
    Modify, OpenApi,
};
use utoipa_actix_web::AppExt;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("src/log4rs.yml", Default::default()).expect("logger didnt initialize");

    #[derive(OpenApi)]
    #[openapi(
        info(license(name = "MIT", identifier = "MIT")),
        modifiers(&ApiDocInfo),
        components(schemas(FilterMode))
    )]
    struct ApiDoc;
    struct ApiDocInfo;
    impl Modify for ApiDocInfo {
        fn modify(&self, openapi: &mut openapi::OpenApi) {
            openapi.info.description = Some(include_str!("../README.md").to_string());
        }
    }
    let server = HttpServer::new(move || {
        let doc = ApiDoc::openapi();
        let config = ensure_confy_works();
        if !&config.book_path.is_dir() {
            fs::create_dir_all(&config.book_path).expect("couldn't create book folder");
        }
        let (app, _) = App::new()
            .into_utoipa_app()
            .openapi(doc)
            .map(|app| {
                app.wrap(Logger::default())
                    .wrap_fn(|req, srv| {
                        srv.call(req).map(|res| {
                            println!("{:#?}", res);
                            if let Err(e) = res {
                                println!("{:#?}", e);
                                return Err(e);
                            }
                            res
                        })
                    })
                    .service(Files::new("/static", "./static").show_files_listing())
            })
            .service(utoipa_actix_web::scope("/v1/books").configure(views::books::configure()))
            .app_data(TempFileConfig::default().directory(&config.book_path))
            .openapi_service(|api| Redoc::with_url("/v1/redoc", api))
            // .openapi_service(|api| {
            //     RapiDoc::with_openapi("/api-docs/openapi.json", api).path("/rapidoc")
            // })
            // .openapi_service(|api| {
            //     SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", api)
            // })
            .split_for_parts();
        app
    })
    .bind("127.0.0.1:8000")?;
    server.run().await?;
    Ok(())
}

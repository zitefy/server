
use actix_cors::Cors;
use actix_files::Files;
use actix_web::http::header;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use env_logger;
use mongodb::{options::ClientOptions, Client, Database};
use std::{sync::Arc, env};
use tokio::task;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use reqwest;

mod config;
mod handlers;
mod models;
pub mod server;
mod services;

use crate::server::domain_server;
use handlers::{user::LoginResponse, proxy_anthropic};
use models::user::{EditData, LoginData, SignupData, UserDataResponse};
use services::tempfiles::TempFileService;

// this is very cumbersome, has to be changed.
// right now, we don't have time to clean this up, but there should be a way.
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::user::signup,
        handlers::user::login,
        handlers::user::edit,
        handlers::user::upload_dp,
        handlers::user::get_data,
        handlers::user::get_profile_picture,
        handlers::user::get_sites,
        handlers::user::set_active,
        handlers::site::new_site,
        handlers::site::view_site,
        handlers::site::save_site,
        handlers::site::retrieve_resource,
        handlers::site::save_resource,
        handlers::site::get_source,
        handlers::site::change_source,
        handlers::site::preview,
        handlers::template::get_list,
        handlers::template::get_templates_by_author,
        handlers::template::search_templates,
        handlers::template::get_latest_templates,
        handlers::template::get_template_by_id,
        handlers::template::get_preview,
        serve_preview_image
    ),
    components(
        schemas(LoginData, UserDataResponse, SignupData, EditData, LoginResponse),
    ),
    tags(
        (name = "user", description = "User management endpoints"),
        (name = "site", description = "Site management endpoints"),
        (name = "template", description = "Template management endpoints"),
        (name = "proxy", description = "Proxy to external servers with CORS disallowed")
    )
)]
struct ApiDoc;

struct AppState {
    db: Database,
    secret_key: Vec<u8>,
    tempfiles: TempFileService,
    client: reqwest::Client,
    anthropic_token: String,
}

#[utoipa::path(
    get,
    path = "/preview/{id}",
    params(
        ("id" = String, Query, description = "The unique ID of the preview")
    ),
    responses(
        (status = 200, description = "Preview image served successfully", content_type = "image/*"),
        (status = 404, description = "Preview image not found")
    ),
    tag = "site"
)]
#[get("/preview/{id}")]
async fn serve_preview_image(
    req: HttpRequest,
    id: web::Path<String>,
    app_state: web::Data<Arc<AppState>>,
) -> impl Responder {
    if let Some(path) = app_state.tempfiles.get_file(&id).await {
        match actix_files::NamedFile::open(path) {
            Ok(file) => file.into_response(&req),
            Err(_) => HttpResponse::NotFound()
                .body("this preview url has likely expired. try generating a new one."),
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = config::Config::from_env();
    let client_options = ClientOptions::parse(&config.mongodb_uri).await.unwrap();
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("zitefy");

    let app_state = Arc::new(AppState {
        db: db.clone(),
        secret_key: config.secret_key.clone(),
        tempfiles: TempFileService::new(),
        client: reqwest::Client::new(),
        anthropic_token: config.anthropic_token
    });

    // Start the background task for monitoring the templates directory
    let app_state_clone = app_state.clone();
    task::spawn(async move {
        config::monitor_templates_directory(app_state_clone).await;
    });

    // Start both servers concurrently
    let api = HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://localhost:5000")
            .allowed_origin("https://zitefy.com")
            .allowed_origin("https://www.zitefy.com")
            .allowed_origin("https://api.zitefy.com")
            .allowed_origin("https://www.api.zitefy.com")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .allowed_header(header::CONTENT_TYPE)
            .max_age(3600);
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(app_state.clone()))
            .service(serve_preview_image)
            .service(proxy_anthropic)
            .configure(handlers::user::init_routes)
            .configure(handlers::template::init_routes)
            .configure(handlers::site::init_routes)
            .service(
                SwaggerUi::new("/docs/{_:.*}").url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
    })
    .bind(&config.api_addr)?
    .run();

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://localhost:5000")
            .allowed_origin("https://zitefy.com")
            .allowed_origin("https://www.zitefy.com")
            .allowed_origin("https://api.zitefy.com")
            .allowed_origin("https://www.api.zitefy.com")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
            .allowed_header(header::CONTENT_TYPE)
            .max_age(3600);
        let db = db.clone();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(Arc::new(db.clone())))
            .service(
                Files::new("/assets", format!("{}/portal/assets", env::var("HOME").unwrap()))
                    .show_files_listing()
                    .use_last_modified(true)
            )
            .service(
                Files::new("/seo", format!("{}/portal/seo", env::var("HOME").unwrap()))
                    .show_files_listing()
                    .use_last_modified(true)
            )
            .default_service(web::get().to(domain_server))
    })
    .bind(&config.server_addr)?
    .run();

    futures::future::try_join(api, server).await?;

    Ok(())
}

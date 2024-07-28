use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use actix_web::web::Json;
use mongodb::bson::{doc, oid::ObjectId};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use futures::{TryStreamExt, StreamExt};
use utoipa::ToSchema;

use crate::models::site::{Site, Data, preview_code};
use crate::handlers::user::get_user_id_from_token;
use crate::AppState;

#[derive(Deserialize, ToSchema)]
struct NewSiteRequest {
    template_id: String,
}

#[derive(Deserialize, ToSchema)]
struct SaveDataRequest {
    site_id: String,
    data: Vec<Data>,
}

#[derive(Deserialize, ToSchema)]
struct Request {
    site_id: String,
}

#[derive(Deserialize, ToSchema)]
struct ResourceRequest {
    site: String,
    resource: String
}

#[derive(Deserialize, ToSchema)]
struct PreviewRequest {
    id: String,
    wide: Option<bool>
}

#[derive(Deserialize, ToSchema)]
struct CodePreviewRequest {
    html: String,
    css: String,
    js: String,
    data: Vec<Data>
}

#[derive(Deserialize, ToSchema)]
struct RenameRequest {
    site_id: String,
    new_name: String
}

#[utoipa::path(
    post,
    path = "/site/new",
    request_body = NewSiteRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "The ID of the new site", body = Request),
        (status = 400, description = "Bad request payload"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "site"
)]
async fn new_site(
    req: HttpRequest,
    new_site_req: Json<NewSiteRequest>,
    app_state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(resp) => return Ok(resp),
    };

    let template_id = match ObjectId::parse_str(&new_site_req.template_id) {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid template ID")),
    };

    match Site::new(template_id, user_id, &app_state).await {
        Ok(site_id) => Ok(HttpResponse::Ok().json(doc! { "site_id": site_id.to_hex() })),
        Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
    }
}

#[utoipa::path(
    post,
    path = "/site/data",
    request_body = Request,
    responses(
        (status = 200, description = "The data objects of the site", body = Vec<Data>),
        (status = 400, description = "Bad request payload"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "site"
)]
async fn view_site(
    save_data_req: Json<Request>,
    app_state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, actix_web::Error> {
    let site_id = match ObjectId::parse_str(&save_data_req.site_id) {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid site ID")),
    };

    match Site::from(site_id, &app_state).await {
        Ok(site) => Ok(HttpResponse::Ok().json(site.data)),
        Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string()))
    }
}

#[utoipa::path(
    put,
    path = "/site/save",
    request_body = SaveDataRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "The ID of the new site", body = String),
        (status = 401, description = "Expired/invalid access token"),
        (status = 400, description = "Invalid site ID"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "site"
)]
async fn save_site(
    req: HttpRequest,
    payload: Json<SaveDataRequest>,
    app_state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(resp) => return Ok(resp),
    };

    let site_id = match ObjectId::parse_str(&payload.site_id) {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid site ID")),
    };

    if !Site::is_owner(site_id, user_id, &app_state).await? {
        return Ok(HttpResponse::Unauthorized().body("Unauthorized"));
    }

    match Site::save(site_id, payload.data.clone(), &app_state).await {
        Ok(_) => Ok(HttpResponse::Ok().body("Data saved successfully")),
        Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
    }
}

#[utoipa::path(
    put,
    path = "/site/rename",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "The current name of this site", body = String),
        (status = 401, description = "Expired/invalid access token"),
        (status = 400, description = "Invalid site ID"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "site"
)]
async fn rename(
    req: HttpRequest,
    payload: Json<RenameRequest>,
    app_state: web::Data<Arc<AppState>>
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(resp) => return Ok(resp),
    };

    let site_id = match ObjectId::parse_str(&payload.site_id) {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid site ID")),
    };

    if !Site::is_owner(site_id, user_id, &app_state).await? {
        return Ok(HttpResponse::Unauthorized().body("Unauthorized"));
    }

    match Site::from(site_id, &app_state).await {
        Ok(mut site) => Ok(HttpResponse::Ok().body(site.rename(payload.new_name.clone(), &app_state).await)),
        Err(_) => Ok(HttpResponse::NotFound().body("site with this id wasn't found"))
    }
}

#[utoipa::path(
    put,
    path = "/site/asset",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "The URL's of all assets", body = Vec<String>),
        (status = 401, description = "Expired/invalid access token"),
        (status = 400, description = "Invalid site ID"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "site"
)]
async fn save_resource(
    req: HttpRequest,
    mut payload: Multipart,
    app_state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(resp) => return Ok(resp),
    };

    let mut site_id = None;
    let mut file_name = None;
    let mut file_content = Vec::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition().unwrap();
        let name = content_disposition.get_name().unwrap();

        if name == "site_id" {
            while let Some(Ok(chunk)) = field.next().await {
                site_id = Some(String::from_utf8(chunk.to_vec()).unwrap());
            }
        } else if name == "file_name" {
            while let Some(Ok(chunk)) = field.next().await {
                file_name = Some(String::from_utf8(chunk.to_vec()).unwrap());
            }
        } else if name == "file" {
            while let Some(Ok(chunk)) = field.next().await {
                file_content.extend_from_slice(&chunk);
            }
        }
    }

    let site_id = match site_id {
        Some(id) => match ObjectId::parse_str(&id) {
            Ok(oid) => oid,
            Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid site ID")),
        },
        None => return Ok(HttpResponse::BadRequest().body("Site ID not provided")),
    };

    let file_name = match file_name {
        Some(name) => name,
        None => return Ok(HttpResponse::BadRequest().body("File name not provided")),
    };

    if !Site::is_owner(site_id, user_id, &app_state).await? {
        return Ok(HttpResponse::Unauthorized().body("Unauthorized"));
    }

    match Site::from(site_id, &app_state).await {
        Ok(site) => {
            match site.save_resource(&file_name, &file_content).await {
                Ok(response) => Ok(HttpResponse::Ok().json(response)),
                Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
            }
        },
        Err(_) => Ok(HttpResponse::NotFound().body("site with this id wasn't found"))
    }
}

#[utoipa::path(
    get,
    path = "/site/asset",
    params(
        ("site" = String, Query, description = "ID of the site in which resource lives"),
        ("resource" = String, Query, description = "filename of the required resource")
    ),
    responses(
        (status = 200, description = "The requested resource"),
        (status = 401, description = "Expired/invalid access token"),
        (status = 404, description = "site/asset doesn't exist"),
        (status = 400, description = "Invalid site ID"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "site"
)]
async fn retrieve_resource(
    req: HttpRequest,
    app_state: web::Data<Arc<AppState>>
) -> Result<HttpResponse, actix_web::Error> {
    let query = web::Query::<ResourceRequest>::from_query(req.query_string()).unwrap();

    let site_id = match ObjectId::parse_str(&query.site) {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid site ID")),
    };

    match Site::from(site_id, &app_state).await {
        Ok(site) => {
            match site.retrieve_resource(query.resource.clone()).await {
                Ok(file) => Ok(file.into_response(&req)),
                Err(_) => Ok(HttpResponse::NotFound().body("Requested resource doesn't exist")),
            }
        },
        Err(_) => Ok(HttpResponse::NotFound().body("site likely doesn't exist"))
    }
}

#[utoipa::path(
    post,
    path = "/site/source",
    request_body = Request,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "The source code of the site", body = Code),
        (status = 401, description = "Unauthorized user"),
        (status = 400, description = "Invalid site ID"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "site"
)]
async fn get_source(
    req: HttpRequest,
    edit_req: Json<Request>,
    app_state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(resp) => return Ok(resp),
    };

    let site_id = match ObjectId::parse_str(&edit_req.site_id) {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid site ID")),
    };

    if !Site::is_owner(site_id, user_id, &app_state).await? {
        return Ok(HttpResponse::Unauthorized().body("Unauthorized"));
    }

    match Site::from(site_id, &app_state).await {
        Ok(site) => {
            match site.get_source() {
                Ok(source) => Ok(HttpResponse::Ok().json(source)),
                Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string()))
            }
        },
        Err(_) => Ok(HttpResponse::NotFound().body("a site with this id wasn't found")),
    }
}

#[utoipa::path(
    put,
    path = "/site/source",
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "The source code of the site", body = String),
        (status = 401, description = "Unauthorized user"),
        (status = 400, description = "Invalid site ID"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "site"
)]
async fn change_source(
    req: HttpRequest,
    mut payload: Multipart,
    app_state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(resp) => return Ok(resp),
    };

    let mut site_id = None;
    let mut html_content = Vec::new();
    let mut css_content = Vec::new();
    let mut js_content = Vec::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition().unwrap();
        let name = content_disposition.get_name().unwrap();

        if name == "site_id" {
            while let Some(chunk) = field.try_next().await? {
                site_id = Some(String::from_utf8(chunk.to_vec()).unwrap());
            }
        } else if name == "html" {
            while let Some(chunk) = field.try_next().await? {
                html_content.extend_from_slice(&chunk);
            }
        } else if name == "css" {
            while let Some(chunk) = field.try_next().await? {
                css_content.extend_from_slice(&chunk);
            }
        } else if name == "js" {
            while let Some(chunk) = field.try_next().await? {
                js_content.extend_from_slice(&chunk);
            }
        }
    }

    let site_id = match site_id {
        Some(id) => match ObjectId::parse_str(&id) {
            Ok(oid) => oid,
            Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid site ID")),
        },
        None => return Ok(HttpResponse::BadRequest().body("Site ID not provided")),
    };

    if !Site::is_owner(site_id, user_id, &app_state).await? {
        return Ok(HttpResponse::Unauthorized().body("Unauthorized"));
    }

    match Site::from(site_id, &app_state).await {
        Ok(site) => match site.save_source(&html_content, &css_content, &js_content).await {
            Ok(_) => Ok(HttpResponse::Ok().body("Source code updated")),
            Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
        },
        Err(_) => Ok(HttpResponse::NotFound().body("site with this id wasn't found"))
    }
}

#[utoipa::path(
    get,
    path = "/site/preview",
    params(
        ("id" = String, Query, description = "site id"),
        ("wide" = String, Query, description = "whether the preview should be wide (desktop view) or narrow (mobile view)")
    ),
    responses(
        (status = 200, description = "image found", content_type = "image/*"),
        (status = 404, description = "site/preview not found")
    ),
    tag = "site"
)]
async fn preview(
    req: HttpRequest,
    app_state: web::Data<Arc<AppState>>
) -> Result<HttpResponse, actix_web::Error> {
    let query = web::Query::<PreviewRequest>::from_query(req.query_string()).unwrap();
    let mut is_mobile = true;

    if let Some(wide) = query.wide {
        is_mobile = wide;
    }

    let site_id = match ObjectId::parse_str(&query.id) {
        Ok(oid) => oid,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid site ID")),
    };

    match Site::from(site_id, &app_state).await {
        Ok(site) => match site.get_preview(!is_mobile).await {
            Ok(file) => Ok(file.into_response(&req)),
            Err(e) => Ok(HttpResponse::InternalServerError().body(e.to_string())),
        },
        Err(_) => Ok(HttpResponse::NotFound().body("site with this id wasn't found"))
    }
}

#[utoipa::path(
    post,
    path = "/site/preview_code",
    request_body = CodePreviewRequest,
    responses(
        (status = 200, description = "URL's resolving to the respective previews", body = Preview),
        (status = 401, description = "Unauthorized user"),
        (status = 400, description = "Invalid site ID"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "site"
)]
async fn editor_preview(payload: Json<CodePreviewRequest>, app_state: web::Data<Arc<AppState>>) -> impl Responder {
    match preview_code(&payload.html, &payload.css, &payload.js, &payload.data, &app_state).await {
        Ok((mobile, desktop)) => HttpResponse::Ok().json(json!({"mobile": mobile, "desktop": desktop})),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string())
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/site")
            .route("/new", web::post().to(new_site))
            .route("/data", web::post().to(view_site))
            .route("/save", web::put().to(save_site))
            .route("/rename", web::put().to(rename))
            .route("/asset", web::get().to(retrieve_resource))
            .route("/asset", web::put().to(save_resource))
            .route("/source", web::post().to(get_source))
            .route("/source", web::put().to(change_source))
            .route("/preview", web::get().to(preview))
            .route("/preview_code", web::post().to(editor_preview))
    );
}

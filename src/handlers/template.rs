use actix_web::{web, HttpRequest, HttpResponse, Responder};
use mongodb::bson::{doc, oid::ObjectId};
use mongodb::options::FindOptions;
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use futures::TryStreamExt;
use utoipa::ToSchema;
use std::sync::Arc;

use crate::AppState;
use crate::models::template::Template;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Query {
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthorQuery {
    pub author: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IdQuery {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PreviewQuery {
    pub id: String,
    pub wide: Option<bool>
}

#[utoipa::path(
    get,
    path = "/template/all",
    responses(
        (status = 200, description = "An array of all the available templates", body = Vec<Template>),
        (status = 400, description = "Invalid reqeust."),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "template"
)]
async fn get_list(app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let templates: Collection<Template> = app_state.db.collection("templates");
    let cursor = templates.find(None, None).await.unwrap();
    let templates: Vec<Template> = cursor.try_collect().await.unwrap();

    HttpResponse::Ok().json(templates)
}

#[utoipa::path(
    get,
    path = "/template/author",
    request_body = AuthorQuery,
    responses(
        (status = 200, description = "An array of the templates by the author", body = Vec<Tempplate>),
        (status = 400, description = "Invalid request format. Pass in an author's username to get templates."),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "template"
)]
async fn get_templates_by_author(author: web::Json<AuthorQuery>, app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let templates: Collection<Template> = app_state.db.collection("templates");
    let cursor = templates.find(doc! { "author": &author.author }, None).await.unwrap();
    let templates: Vec<Template> = cursor.try_collect().await.unwrap();

    HttpResponse::Ok().json(templates)
}

#[utoipa::path(
    get,
    path = "/site/search",
    request_body = Query,
    responses(
        (status = 200, description = "All the templates with names/author names matching the query.", body = Vec<Template>),
        (status = 400, description = "Bad request, Pass in a search query to get results."),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "template"
)]
async fn search_templates(query: web::Json<Query>, app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let templates: Collection<Template> = app_state.db.collection("templates");
    let search_query = doc! {
        "$or": [
            { "name": { "$regex": query.query.to_lowercase(), "$options": "i" } },
            { "category": { "$regex": query.query.to_lowercase(), "$options": "i" } }
        ]
    };
    let cursor = templates.find(search_query, None).await.unwrap();
    let templates: Vec<Template> = cursor.try_collect().await.unwrap();

    HttpResponse::Ok().json(templates)
}

#[utoipa::path(
    get,
    path = "/template/by_id",
    request_body = IdQuery,
    responses(
        (status = 200, description = "Template with that ID", body = Template),
        (status = 404, description = "No template by that ID", body = String),
        (status = 400, description = "Invalid site ID"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "template"
)]
async fn get_template_by_id(id: web::Json<IdQuery>, app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let templates: Collection<Template> = app_state.db.collection("templates");
    let object_id = match ObjectId::parse_str(&id.id) {
        Ok(oid) => oid,
        Err(_) => return HttpResponse::BadRequest().body("Invalid ObjectId format"),
    };
    let template = match templates.find_one(doc! { "_id": object_id }, None).await {
        Ok(Some(template)) => template,
        Ok(None) => return HttpResponse::NotFound().body("Template not found"),
        Err(_) => return HttpResponse::InternalServerError().body("Database error"),
    };

    HttpResponse::Ok().json(template)
}

#[utoipa::path(
    get,
    path = "/template/latest",
    responses(
        (status = 200, description = "Template with that ID", body = Template),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "template"
)]
async fn get_latest_templates(app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let templates: Collection<Template> = app_state.db.collection("templates");
    
    let options = FindOptions::builder()
        .sort(doc! { "time": -1 })
        .limit(3)
        .build();
    
    let cursor = templates.find(None, options).await.unwrap();
    let templates: Vec<Template> = cursor.try_collect().await.unwrap();

    HttpResponse::Ok().json(templates)
}

#[utoipa::path(
    get,
    path = "/template/preview",
    params(
        ("id" = String, Query, description = "Template ID"),
        ("wide" = String, Query, description = "whether the preview should be wide (desktop view) or narrow (mobile view)")
    ),
    responses(
        (status = 200, description = "URL resolving to the respective preview"),
        (status = 401, description = "Unauthorized user"),
        (status = 400, description = "Invalid template ID"),
        (status = 404, description = "no preview available"),
        (status = 500, description = "Internal error, contact admin.")
    ),
    tag = "template"
)]
async fn get_preview(req: HttpRequest, app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let query = web::Query::<PreviewQuery>::from_query(req.query_string()).unwrap();
    let is_mobile = match query.wide {
        Some(wide) => wide,
        None => false
    };
    let object_id = match ObjectId::parse_str(&query.id) {
        Ok(oid) => oid,
        Err(_) => return HttpResponse::BadRequest().body("Invalid template id"),
    };
    match Template::get_preview(object_id, !is_mobile, &app_state.clone()).await {
        Ok(file) => file.into_response(&req),
        Err(_) => HttpResponse::NotFound().body("no preview available")
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/template")
            .route("/all", web::get().to(get_list))
            .route("/author", web::get().to(get_templates_by_author))
            .route("/search", web::get().to(search_templates))
            .route("/by_id", web::get().to(get_template_by_id))
            .route("/latest", web::get().to(get_latest_templates))
            .route("/preview", web::get().to(get_preview))
    );
}

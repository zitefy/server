use actix_files::NamedFile;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use mongodb::{bson::doc, Collection, Database};
use std::{sync::Arc, env, path::PathBuf};

use crate::models::user::User;

// the server at zitefy.com
// checks if a username matches one in the db & if it has an active site.
// if so, serves the site, otherwise redirects to the portal
pub async fn domain_server(
    app_state: web::Data<Arc<Database>>,
    req: HttpRequest,
) -> impl Responder {
    let path = req.path().trim_start_matches('/');
    
    // Search for user in the database
    let users_collection: Collection<User> = app_state.collection("users");
    let filter = doc! { "username": path };
    
    match users_collection.find_one(filter, None).await {
        Ok(Some(user)) => {
            if let Some(quick_response) = user.quick_response {
                if !quick_response.is_empty() {
                    // Return quick_response as HTML
                    return HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body(quick_response);
                }
            }
        }
        Ok(None) => {
            // return HttpResponse::Found()
            //     .append_header(("Location", format!("/404?username={}", path)))
            //     .finish();
        }
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    }

    // If no matching user found, serve the Solid.js app
    let solid_app_path = PathBuf::from(format!("{}/portal", env::var("HOME").unwrap()));
    let index_path = solid_app_path.join("index.html");
    
    match NamedFile::open(&index_path) {
        Ok(file) => file.into_response(&req),
        Err(_) => HttpResponse::NotFound().body("File not found"),
    }
}
use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::http::Error;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use bcrypt::{hash, verify};
use chrono::{Duration, Utc};
use futures::{StreamExt, TryStreamExt};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use mongodb::bson::{doc, oid::ObjectId, Bson};
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::models::site::Site;
use crate::models::user::{EditData, LoginData, SignupData, User, UserDataResponse};
use crate::AppState;

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    token: String,
}

#[derive(Serialize, ToSchema)]
pub struct FilePayload {
    file: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: String,
    exp: usize,
}

fn generate_token(user_id: &ObjectId, secret: &[u8]) -> String {
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(24 * 3600)) // Token expires in 24 hours
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user_id.to_hex(),
        exp: expiration as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .unwrap()
}

pub fn verify_token(
    token: &str,
    secret: &[u8],
) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
}

pub async fn get_user_id_from_token(
    req: &HttpRequest,
    app_state: &web::Data<Arc<AppState>>,
) -> Result<ObjectId, HttpResponse> {
    let token = match req.headers().get("Authorization") {
        Some(header_value) => {
            let token_str = header_value.to_str().unwrap();
            if token_str.starts_with("Bearer ") {
                &token_str[7..]
            } else {
                return Err(
                    HttpResponse::Unauthorized().body("Invalid authorization header format")
                );
            }
        }
        None => return Err(HttpResponse::Unauthorized().body("No authorization header provided")),
    };

    let token_data = match verify_token(token, &app_state.secret_key) {
        Ok(data) => data,
        Err(_) => {
            return Err(HttpResponse::Unauthorized().body("Invalid token. Has likely expired."))
        }
    };

    match ObjectId::parse_str(&token_data.claims.sub) {
        Ok(id) => Ok(id),
        Err(_) => Err(HttpResponse::Unauthorized().body("Invalid token data")),
    }
}

#[utoipa::path(
    post,
    path = "/user/signup",
    request_body = SignupData,
    responses(
        (status = 200, description = "Signup successful", body = LoginResponse),
        (status = 400, description = "Bad request"),
        (status = 409, description = "Credentials already taken", body = String)
    ),
    tag = "user"
)]
async fn signup(
    data: web::Json<SignupData>,
    app_state: web::Data<Arc<AppState>>,
) -> impl Responder {
    
    if vec![
        "admin", "test", "zitefy", "login", "signup", "profile", "explore", "editor", "api", "404", "assets", "seo", ""
    ].contains(&data.username.as_str()) {
        return HttpResponse::Conflict().body("Sorry, this username is restricted.");
    }

    let users: Collection<User> = app_state.db.collection("users");

    if users
        .find_one(doc! {"email": &data.email}, None)
        .await
        .unwrap()
        .is_some()
    {
        return HttpResponse::Conflict().body("Someone has already signed up with this email ID.");
    }

    if users
        .find_one(doc! {"username": &data.username}, None)
        .await
        .unwrap()
        .is_some()
    {
        return HttpResponse::Conflict().body("Sorry, this username has either been already taken.");
    }

    let hashed_password = hash(&data.password, 10).unwrap();
    let new_user = User {
        _id: ObjectId::new(),
        name: "".to_string(),
        username: data.username.clone(),
        email: data.email.clone(),
        passwd: hashed_password,
        active: None,
        quick_response: None,
        dob: None,
        bio: None,
        links: vec![],
        pronouns: None,
        phone: None,
        image: None,
    };

    users.insert_one(new_user.clone(), None).await.unwrap();
    let token = generate_token(&new_user._id, &app_state.secret_key);

    HttpResponse::Ok().json(json!({ "token": token }))
}

#[utoipa::path(
    post,
    path = "/user/login",
    request_body = LoginData,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 400, description = "Bad request")
    ),
    tag = "user"
)]
async fn login(data: web::Json<LoginData>, app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let users: Collection<User> = app_state.db.collection("users");

    let user = users
        .find_one(
            doc! {
                "$or": [
                    { "email": &data.identifier },
                    { "username": &data.identifier }
                ]
            },
            None,
        )
        .await
        .unwrap();

    if let Some(user) = user {
        if verify(&data.password, &user.passwd).unwrap() {
            let token = generate_token(&user._id, &app_state.secret_key);
            return HttpResponse::Ok().json(LoginResponse { token });
        }
    }

    HttpResponse::Unauthorized().body("Invalid credentials")
}

macro_rules! update_field {
    ($doc:expr, $field:expr, $value:expr) => {
        if let Some(val) = $value {
            $doc.insert($field, Bson::from(val));
        }
    };
}

macro_rules! update_links {
    ($doc:expr, $field:expr, $value:expr) => {
        if let Some(val) = $value {
            let bson_array = val.into_iter().map(|v| v.to_bson()).collect::<Vec<Bson>>();
            $doc.insert($field, Bson::Array(bson_array));
        }
    };
}

#[utoipa::path(
    put,
    path = "/user/edit",
    request_body = EditData,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Edit successful", body = String),
        (status = 400, description = "Bad request"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal error, contact admin")
    ),
    tag = "user"
)]
async fn edit(
    req: HttpRequest,
    data: web::Json<EditData>,
    app_state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let users: Collection<User> = app_state.db.collection("users");

    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(response) => return response,
    };

    let mut update_doc = doc! {};

    update_field!(update_doc, "name", data.name.clone());
    update_field!(update_doc, "bio", data.bio.clone());
    update_links!(update_doc, "links", data.links.clone());
    update_field!(update_doc, "pronouns", data.pronouns.clone());
    update_field!(update_doc, "phone", data.phone.clone());
    update_field!(update_doc, "dob", data.dob.clone());

    users
        .update_one(doc! { "_id": user_id }, doc! { "$set": update_doc }, None)
        .await
        .unwrap();

    HttpResponse::Ok().body("User updated successfully")
}

#[utoipa::path(
    put,
    path = "/user/upload_dp",
    request_body = FilePayload,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Upload successful", body = String),
        (status = 404, description = "User not found"),
        (status = 400, description = "Bad request, try with a different file."),
        (status = 400, description = "Internal error. Contact admin or try again later.")
    ),
    tag = "user"
)]
async fn upload_dp(
    req: HttpRequest,
    mut payload: Multipart,
    app_state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(response) => return response,
    };

    let user_dir = format!(
        "{}/.zitefy/users/{}",
        std::env::var("HOME").unwrap(),
        user_id
    );
    if let Err(e) = fs::create_dir_all(&user_dir) {
        eprintln!("Failed to create directory: {}", e);
        return HttpResponse::InternalServerError().body("Failed to create directory");
    }

    let dp_path = format!("{}/dp.png", user_dir);

    let mut file_written = false;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition().unwrap();

        if let Some(_filename) = content_disposition.get_filename() {
            let filepath = Path::new(&dp_path);
            let mut f = match File::create(filepath) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Failed to create file: {}", e);
                    return HttpResponse::InternalServerError().body("Failed to create file");
                }
            };

            while let Some(chunk) = field.next().await {
                let data = match chunk {
                    Ok(data) => data,
                    Err(e) => {
                        eprintln!("Error reading chunk: {}", e);
                        return HttpResponse::BadRequest().body("Error reading file chunk");
                    }
                };
                if let Err(e) = f.write_all(&data) {
                    eprintln!("Error writing to file: {}", e);
                    return HttpResponse::InternalServerError().body("Error writing to file");
                }
                file_written = true;
            }
        } else {
            eprintln!("Content disposition has no filename");
        }
    }
    
    if !file_written {
        eprintln!("No file was written");
        return HttpResponse::BadRequest().body("No file was uploaded");
    }

    let users: Collection<User> = app_state.db.collection("users");
    match users
        .update_one(
            doc! { "_id": user_id },
            doc! { "$set": { "image": dp_path.clone() } },
            None,
        )
        .await
    {
        Ok(_) => HttpResponse::Ok().body("Profile picture uploaded successfully"),
        Err(e) => {
            eprintln!("Failed to update user document: {}", e);
            HttpResponse::InternalServerError().body("Failed to update user document")
        }
    }
}

#[derive(serde::Deserialize)]
struct ProfilePictureQuery {
    username: String,
}
#[utoipa::path(
    get,
    path = "/user/dp",
    params(
        ("username" = String, Query, description = "zitefy username of the user")
    ),
    responses(
        (status = 200, description = "image found", content_type = "image/*"),
        (status = 404, description = "user/image doesn't exist")
    ),
    tag = "user"
)]
async fn get_profile_picture(
    req: HttpRequest,
    app_state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let query = web::Query::<ProfilePictureQuery>::from_query(req.query_string()).unwrap();

    let users: mongodb::Collection<User> = app_state.db.collection("users");
    let user = users
        .find_one(doc! { "username": &query.username }, None)
        .await
        .unwrap();

    if let Some(user) = user {
        if let Some(image_path) = user.image {
            let path: PathBuf = image_path.into();
            return match NamedFile::open(path) {
                Ok(file) => file.into_response(&req),
                Err(_) => HttpResponse::NotFound().body("Image not found"),
            };
        } else {
            return HttpResponse::NotFound().body("No profile picture set");
        }
    } else {
        HttpResponse::NotFound().body("User not found")
    }
}

#[utoipa::path(
    get,
    path = "/user/data",
    params(
        ("id" = String, Query, description = "ID of user")
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "User's data object", body = UserDataResponse),
        (status = 404, description = "User not found"),
        (status = 400, description = "Bad request")
    ),
    tag = "user"
)]
async fn get_data(req: HttpRequest, app_state: web::Data<Arc<AppState>>) -> impl Responder {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(response) => return response,
    };

    let users: Collection<User> = app_state.db.collection("users");
    let user = users.find_one(doc! { "_id": user_id }, None).await.unwrap();

    if let Some(user) = user {
        let response = UserDataResponse {
            name: Some(user.name),
            username: user.username,
            email: user.email,
            active: user.active,
            bio: user.bio,
            links: user.links,
            pronouns: user.pronouns,
            phone: user.phone,
            dob: user.dob
        };
        HttpResponse::Ok().json(response)
    } else {
        HttpResponse::NotFound().body("User not found")
    }
}

#[derive(serde::Deserialize, ToSchema)]
struct SiteRequest {
    id: String,
}
#[utoipa::path(
    get,
    path = "/user/sites",
    responses(
        (status = 200, description = "The list of this user's sites", body = Vec<Site>),
        (status = 401, description = "Invalid access token, likely expired.")
    ),
    tag = "user"
)]
async fn get_sites(
    req: HttpRequest,
    app_state: web::Data<Arc<AppState>>,
) -> Result<HttpResponse, Error> {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    let response = match Site::get_by_user(user_id, &app_state.clone()).await {
        Ok(sites) => HttpResponse::Ok().json(sites),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    };

    Ok(response)
}

#[utoipa::path(
    post,
    path = "/user/activate",
    request_body = SiteRequest,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "The site has been activated", body = String),
        (status = 400, description = "Invalid site ID"),
        (status = 401, description = "Not this user's site"),
        (status = 404, description = "No site with this ID"),
        (status = 500, description = "Internal error, contact admin")
    ),
    tag = "user"
)]
async fn set_active(
    req: HttpRequest,
    payload: web::Json<SiteRequest>,
    app_state: web::Data<Arc<AppState>>,
) -> impl Responder {
    let user_id = match get_user_id_from_token(&req, &app_state).await {
        Ok(id) => id,
        Err(response) => return response,
    };

    let site_id = match ObjectId::parse_str(&payload.id) {
        Ok(oid) => oid,
        Err(_) => return HttpResponse::BadRequest().body("Invalid site ID"),
    };

    let users: Collection<User> = app_state.db.collection("users");
    if !Site::is_owner(site_id, user_id, &app_state).await.unwrap() {
        HttpResponse::Unauthorized().body("This user isn't authorized to use this site")
    } else {
        match Site::from(site_id, &app_state).await {
            Ok(site) => {
                match users.update_one(
                    doc! { "_id": user_id },
                    doc! { "$set": { "active": site.id, "quick_response": Some(site.get_html().await.unwrap()) } },
                    None
                ).await {
                    Ok(_) => HttpResponse::Ok().body("Set the active site!"),
                    Err(_) => HttpResponse::InternalServerError().body("Failed to update active site"),
                }
            },
            Err(_) => HttpResponse::NotFound().body("A site with this ID doesn't exist")
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/user")
            .route("/signup", web::post().to(signup))
            .route("/login", web::post().to(login))
            .route("/edit", web::put().to(edit))
            .route("/upload_dp", web::put().to(upload_dp))
            .route("/activate", web::post().to(set_active))
            .route("/data", web::get().to(get_data))
            .route("/dp", web::get().to(get_profile_picture))
            .route("/sites", web::get().to(get_sites)),
    );
}

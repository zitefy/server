use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::site::Data;

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    pub _id: ObjectId,
    pub name: String,
    pub username: String,
    pub email: String,
    pub passwd: String,
    pub active: Option<ObjectId>,
    pub quick_response: Option<String>,
    pub dob: Option<String>,
    pub bio: Option<String>,
    pub links: Vec<Data>,
    pub pronouns: Option<String>,
    pub phone: Option<String>,
    pub image: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SignupData {
    #[schema(example = "athul@email.com")]
    pub email: String,
    #[schema(example = "athulas")]
    pub username: String,
    #[schema(example = "pass123")]
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginData {
    #[schema(example = "athulas")]
    pub identifier: String,
    #[schema(example = "pass123")]
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct EditData {
    pub name: Option<String>,
    pub bio: Option<String>,
    pub links: Option<Vec<Data>>,
    pub pronouns: Option<String>,
    pub phone: Option<String>,
    pub dob: Option<String>
}

#[derive(Serialize, ToSchema)]
pub struct UserDataResponse {
    pub name: Option<String>,
    pub username: String,
    pub email: String,
    pub active: Option<ObjectId>,
    pub bio: Option<String>,
    pub dob: Option<String>,
    pub links: Vec<Data>,
    pub pronouns: Option<String>,
    pub phone: Option<String>,
}

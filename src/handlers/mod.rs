pub mod user;
pub mod template;
pub mod site;

use actix_web::{web, HttpResponse, HttpRequest, post};
use bytes::Bytes;
use std::sync::Arc;
 use crate::AppState;

#[utoipa::path(
    post,
    path = "/anthropic",
    responses(
        (status = 200, description = "The response from the API", content_type = "application/json"),
        (status = 400, description = "Bad request. Check the API docs."),
        (status = 401, description = "Bad Anthropic token"),
        (status = 404, description = "No such route in the API"),
        (status = 500, description = "Internal error communicating with the API")
    ),
    tag = "proxy"
)]
#[post("/anthropic")]
pub async fn proxy_anthropic(
    req: HttpRequest,
    payload: Bytes,
    app_state: web::Data<Arc<AppState>>,
) -> HttpResponse {
    let bearer_token = match req.headers().get("Authorization") {
        Some(token) => token.to_str().unwrap_or("").trim_start_matches("Bearer "),
        None => &app_state.anthropic_token,
    };

    let anthropic_req = app_state.client.post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", bearer_token)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "max-tokens-3-5-sonnet-2024-07-15")
        .header("content-type", "application/json")
        .body(payload);

    match anthropic_req.send().await {
        Ok(response) => {
            let status = response.status();
            let body = response.bytes().await.unwrap_or_default();
            HttpResponse::build(status).body(body)
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}
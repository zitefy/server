use std::env;
use std::sync::Arc;
use tokio::fs;
use tokio::time::{self, Duration};

use crate::AppState;
use crate::models::template::update_template_in_db;

pub struct Config {
    pub mongodb_uri: String,
    pub secret_key: Vec<u8>,
    pub anthropic_token: String,
    pub api_addr: String,
    pub server_addr: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();
        Config {
            mongodb_uri: env::var("MONGODB_URI").expect("MONGODB_URI must be set"),
            secret_key: env::var("SECRET_KEY").expect("SECRET_KEY must be set").as_bytes().to_vec(),
            anthropic_token: env::var("ANTHROPIC_KEY").expect("Anthropic account credentials must be set"),
            api_addr: env::var("API_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string()),
            server_addr: env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:5000".to_string()),
        }
    }
}

// Right now, this seems like the best way to manage the upload of templates from github
// This function is very expensive, so a better method would be to write a dedicated endpoint for uploading templates from the site,
// and adding it to the database along with it. But that's for another day.
pub async fn monitor_templates_directory(app_state: Arc<AppState>) {
    let templates_dir = format!("{}/.zitefy/templates", env::var("HOME").unwrap());

    let mut interval = time::interval(Duration::from_secs(60*60));

    loop {
        interval.tick().await;

        match fs::read_dir(&templates_dir).await {
            Ok(mut entries) => {
                while let Some(entry) = entries.next_entry().await.unwrap() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Err(e) = update_template_in_db(&path, &app_state).await {
                            eprintln!("Failed to update template in database: {}", e);
                        }
                    }
                }
            },
            Err(e) => eprintln!("Failed to read templates directory: {}", e),
        }
    }
}

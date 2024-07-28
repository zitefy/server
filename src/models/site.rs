use actix_files::NamedFile;
use actix_web::web;
use chrono::Utc;
use futures::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, Bson, to_bson};
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::to_string;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use utoipa::ToSchema;

use crate::models::template::Template;
use crate::services::preview::{build_html_string, generate_preview, Preview};
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Data {
    selector: Option<String>,
    value: Option<String>,
    link: Option<String>,
}

impl Data {
    pub fn to_bson(&self) -> Bson {
        doc! {
            "selector": self.selector.clone(),
            "value": self.value.clone(),
            "link": self.link.clone()
        }.into()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct MetaData {
    name: String,
    category: Option<String>,
    time: String,
}

impl MetaData {
    pub fn new(name: &str, category: Option<String>) -> Self {
        MetaData {
            name: String::from(name),
            category,
            time: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Site {
    #[serde(rename = "_id")]
    pub id: Option<ObjectId>,
    pub path: String,
    pub data: Vec<Data>,
    pub metadata: MetaData,
    pub user: ObjectId,
}

// most of the names & code are self-explanatory. nothing much to document per se
impl Site {
    pub async fn new(
        template_id: ObjectId,
        user_id: ObjectId,
        app_state: &Arc<AppState>,
    ) -> Result<ObjectId, Box<dyn std::error::Error>> {
        let site_id = ObjectId::new();
        let _ = Template::build_site(site_id, user_id, template_id, app_state).await?;
        let site = Site::from(site_id, app_state).await?;
        site.update_preview().await?;
        Ok(site_id)
    }

    pub async fn from(
        id: ObjectId,
        app_state: &Arc<AppState>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let sites: Collection<Site> = app_state.db.collection("sites");
        Ok(sites
            .find_one(doc! { "_id": id }, None)
            .await?
            .ok_or("site not found")?)
    }

    pub async fn rename(&mut self, new_name: String, app_state: &Arc<AppState>) -> String {
        let sites: Collection<Site> = app_state.db.collection("sites");
        match self.id {
            Some(id) => {
                let new_metadata = MetaData {
                    name: new_name.clone(),
                    category: self.metadata.category.clone(),
                    time: Utc::now().to_rfc3339(),
                };

                match sites.update_one(
                    doc! { "_id": id },
                    doc! { "$set": { "metadata": to_bson(&new_metadata).unwrap_or_default() } },
                    None
                ).await {
                    Ok(result) if result.modified_count == 1 => {
                        self.metadata = new_metadata;
                        new_name
                    },
                    Ok(_) => {
                        eprintln!("Failed to update site name in the database");
                        self.metadata.name.clone()
                    },
                    Err(e) => {
                        eprintln!("Database error: {}", e);
                        self.metadata.name.clone()
                    },
                }
            },
            None => {
                eprintln!("Site does not have an ID");
                self.metadata.name.clone()
            },
        }
    }

    pub fn get_source(self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let site_dir = Path::new(&self.path);

        let html = match read_to_string(&site_dir.join("index.html")) {
            Ok(html) => html,
            Err(_) => String::from(""),
        };

        let js = match read_dir_to_string(&site_dir.join("js")) {
            Ok(js) => js,
            Err(_) => String::from(""),
        };

        let css = match read_dir_to_string(&site_dir.join("styles")) {
            Ok(css) => css,
            Err(_) => String::from(""),
        };

        let resources = match read_dir_to_names(&site_dir.join("resources")) {
            Ok(resources) => resources,
            Err(_) => vec![],
        };

        Ok(json!({ "html": html, "js": js, "css": css, "assets": resources }))
    }

    pub async fn save_resource(
        self,
        file_name: &str,
        file_content: &[u8],
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let resources_dir = Path::new(&self.path).join("resources");

        if !resources_dir.exists() {
            fs::create_dir_all(&resources_dir)?;
        }

        let file_path = resources_dir.join(file_name);
        let mut file = File::create(file_path)?;
        file.write_all(file_content)?;

        Ok(json!({ "assets": read_dir_to_names(&resources_dir).unwrap() }))
    }

    pub async fn save_source(
        self,
        html_content: &[u8],
        css_content: &[u8],
        js_content: &[u8],
    ) -> io::Result<()> {
        let base_dir = Path::new(&self.path);
        let resources_dir = base_dir.join("resources");

        if !resources_dir.exists() {
            fs::create_dir_all(&resources_dir)?;
        }

        let html_path = base_dir.join("index.html");
        let mut html_file = File::create(html_path)?;
        html_file.write_all(html_content)?;

        let css_path = base_dir.join("styles").join("styles.css");
        let mut css_file = File::create(css_path)?;
        css_file.write_all(css_content)?;

        let js_path = base_dir.join("js").join("script.js");
        let mut js_file = File::create(js_path)?;
        js_file.write_all(js_content)?;

        self.update_preview().await.unwrap();

        Ok(())
    }

    pub async fn save(
        site_id: ObjectId,
        new_data: Vec<Data>,
        app_state: &Arc<AppState>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sites: Collection<Site> = app_state.db.collection("sites");
        sites.update_one(
            doc! { "_id": site_id }, 
            doc! { "$set": { "data": to_bson(&new_data)?.as_array().cloned().unwrap() } },
            None
        ).await?;
        Ok(())
    }

    pub async fn retrieve_resource(
        self,
        filename: String,
    ) -> Result<NamedFile, Box<dyn std::error::Error>> {
        let path = Path::new(&std::env::var("HOME")?)
            .join(".zitefy")
            .join("sites")
            .join(self.id.unwrap().to_hex())
            .join("resources")
            .join(filename);
        Ok(NamedFile::open(path)?)
    }

    pub async fn get_html(self) -> Result<String, Box<dyn std::error::Error>> {
        let base = Path::new(&self.path);
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().join("input.json");
        File::create(temp_path.clone())?.write_all(to_string(&self.data)?.as_bytes())?;

        let temp_dir_path = temp_dir.into_path();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            if let Err(e) = tokio::fs::remove_dir_all(temp_dir_path).await {
                eprintln!("Error cleaning up temporary directory: {}", e);
            }
        });
        build_html_string(
            base.join("index.html"), 
            base.join("styles").join("styles.css"), 
            base.join("js").join("script.js"), 
            Some(temp_path)
        )
    }

    pub async fn update_preview(self) -> Result<(), Box<dyn std::error::Error>> {
        let base_path = Path::new(&self.path).join("previews");
        let html = self.get_html().await?;
        if !base_path.exists() {
            fs::create_dir_all(base_path.clone()).unwrap();
        }

        generate_preview(&html, Some(Preview {
            mobile: base_path.join("mobile.png"),
            desktop: base_path.join("desktop.png")
        })).await?;
        Ok(())   
    }

    pub async fn get_preview(self, is_mobile: bool) -> Result<NamedFile, Box<dyn std::error::Error>> {
        let base_path = Path::new(&self.path).join("previews");
        let path = match is_mobile {
            true => base_path.join("mobile.png"),
            false => base_path.join("desktop.png")
        };
        Ok(NamedFile::open(&path)?)
    }

    pub async fn is_owner(
        site_id: ObjectId,
        user_id: ObjectId,
        app_state: &web::Data<Arc<AppState>>,
    ) -> Result<bool, actix_web::Error> {
        let sites: Collection<Site> = app_state.db.collection("sites");
        let filter = doc! { "_id": site_id, "user": user_id };
        match sites.find_one(filter, None).await {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(_) => Ok(false),
        }
    }

    pub async fn get_by_user(
        id: ObjectId,
        app_state: &web::Data<Arc<AppState>>,
    ) -> Result<Vec<Site>, Box<dyn std::error::Error>> {
        let collection: Collection<Site> = app_state.db.collection("sites");

        let cursor = collection.find(doc! { "user": id }, None).await?;
        let sites: Vec<Site> = cursor.try_collect().await?;

        Ok(sites)
    }
}

pub async fn preview_code(
    html: &str,
    css: &str,
    js: &str,
    data: &[Data],
    app_state: &web::Data<Arc<AppState>>,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    let html_path = temp_dir.path().join("input.html");
    let css_path = temp_dir.path().join("input.css");
    let js_path = temp_dir.path().join("input.js");
    let data_path = temp_dir.path().join("input.json");

    // Write input to temporary files
    File::create(&html_path)?.write_all(html.as_bytes())?;
    File::create(&css_path)?.write_all(css.as_bytes())?;
    File::create(&js_path)?.write_all(js.as_bytes())?;
    File::create(&data_path)?.write_all(to_string(data)?.as_bytes())?;

    let result = build_html_string(html_path, css_path, js_path, Some(data_path))?;

    // Schedule cleanup after 1 minute
    let temp_dir_path = temp_dir.into_path();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        if let Err(e) = tokio::fs::remove_dir_all(temp_dir_path).await {
            eprintln!("Error cleaning up temporary directory: {}", e);
        }
    });
    let paths = generate_preview(&result, None).await?;
    Ok((
        app_state.tempfiles.add_file(paths.mobile).await,
        app_state.tempfiles.add_file(paths.desktop).await,
    ))
}

pub fn read_to_string(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn read_dir_to_string(dir: &Path) -> io::Result<String> {
    let mut contents = String::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            contents.push_str(&read_to_string(&path)?);
            contents.push('\n');
        }
    }
    Ok(contents)
}

// recursively reads the contents of a directory to filenames and returns them
pub fn read_dir_to_names(dir: &Path) -> io::Result<Vec<String>> {
    let mut urls = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|name| name.to_str()) {
                let url = format!("{}", filename);
                urls.push(url);
            }
        }
    }
    Ok(urls)
}

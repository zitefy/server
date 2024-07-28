use actix_files::NamedFile;
use actix_web::web;
use mongodb::bson::{doc, oid::ObjectId, to_bson};
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io;
use std::fs::create_dir_all;

use crate::services::preview::{build_html_string, generate_preview, Preview};
use crate::AppState;
use crate::models::site::Site;
use crate::models::site::MetaData;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Template {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub author: String,
    pub time: String,
    pub author_link: String,
    pub category: String,
    #[serde(default)]
    pub previews: Preview,
    #[serde(default)]
    pub dir_path: String
}

impl Template {
    // add a template from metadata.json in the directory
    pub async fn from_metadata(path: &Path) -> Result<Template, Box<dyn std::error::Error>> {
        let metadata_path = path.join("metadata.json");
        let metadata_file = fs::File::open(metadata_path)?;
        let mut template: Template = serde_json::from_reader(metadata_file)?;
        template.dir_path = path.to_string_lossy().to_string();
        template.previews = template.build_preview().await?;
        Ok(template)
    }

    pub async fn build_preview(&self) -> Result<Preview, Box<dyn std::error::Error>> {
        let base_path = Path::new(&self.dir_path);
        let html = base_path.join("index.html");
        let css = base_path.join("styles/styles.css");
        let js = base_path.join("js/script.js");

        let dir = base_path.join("previews");
        if !dir.exists() { fs::create_dir_all(dir.clone())?; }
        let html = build_html_string(html, css, js, None).unwrap();
        Ok(generate_preview(&html, Some(Preview {
            mobile: dir.join("mobile.png"),
            desktop: dir.join("desktop.png")
        })).await?)
    }

    // build a site from a specified template
    pub async fn build_site(site_id: ObjectId, user_id: ObjectId, template_id: ObjectId, app_state: &Arc<AppState>) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home_dir = std::env::var("HOME")?;
        let site_dir = Path::new(&home_dir).join(".zitefy").join("sites").join(site_id.to_hex());

        if !site_dir.exists() {
            create_dir_all(&site_dir)?;
        }

        let templates: Collection<Template> = app_state.db.collection("templates");
        let template = templates.find_one(doc! { "_id": template_id }, None).await?.ok_or("Template not found")?;

        let template_dir = Path::new(&template.dir_path);
        copy_dir_all(&template_dir, &site_dir, Some("previews"))?;

        let metadata = MetaData::new(&template.name.clone(), Some(template.category.clone()));

        let sites: Collection<Site> = app_state.db.collection("sites");
        let site = Site {
            id: Some(site_id),
            path: site_dir.to_string_lossy().into_owned(),
            data: Vec::new(),
            metadata,
            user: user_id,
        };
        sites.insert_one(site, None).await?;

        Ok(site_dir)
    }
    
    pub async fn get_preview(id: ObjectId, is_mobile: bool, app_state: &web::Data<Arc<AppState>>) -> Result<NamedFile, Box<dyn std::error::Error>> {
        let templates = app_state.db.collection("templates");
        let template: Template = templates.find_one(doc! { "_id": id }, None).await?.ok_or("Template not found")?;

        let path = match is_mobile {
            true => Path::new(&template.previews.mobile),
            false => Path::new(&template.previews.desktop),
        };

        Ok(NamedFile::open(path)?)
    }

}

// copy everything from one dir to another
fn copy_dir_all(src: &Path, dst: &Path, exclude: Option<&str>) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let path = entry.path();
        
        if ty.is_dir() {
            if let Some(exclude_dir) = exclude {
                if path.file_name().unwrap_or_default() == exclude_dir {
                    continue;
                }
            }
            let new_dst = dst.join(path.file_name().unwrap());
            copy_dir_all(&path, &new_dst, None)?;
        } else {
            let new_dst = dst.join(path.file_name().unwrap());
            fs::copy(path, new_dst)?;
        }
    }
    
    Ok(())
}

// invoked by the background task to add a template to the db
pub async fn update_template_in_db(path: &Path, app_state: &Arc<AppState>) -> Result<(), Box<dyn std::error::Error>> {
    let mut template = Template::from_metadata(path).await?;
    template.dir_path = path.to_string_lossy().to_string();
    let templates: Collection<Template> = app_state.db.collection("templates");

    templates.update_one(
        doc! { "name": &template.name },
        doc! { "$set": to_bson(&template)? },
        mongodb::options::UpdateOptions::builder().upsert(true).build(),
    ).await?;

    Ok(())
}

// if ever we wanna implement deleting templates. code is redundant rn, so commenting to reduce build size.
// pub async fn delete_template_from_db(path: &Path, app_state: &Arc<AppState>) -> Result<(), Box<dyn std::error::Error>> {
//     let metadata_path = path.join("metadata.json");
//     if metadata_path.exists() {
//         let metadata_content = fs::read_to_string(metadata_path)?;
//         let template: Template = serde_json::from_str(&metadata_content)?;
//         let templates: Collection<Template> = app_state.db.collection("templates");

//         templates.delete_one(doc! { "name": &template.name }, None).await?;
//     }

//     Ok(())
// }

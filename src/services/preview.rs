use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::File;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct Preview {
    pub mobile: PathBuf,
    pub desktop: PathBuf,
}

pub async fn generate_preview(
    html: &str,
    paths: Option<Preview>,
) -> Result<Preview, Box<dyn std::error::Error>> {
    // Create a temporary directory
    let temp_dir = TempDir::new()?;
    let html_path = temp_dir.path().join("preview.html");

    let (mobile_path, desktop_path) = match paths {
        Some(path) => (path.mobile, path.desktop),
        None => (
            temp_dir.path().join("mobile_preview.png"),
            temp_dir.path().join("desktop_preview.png"),
        ),
    };

    // Write HTML to a temporary file
    let mut file = File::create(&html_path)?;
    file.write_all(html.as_bytes())?;

    Command::new("bun")
        .arg("run")
        .arg("scripts/screenshot.js")
        .arg(&html_path)
        .arg(&mobile_path)
        .arg(&desktop_path)
        .output()?;

    // Schedule cleanup after 2 minutes
    tokio::spawn(async move {
        sleep(Duration::from_secs(120)).await;
        if let Err(e) = temp_dir.close() {
            eprintln!("Error cleaning up temporary directory: {}", e);
        }
    });

    Ok(Preview {
        mobile: mobile_path, 
        desktop: desktop_path
    })
}

pub fn build_html_string(
    html: PathBuf,
    css: PathBuf,
    js: PathBuf,
    data: Option<PathBuf>,
) -> Result<String, Box<dyn std::error::Error>> {
    let data = match data {
        Some(path) => path,
        None => {
            let base_path = Path::new("scripts/");
            base_path.join("default.json")
        },
    };

    let output = Command::new("bun")
        .arg("run")
        .arg("scripts/builder.js")
        .arg(&html)
        .arg(&css)
        .arg(&js)
        .arg(&data)
        .output()?;

    if !output.status.success() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr),
        )));
    }

    Ok(String::from_utf8(output.stdout)?)
}

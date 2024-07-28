use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TempFile {
    path: PathBuf,
    expiry: u64,
}

impl TempFile {
    pub fn new(path: PathBuf) -> Self {
        let expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() + 120; // 2 minutes from now
        TempFile { path, expiry }
    }
}

pub struct TempFileService {
    files: Arc<RwLock<HashMap<String, TempFile>>>,
}

impl TempFileService {
    pub fn new() -> Self {
        TempFileService {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_file(&self, path: PathBuf) -> String {
        let id = Uuid::new_v4().to_string();
        let temp_file = TempFile::new(path);
        self.files.write().await.insert(id.clone(), temp_file);
        id
    }

    pub async fn get_file(&self, id: &str) -> Option<PathBuf> {
        let mut files = self.files.write().await;
        if let Some(file) = files.get(id) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now < file.expiry {
                return Some(file.path.clone());
            }
        }
        files.remove(id);
        None
    }
}
pub fn get_url(path: &str) -> String {
    format!("https://api.site.com/resource/{}", path)
}

pub fn get_resource(type_: &str, path: &str) -> std::path::PathBuf {
    std::path::Path::new("base_dir").join(type_).join(path)
}

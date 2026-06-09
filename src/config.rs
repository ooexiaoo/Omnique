use std::path::PathBuf;

#[allow(dead_code)]
pub struct Config {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
}

impl Config {
    pub fn load() -> Self {
        let data_dir = directories::ProjectDirs::from("com", "omnique", "omnique")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
                PathBuf::from(home).join(".local").join("share").join("omnique")
            });

        let config_dir = directories::ProjectDirs::from("com", "omnique", "omnique")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
                PathBuf::from(home).join(".config").join("omnique")
            });

        std::fs::create_dir_all(&data_dir).ok();
        std::fs::create_dir_all(&config_dir).ok();

        Self {
            data_dir,
            config_dir,
        }
    }
}

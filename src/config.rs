use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub theme: String,
    pub notes_dir: Option<String>,
    pub max_results: usize,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let (data_dir, config_dir) = Self::xdg_dirs();
        Self {
            theme: "catppuccin".to_string(),
            notes_dir: None,
            max_results: 10,
            data_dir,
            config_dir,
        }
    }
}

impl Config {
    fn xdg_dirs() -> (PathBuf, PathBuf) {
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

        (data_dir, config_dir)
    }

    pub fn load() -> Self {
        let (data_dir, config_dir) = Self::xdg_dirs();

        std::fs::create_dir_all(&data_dir).ok();
        std::fs::create_dir_all(&config_dir).ok();

        let config_path = config_dir.join("config.toml");
        let mut config = Config::default();
        config.data_dir = data_dir;
        config.config_dir = config_dir;

        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(parsed) = toml::from_str::<ConfigFile>(&content) {
                config.theme = parsed.theme.unwrap_or_else(|| "catppuccin".to_string());
                config.notes_dir = parsed.notes_dir;
                config.max_results = parsed.max_results.unwrap_or(10);
            }
        }

        config
    }
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    theme: Option<String>,
    notes_dir: Option<String>,
    max_results: Option<usize>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub primary: ratatui::style::Color,
    pub secondary: ratatui::style::Color,
    pub background: ratatui::style::Color,
    pub text: ratatui::style::Color,
    pub text_dim: ratatui::style::Color,
    pub highlight: ratatui::style::Color,
    pub selection: ratatui::style::Color,
    pub border: ratatui::style::Color,
}

impl Theme {
    pub fn catppuccin() -> Self {
        use ratatui::style::Color::Rgb;
        Self {
            primary: Rgb(0x89, 0xb4, 0xfa),    // blue
            secondary: Rgb(0xa6, 0xe3, 0xa1),  // green
            background: Rgb(0x1e, 0x1e, 0x2e), // base
            text: Rgb(0xcd, 0xd6, 0xf4),       // text
            text_dim: Rgb(0x6c, 0x70, 0x8f),   // overlay0
            highlight: Rgb(0xcb, 0xa6, 0xf7),  // mauve
            selection: Rgb(0x45, 0x47, 0x5a),   // surface0
            border: Rgb(0x89, 0xb4, 0xfa),     // blue
        }
    }

    pub fn dracula() -> Self {
        use ratatui::style::Color::Rgb;
        Self {
            primary: Rgb(0xbd, 0x93, 0xf9),    // purple
            secondary: Rgb(0x50, 0xfa, 0x7b),  // green
            background: Rgb(0x28, 0x28, 0x28), // background
            text: Rgb(0xf8, 0xf8, 0xf2),       // foreground
            text_dim: Rgb(0x62, 0x72, 0xa4),   // comment
            highlight: Rgb(0xff, 0x79, 0xc6),  // pink
            selection: Rgb(0x44, 0x44, 0x75),   // selection
            border: Rgb(0xbd, 0x93, 0xf9),     // purple
        }
    }

    pub fn nord() -> Self {
        use ratatui::style::Color::Rgb;
        Self {
            primary: Rgb(0x88, 0xc0, 0xd0),    // frost 8
            secondary: Rgb(0xa3, 0xbe, 0x8c),  // aurora green
            background: Rgb(0x2e, 0x34, 0x40), // polar night 2
            text: Rgb(0xd8, 0xde, 0xe9),       // snow storm 2
            text_dim: Rgb(0x61, 0x6e, 0x88),   // polar night 4
            highlight: Rgb(0xbf, 0x61, 0x6a),  // aurora red
            selection: Rgb(0x43, 0x4c, 0x5e),   // polar night 3
            border: Rgb(0x88, 0xc0, 0xd0),     // frost 8
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            _ => Self::catppuccin(),
        }
    }
}

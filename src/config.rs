use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub gemini_api_key: String,
    pub default_connect_amount: i8,
    pub default_interact_amount: i8,
    pub default_comment_amount: i8,
    pub rating_threshold: i32,
    pub rating_sleep_ms: u64,
    pub comment_sleep_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            gemini_api_key: String::new(),
            default_connect_amount: 10,
            default_interact_amount: 5,
            default_comment_amount: 5,
            rating_threshold: 7,
            rating_sleep_ms: 4000,
            comment_sleep_ms: 5000,
        }
    }
}

fn config_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("linky");
    fs::create_dir_all(&dir).unwrap();
    dir.join("config.json")
}

impl Config {
    pub fn load() -> Self {
        match fs::read_to_string(config_path()) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => {
                let config = Config::default();
                config.save();
                config
            }
        }
    }

    pub fn save(&self) {
        let contents = serde_json::to_string_pretty(self).unwrap();
        fs::write(config_path(), contents).unwrap();
    }
}

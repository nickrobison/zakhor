use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                path: PathBuf::from("./zakhor-db"),
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        // Start with defaults
        let mut config = Config::default();

        // Try to load TOML config file if it exists
        if let Ok(content) = std::fs::read_to_string("./zakhor.toml") {
            if let Ok(file_config) = toml::from_str::<Config>(&content) {
                config.database.path = file_config.database.path;
            }
        }

        // Environment variables override everything
        if let Ok(path) = std::env::var("ZAKHOR_DB_PATH") {
            config.database.path = PathBuf::from(path);
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.database.path, PathBuf::from("./zakhor-db"));
    }

    #[test]
    fn test_config_env_override() {
        // SAFETY: tests are single-threaded, no concurrent env access
        unsafe { std::env::set_var("ZAKHOR_DB_PATH", "/tmp/test-db") };
        let config = Config::load();
        assert_eq!(config.database.path, PathBuf::from("/tmp/test-db"));
        unsafe { std::env::remove_var("ZAKHOR_DB_PATH") };
    }
}

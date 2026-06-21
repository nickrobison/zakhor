use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    #[serde(default)]
    pub http: HttpConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                path: PathBuf::from("./zakhor-db"),
            },
            http: HttpConfig::default(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        // Start with defaults
        let mut config = Config::default();

        // Try to load TOML config file if it exists
        if let Ok(content) = std::fs::read_to_string("./zakhor.toml")
            && let Ok(file_config) = toml::from_str::<Config>(&content)
        {
            config.database.path = file_config.database.path;
            config.http.host.clone_from(&file_config.http.host);
            config.http.port = file_config.http.port;
        }

        // Environment variables override everything
        if let Ok(path) = std::env::var("ZAKHOR_DB_PATH") {
            config.database.path = PathBuf::from(path);
        }
        if let Ok(host) = std::env::var("ZAKHOR_HTTP_HOST") {
            config.http.host = host;
        }
        if let Ok(port) = std::env::var("ZAKHOR_HTTP_PORT")
            && let Ok(p) = port.parse::<u16>()
        {
            config.http.port = p;
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
        assert_eq!(config.http.host, "127.0.0.1");
        assert_eq!(config.http.port, 3000);
    }

    #[test]
    fn test_config_env_override() {
        // SAFETY: tests are single-threaded, no concurrent env access
        unsafe { std::env::set_var("ZAKHOR_DB_PATH", "/tmp/test-db") };
        let config = Config::load();
        assert_eq!(config.database.path, PathBuf::from("/tmp/test-db"));
        unsafe { std::env::remove_var("ZAKHOR_DB_PATH") };
    }

    #[test]
    fn test_http_default_config() {
        let config = Config::default();
        assert_eq!(config.http.host, "127.0.0.1");
        assert_eq!(config.http.port, 3000);
    }

    #[test]
    fn test_http_env_override() {
        // SAFETY: tests are single-threaded, no concurrent env access
        unsafe { std::env::set_var("ZAKHOR_HTTP_HOST", "0.0.0.0") };
        unsafe { std::env::set_var("ZAKHOR_HTTP_PORT", "8080") };
        let config = Config::load();
        assert_eq!(config.http.host, "0.0.0.0");
        assert_eq!(config.http.port, 8080);
        unsafe { std::env::remove_var("ZAKHOR_HTTP_HOST") };
        unsafe { std::env::remove_var("ZAKHOR_HTTP_PORT") };
    }

    #[test]
    fn test_toml_without_http_section() {
        let toml_content = r#"
[database]
path = "/custom/db"
"#;
        let config: Config =
            toml::from_str(toml_content).expect("TOML without [http] should parse");
        assert_eq!(config.database.path, PathBuf::from("/custom/db"));
        assert_eq!(config.http.host, "127.0.0.1");
        assert_eq!(config.http.port, 3000);
    }
}

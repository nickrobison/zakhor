//! Config-file discovery: resolve which `zakhor.toml` to load.
//!
//! Precedence (first match wins):
//! 1. Explicit `--config PATH` — error if the given path does not exist.
//! 2. `<cwd>/zakhor.toml` — legacy working-directory config (preserves existing setups).
//! 3. `$XDG_CONFIG_HOME/zakhor/zakhor.toml` (default `~/.config/zakhor/zakhor.toml`).
//! 4. No file found → [`ConfigSource::DefaultsOnly`] (built-in defaults + `ZAKHOR_*` env).

use std::path::{Path, PathBuf};

use super::xdg::default_config_candidates;

/// Where the loaded config came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    /// Explicit `-c/--config PATH` that exists on disk.
    Explicit(PathBuf),
    /// `<cwd>/zakhor.toml` legacy working-directory config.
    CwdToml(PathBuf),
    /// `$XDG_CONFIG_HOME/zakhor/zakhor.toml` user config.
    XdgToml(PathBuf),
    /// No config file found — use built-in defaults + env vars.
    DefaultsOnly,
}

impl ConfigSource {
    /// The filesystem path of the config file, if any.
    pub fn path(&self) -> Option<&Path> {
        match self {
            ConfigSource::Explicit(p) | ConfigSource::CwdToml(p) | ConfigSource::XdgToml(p) => {
                Some(p)
            }
            ConfigSource::DefaultsOnly => None,
        }
    }
}

/// Error returned when an explicit `--config` path is given but does not exist.
#[derive(Debug, Clone)]
pub struct DiscoveryError {
    pub path: PathBuf,
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "explicit --config path does not exist: {}",
            self.path.display()
        )
    }
}

impl std::error::Error for DiscoveryError {}

/// Discover the config source using the real working directory and XDG config home.
///
/// See the module docs for the precedence rules.
pub fn discover_config(explicit: Option<&Path>) -> Result<ConfigSource, DiscoveryError> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config_home = super::xdg::xdg_config_home();
    discover_config_in(explicit, &cwd, &config_home)
}

/// Pure core for [`discover_config`]. Accepts injected `cwd` and `config_home` so
/// the precedence ladder is unit-testable without touching process state.
pub(crate) fn discover_config_in(
    explicit: Option<&Path>,
    cwd: &Path,
    config_home: &Path,
) -> Result<ConfigSource, DiscoveryError> {
    if let Some(p) = explicit {
        if p.exists() {
            return Ok(ConfigSource::Explicit(p.to_path_buf()));
        }
        return Err(DiscoveryError {
            path: p.to_path_buf(),
        });
    }

    let mut candidates = default_config_candidates(cwd, config_home);
    // candidates[0] = cwd/zakhor.toml → CwdToml
    // candidates[1] = config_home/zakhor/zakhor.toml → XdgToml
    if let Some(cwd_candidate) = candidates.first()
        && cwd_candidate.exists()
    {
        return Ok(ConfigSource::CwdToml(cwd_candidate.clone()));
    }
    candidates.remove(0);
    if let Some(xdg_candidate) = candidates.first()
        && xdg_candidate.exists()
    {
        return Ok(ConfigSource::XdgToml(xdg_candidate.clone()));
    }
    Ok(ConfigSource::DefaultsOnly)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_tmp(prefix: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "zakhor-paths-test-{}-{}",
            prefix,
            std::process::id()
        ));
        let _ = fs::create_dir_all(&base);
        base
    }

    #[test]
    fn explicit_existing_wins() {
        let dir = unique_tmp("explicit-existing");
        let cfg = dir.join("my.toml");
        fs::write(&cfg, "").unwrap();
        let cwd = Path::new("/nonexistent-cwd");
        let xdg = Path::new("/nonexistent-xdg");
        let got =
            discover_config_in(Some(&cfg), cwd, xdg).expect("explicit existing should resolve");
        assert_eq!(got, ConfigSource::Explicit(cfg.clone()));
        fs::remove_file(&cfg).ok();
    }

    #[test]
    fn explicit_missing_errors() {
        let cfg = Path::new("/definitely/does/not/exist.toml");
        let got = discover_config_in(Some(cfg), Path::new("/any"), Path::new("/any"));
        assert!(got.is_err());
        let err = got.unwrap_err();
        assert_eq!(err.path, cfg);
    }

    #[test]
    fn cwd_beats_xdg_when_both_exist() {
        let cwd = unique_tmp("cwd-beats-xdg");
        let xdg = unique_tmp("xdg-loses");
        let cwd_cfg = cwd.join("zakhor.toml");
        let xdg_cfg = xdg.join("zakhor").join("zakhor.toml");
        fs::write(&cwd_cfg, "").unwrap();
        fs::create_dir_all(xdg_cfg.parent().unwrap()).unwrap();
        fs::write(&xdg_cfg, "").unwrap();
        let got = discover_config_in(None, &cwd, &xdg).unwrap();
        assert_eq!(got, ConfigSource::CwdToml(cwd_cfg.clone()));
        fs::remove_file(&cwd_cfg).ok();
        fs::remove_file(&xdg_cfg).ok();
    }

    #[test]
    fn xdg_used_when_cwd_absent() {
        let cwd = unique_tmp("no-cwd-cfg");
        let xdg = unique_tmp("xdg-wins");
        let xdg_cfg = xdg.join("zakhor").join("zakhor.toml");
        fs::create_dir_all(xdg_cfg.parent().unwrap()).unwrap();
        fs::write(&xdg_cfg, "").unwrap();
        let got = discover_config_in(None, &cwd, &xdg).unwrap();
        assert_eq!(got, ConfigSource::XdgToml(xdg_cfg.clone()));
        fs::remove_file(&xdg_cfg).ok();
    }

    #[test]
    fn defaults_only_when_no_candidates_exist() {
        let cwd = unique_tmp("empty-cwd");
        let xdg = unique_tmp("empty-xdg");
        let got = discover_config_in(None, &cwd, &xdg).unwrap();
        assert_eq!(got, ConfigSource::DefaultsOnly);
        assert!(got.path().is_none());
    }

    #[test]
    fn discovery_error_display_mentions_path() {
        let p = PathBuf::from("/nope.toml");
        let e = DiscoveryError { path: p.clone() };
        let s = format!("{e}");
        assert!(s.contains("/nope.toml"));
    }

    #[test]
    fn config_source_path_returns_none_for_defaults_only() {
        assert!(ConfigSource::DefaultsOnly.path().is_none());
    }
}

//! XDG Base Directory resolution for Zakhor.
//!
//! Pure cores accept injected env + fallback values so they can be unit-tested
//! without touching process-global state. The thin public wrappers read the real
//! environment and delegate to the cores.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub const APP_NAME: &str = "zakhor";
pub const CONFIG_DIR_NAME: &str = "zakhor";
pub const CONFIG_FILE_NAME: &str = "zakhor.toml";
pub const MODELS_DIR_NAME: &str = "models";

/// `$XDG_CONFIG_HOME` if absolute, else `dirs::config_dir()`, else `~/.config`.
/// Never panics: returns a best-effort path even without a home directory.
pub fn xdg_config_home() -> PathBuf {
    let env = std::env::var_os("XDG_CONFIG_HOME");
    let fallback = dirs::config_dir();
    config_home_from(env.as_deref(), fallback)
}

/// `$XDG_CACHE_HOME` if absolute, else `dirs::cache_dir()`, else `~/.cache`.
pub fn xdg_cache_home() -> PathBuf {
    let env = std::env::var_os("XDG_CACHE_HOME");
    let fallback = dirs::cache_dir();
    cache_home_from(env.as_deref(), fallback)
}

/// Pure core: resolve a config home from injected inputs.
///
/// Per the XDG spec, a *relative* `XDG_CONFIG_HOME` value is invalid and must be
/// ignored. An absolute env value wins; otherwise the platform fallback
/// (`dirs::config_dir()`) is used; otherwise `~/.config` is synthesized from the
/// home directory if discoverable; otherwise the env value is returned as-is
/// (last-resort, never panics).
pub(crate) fn config_home_from(env: Option<&OsStr>, fallback: Option<PathBuf>) -> PathBuf {
    home_from(env, fallback, ".config")
}

/// Pure core for `$XDG_CACHE_HOME`. Same semantics as [`config_home_from`] but
/// with the `.cache` fallback directory.
pub(crate) fn cache_home_from(env: Option<&OsStr>, fallback: Option<PathBuf>) -> PathBuf {
    home_from(env, fallback, ".cache")
}

fn home_from(env: Option<&OsStr>, fallback: Option<PathBuf>, dot_subdir: &str) -> PathBuf {
    if let Some(v) = env {
        let p = Path::new(v);
        if p.is_absolute() {
            return p.to_path_buf();
        }
        // Relative XDG value is ignored per spec; fall through to fallback.
    }
    if let Some(f) = fallback {
        return f;
    }
    // Last-resort synthesis from the home directory.
    if let Some(home) = dirs::home_dir() {
        return home.join(dot_subdir);
    }
    // Truly nothing: echo back the env value if any (non-absolute, but we have
    // no better option) so callers can still produce a diagnostic.
    env.map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(dot_subdir))
}

/// Default config file candidates, in precedence order:
/// 1. `<cwd>/zakhor.toml` (legacy working-directory config — checked first so
///    existing local setups keep working).
/// 2. `<config_home>/zakhor/zakhor.toml` (XDG-conventional user config).
pub fn default_config_candidates(cwd: &Path, config_home: &Path) -> Vec<PathBuf> {
    vec![
        cwd.join(CONFIG_FILE_NAME),
        config_home.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME),
    ]
}

/// `<cache_home>/zakhor/models` — the shared model cache directory.
pub fn default_models_cache_dir_from(cache_home: &Path) -> PathBuf {
    cache_home.join(CONFIG_DIR_NAME).join(MODELS_DIR_NAME)
}

/// Wrapper over [`xdg_cache_home`] returning the default shared model cache dir.
pub fn default_models_cache_dir() -> PathBuf {
    default_models_cache_dir_from(&xdg_cache_home())
}

/// Resolve the FastEmbed model cache directory.
///
/// `configured` (typically `[models].cache_dir`) wins when non-empty;
/// otherwise the XDG default ([`default_models_cache_dir`]) is used.
pub fn resolve_models_cache_dir(configured: Option<&Path>) -> PathBuf {
    match configured {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => default_models_cache_dir(),
    }
}

/// Resolve the GLiNER model cache directory.
///
/// Precedence (highest first):
/// 1. `model_dir` (`[extraction].model_dir`) when non-empty.
/// 2. `models_cache_dir` (`[models].cache_dir`) when `Some` and non-empty.
/// 3. `$HF_HUB_CACHE` env var when non-empty.
/// 4. XDG default ([`default_models_cache_dir`]).
pub fn resolve_gliner_cache_dir(model_dir: &Path, models_cache_dir: Option<&Path>) -> PathBuf {
    let hf_cache_env = std::env::var_os("HF_HUB_CACHE");
    gliner_cache_from(
        model_dir,
        models_cache_dir,
        hf_cache_env.as_deref(),
        &xdg_cache_home(),
    )
}

/// Pure core for [`resolve_gliner_cache_dir`]. Accepts injected inputs so the
/// full precedence ladder is unit-testable without env mutation.
pub(crate) fn gliner_cache_from(
    model_dir: &Path,
    configured: Option<&Path>,
    hf_cache_env: Option<&OsStr>,
    cache_home: &Path,
) -> PathBuf {
    if !model_dir.as_os_str().is_empty() {
        return model_dir.to_path_buf();
    }
    if let Some(p) = configured
        && !p.as_os_str().is_empty()
    {
        return p.to_path_buf();
    }
    if let Some(v) = hf_cache_env
        && !v.is_empty()
    {
        return PathBuf::from(v);
    }
    default_models_cache_dir_from(cache_home)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_home_ignores_relative_env() {
        // XDG spec: relative values are invalid and must be ignored.
        let got = config_home_from(Some(OsStr::new("rel/config")), Some(PathBuf::from("/etc")));
        assert_eq!(got, PathBuf::from("/etc"));
    }

    #[test]
    fn config_home_absolute_env_wins() {
        let got = config_home_from(Some(OsStr::new("/custom/xdg")), Some(PathBuf::from("/etc")));
        assert_eq!(got, PathBuf::from("/custom/xdg"));
    }

    #[test]
    fn config_home_fallback_when_env_none() {
        let got = config_home_from(None, Some(PathBuf::from("/etc")));
        assert_eq!(got, PathBuf::from("/etc"));
    }

    #[test]
    fn cache_home_relative_env_ignored() {
        let got = cache_home_from(Some(OsStr::new("rel")), Some(PathBuf::from("/var/cache")));
        assert_eq!(got, PathBuf::from("/var/cache"));
    }

    #[test]
    fn cache_home_synthesizes_from_home_when_no_fallback() {
        // No platform fallback, no env: synthesize ~/.cache from home dir.
        let got = cache_home_from(None, None);
        // We can't assert the exact home path portably, but it must end with .cache
        // and be absolute when a home dir is discoverable.
        if let Some(home) = dirs::home_dir() {
            assert_eq!(got, home.join(".cache"));
        }
    }

    #[test]
    fn candidates_cwd_first_then_xdg() {
        let cwd = Path::new("/proj");
        let cfg_home = Path::new("/home/u/.config");
        let got = default_config_candidates(cwd, cfg_home);
        assert_eq!(
            got,
            vec![
                PathBuf::from("/proj/zakhor.toml"),
                PathBuf::from("/home/u/.config/zakhor/zakhor.toml"),
            ]
        );
    }

    #[test]
    fn default_models_dir_from_cache_home() {
        let got = default_models_cache_dir_from(Path::new("/var/cache"));
        assert_eq!(got, PathBuf::from("/var/cache/zakhor/models"));
    }

    #[test]
    fn resolve_models_cache_dir_none_uses_default() {
        let got = resolve_models_cache_dir(None);
        // Should be the XDG default; just assert non-empty and ends with models.
        assert!(got.ends_with(MODELS_DIR_NAME));
    }

    #[test]
    fn resolve_models_cache_dir_empty_uses_default() {
        let got = resolve_models_cache_dir(Some(Path::new("")));
        assert!(got.ends_with(MODELS_DIR_NAME));
    }

    #[test]
    fn resolve_models_cache_dir_set_wins() {
        let got = resolve_models_cache_dir(Some(Path::new("/custom/models")));
        assert_eq!(got, PathBuf::from("/custom/models"));
    }

    #[test]
    fn gliner_precedence_model_dir_wins() {
        let got = gliner_cache_from(
            Path::new("/extraction/dir"),
            Some(Path::new("/models/cache")),
            Some(OsStr::new("/hf/hub")),
            Path::new("/var/cache"),
        );
        assert_eq!(got, PathBuf::from("/extraction/dir"));
    }

    #[test]
    fn gliner_precedence_models_cache_dir_beats_hf_env() {
        let got = gliner_cache_from(
            Path::new(""),
            Some(Path::new("/models/cache")),
            Some(OsStr::new("/hf/hub")),
            Path::new("/var/cache"),
        );
        assert_eq!(got, PathBuf::from("/models/cache"));
    }

    #[test]
    fn gliner_precedence_hf_env_beats_default() {
        let got = gliner_cache_from(
            Path::new(""),
            None,
            Some(OsStr::new("/hf/hub")),
            Path::new("/var/cache"),
        );
        assert_eq!(got, PathBuf::from("/hf/hub"));
    }

    #[test]
    fn gliner_precedence_default_when_all_empty() {
        let got = gliner_cache_from(
            Path::new(""),
            Some(Path::new("")),
            Some(OsStr::new("")),
            Path::new("/var/cache"),
        );
        assert_eq!(got, PathBuf::from("/var/cache/zakhor/models"));
    }

    #[test]
    fn gliner_precedence_empty_models_cache_skipped() {
        // Empty configured should not block the hf env fallback.
        let got = gliner_cache_from(
            Path::new(""),
            Some(Path::new("")),
            Some(OsStr::new("/hf/hub")),
            Path::new("/var/cache"),
        );
        assert_eq!(got, PathBuf::from("/hf/hub"));
    }
}

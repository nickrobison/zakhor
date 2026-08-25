//! XDG-conventional path resolution for Zakhor.
//!
//! This module centralizes:
//! - config-file discovery (explicit > `./zakhor.toml` > XDG > defaults),
//! - the shared model cache directory (`$XDG_CACHE_HOME/zakhor/models`),

//!
//! Pure cores accept injected inputs so the precedence ladders are unit-tested
//! without mutating process-global env vars.

pub mod discovery;
pub mod xdg;

pub use discovery::{ConfigSource, DiscoveryError, discover_config};
pub use xdg::{
    default_config_candidates, default_models_cache_dir, default_models_cache_dir_from,
    resolve_gliner_cache_dir, resolve_models_cache_dir, xdg_cache_home, xdg_config_home,
};

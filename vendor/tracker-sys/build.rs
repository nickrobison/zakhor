// Based on the generated build script from tracker-sys 0.8.0
// Extended to support the `vendored` feature which auto-detects
// TinySPARQL installed via Homebrew on macOS.

fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        // Prevent linking libraries to avoid documentation failure.
        return;
    }

    // When the `vendored` feature is enabled, try to locate TinySPARQL via
    // Homebrew on macOS and extend PKG_CONFIG_PATH before probing.  This
    // allows builds to succeed without the caller having to manually export
    // PKG_CONFIG_PATH when the library is installed through Homebrew.
    if std::env::var("CARGO_FEATURE_VENDORED").is_ok() {
        extend_pkg_config_path_from_homebrew();
    }

    if let Err(s) = system_deps::Config::new().probe() {
        println!("cargo:warning={s}");
        std::process::exit(1);
    }
}

/// On macOS, query Homebrew for the TinySPARQL (and general) pkg-config paths
/// and prepend them to the `PKG_CONFIG_PATH` environment variable so that
/// `system-deps` can find `tracker-sparql-3.0` even when it is not in the
/// system-wide search paths.
///
/// This function is a no-op when Homebrew is not installed or when
/// `tinysparql` has not been installed via Homebrew.
fn extend_pkg_config_path_from_homebrew() {
    let mut extra_paths: Vec<String> = Vec::new();

    // Ask Homebrew for the installation prefix of tinysparql specifically.
    if let Some(prefix) = run_command("brew", &["--prefix", "tinysparql"]) {
        let pkgconfig_dir = format!("{}/lib/pkgconfig", prefix);
        if std::path::Path::new(&pkgconfig_dir).exists() {
            extra_paths.push(pkgconfig_dir);
        }
    }

    // Also add the general Homebrew lib/pkgconfig directory so that
    // transitive dependencies (glib, gio, …) are found automatically.
    if let Some(prefix) = run_command("brew", &["--prefix"]) {
        let pkgconfig_dir = format!("{}/lib/pkgconfig", prefix);
        if std::path::Path::new(&pkgconfig_dir).exists() {
            extra_paths.push(pkgconfig_dir);
        }
    }

    // Hard-coded fallbacks for the two canonical Homebrew locations so that
    // the feature still works even when `brew` is not on PATH.
    for fallback in [
        "/opt/homebrew/lib/pkgconfig", // Apple Silicon
        "/usr/local/lib/pkgconfig",    // Intel
    ] {
        if std::path::Path::new(fallback).exists() && !extra_paths.contains(&fallback.to_string()) {
            extra_paths.push(fallback.to_string());
        }
    }

    if extra_paths.is_empty() {
        return;
    }

    let existing = std::env::var("PKG_CONFIG_PATH").unwrap_or_default();
    let new_path = if existing.is_empty() {
        extra_paths.join(":")
    } else {
        format!("{}:{}", extra_paths.join(":"), existing)
    };

    // Build scripts are single-threaded, so modifying the process environment
    // here does not cause data races.
    // SAFETY: This build script is single-threaded; no other thread reads the
    // environment concurrently, making `set_var` safe to call.
    unsafe {
        std::env::set_var("PKG_CONFIG_PATH", &new_path);
    }
}

/// Run an external command and return its trimmed stdout on success.
fn run_command(program: &str, args: &[&str]) -> Option<String> {
    std::process::Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

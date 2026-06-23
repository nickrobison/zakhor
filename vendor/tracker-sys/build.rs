// Based on the generated build script from tracker-sys 0.8.0
// Extended to support the `vendored` feature which builds TinySPARQL from
// source using meson/ninja rather than relying on a system or Homebrew package.

/// Pinned TinySPARQL release used for the vendored source build.
const TINYSPARQL_VERSION: &str = "3.7.1";
/// Minor version component of TINYSPARQL_VERSION (major.minor), used to
/// construct the GNOME download URL directory path.
const TINYSPARQL_MINOR: &str = "3.7";

fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        // Prevent linking libraries to avoid documentation failure.
        return;
    }

    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_FEATURE_VENDORED").is_ok() {
        let install_dir = build_tinysparql_from_source();
        let lib_dir = install_dir.join("lib");
        let pkgconfig_dir = lib_dir.join("pkgconfig");

        // Expose the built library directory to the root crate's build script
        // via Cargo's DEP_ mechanism so it can embed an RPATH.  The variable
        // will appear as DEP_TRACKER_SPARQL_3_0_LIB_DIR in any crate that
        // directly depends on tracker-sys (including zakhor).
        println!("cargo:lib_dir={}", lib_dir.display());

        // Prepend the built pkgconfig directory to PKG_CONFIG_PATH so that
        // the system-deps probe below finds tracker-sparql-3.0.
        //
        // SAFETY: Cargo guarantees that build scripts run on a single thread.
        // No other thread can be reading or writing the process environment
        // concurrently, so calling `set_var` here is safe.
        let existing = std::env::var("PKG_CONFIG_PATH").unwrap_or_default();
        let new_path = if existing.is_empty() {
            format!("{}", pkgconfig_dir.display())
        } else {
            format!("{}:{}", pkgconfig_dir.display(), existing)
        };
        unsafe {
            std::env::set_var("PKG_CONFIG_PATH", &new_path);
        }
    }

    if let Err(s) = system_deps::Config::new().probe() {
        println!("cargo:warning={s}");
        std::process::exit(1);
    }
}

/// Download, configure, compile, and install TinySPARQL into `OUT_DIR`.
/// Returns the installation prefix (`{OUT_DIR}/tinysparql-install`).
///
/// The function is idempotent: if `tracker-sparql-3.0.pc` already exists in
/// the install tree from a previous Cargo build, all steps are skipped so
/// that incremental builds remain fast.
fn build_tinysparql_from_source() -> std::path::PathBuf {
    let out_dir = std::path::PathBuf::from(
        std::env::var("OUT_DIR").expect("OUT_DIR must be set by Cargo"),
    );
    let install_dir = out_dir.join("tinysparql-install");
    let pc_file = install_dir
        .join("lib")
        .join("pkgconfig")
        .join("tracker-sparql-3.0.pc");

    // Fast path: skip everything if the library is already installed.
    if pc_file.exists() {
        return install_dir;
    }

    // On macOS some of TinySPARQL's build dependencies (icu4c, libxml2) are
    // "keg-only" in Homebrew and therefore absent from the default
    // PKG_CONFIG_PATH.  Query `brew --prefix <pkg>` for each and prepend the
    // result so that meson can find them during configuration.
    extend_pkg_config_for_keg_only_brew_packages();

    // ── 1. Download source tarball ─────────────────────────────────────────
    let tarball_name = format!("tinysparql-{TINYSPARQL_VERSION}.tar.xz");
    let tarball = out_dir.join(&tarball_name);
    if !tarball.exists() {
        let url = format!(
            "https://download.gnome.org/sources/tinysparql/{TINYSPARQL_MINOR}/{tarball_name}"
        );
        println!("cargo:warning=Downloading TinySPARQL {TINYSPARQL_VERSION} from {url}");
        run_or_fail(
            std::process::Command::new("curl")
                .args(["-fsSL", "-o"])
                .arg(&tarball)
                .arg(&url),
            "curl",
        );
    }

    // ── 2. Extract ─────────────────────────────────────────────────────────
    let src_dir = out_dir.join(format!("tinysparql-{TINYSPARQL_VERSION}"));
    if !src_dir.exists() {
        run_or_fail(
            std::process::Command::new("tar")
                .arg("xf")
                .arg(&tarball)
                .current_dir(&out_dir),
            "tar",
        );
    }

    // ── 3. Configure with meson ────────────────────────────────────────────
    // Check for build.ninja as the indicator of a successful meson setup.  If
    // the file is absent (e.g. a previous setup was interrupted), remove any
    // stale directory and reconfigure from scratch.
    let build_dir = out_dir.join("tinysparql-build");
    let build_ninja = build_dir.join("build.ninja");
    if !build_ninja.exists() {
        if build_dir.exists() {
            std::fs::remove_dir_all(&build_dir)
                .expect("failed to remove stale tinysparql build directory");
        }
        run_or_fail(
            std::process::Command::new("meson")
                .arg("setup")
                .arg(&build_dir)
                .arg(&src_dir)
                .arg(format!("--prefix={}", install_dir.display()))
                .arg("--libdir=lib")
                .arg("--buildtype=release")
                // Disable optional features that require extra tools or
                // are unnecessary for library-only usage:
                .arg("-Ddocs=false")
                .arg("-Dman=false")
                .arg("-Dintrospection=disabled")
                .arg("-Dvapi=disabled")
                .arg("-Dtests=false")
                .arg("-Dbash_completion=false")
                .arg("-Dsystemd_user_services=false")
                .arg("-Davahi=disabled"),
            "meson setup",
        );
    }

    // ── 4. Compile ─────────────────────────────────────────────────────────
    run_or_fail(
        std::process::Command::new("meson")
            .args(["compile", "-C"])
            .arg(&build_dir),
        "meson compile",
    );

    // ── 5. Install ─────────────────────────────────────────────────────────
    run_or_fail(
        std::process::Command::new("meson")
            .args(["install", "-C"])
            .arg(&build_dir),
        "meson install",
    );

    install_dir
}

/// Extend `PKG_CONFIG_PATH` with the pkg-config directories of "keg-only"
/// Homebrew packages that TinySPARQL requires to build but that Homebrew does
/// not add to the default search path (`icu4c`, `libxml2`).
///
/// This function is a no-op on Linux (where these libraries are in the
/// standard paths) and when Homebrew is not installed on macOS.
fn extend_pkg_config_for_keg_only_brew_packages() {
    let keg_only_packages = ["icu4c", "libxml2"];
    let mut extra_paths: Vec<String> = Vec::new();

    for pkg in &keg_only_packages {
        if let Some(prefix) = run_command("brew", &["--prefix", pkg]) {
            let pc_dir = format!("{prefix}/lib/pkgconfig");
            if std::path::Path::new(&pc_dir).exists() {
                extra_paths.push(pc_dir);
            }
        }
    }

    if extra_paths.is_empty() {
        return;
    }

    let existing = std::env::var("PKG_CONFIG_PATH").unwrap_or_default();
    // SAFETY: Cargo guarantees that build scripts run on a single thread.
    // No other thread can be reading or writing the process environment
    // concurrently, so calling `set_var` here is safe.
    let new_path = if existing.is_empty() {
        extra_paths.join(":")
    } else {
        format!("{}:{}", extra_paths.join(":"), existing)
    };
    unsafe {
        std::env::set_var("PKG_CONFIG_PATH", &new_path);
    }
}

/// Run an external command, panicking with a clear message on failure.
fn run_or_fail(cmd: &mut std::process::Command, label: &str) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("{label} could not be run: {e}"));
    assert!(
        status.success(),
        "{label} exited with status {status}; ensure all TinySPARQL build \
         dependencies (meson, ninja, pkg-config, glib, json-glib, libsoup, \
         icu4c or libunistring) are installed"
    );
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

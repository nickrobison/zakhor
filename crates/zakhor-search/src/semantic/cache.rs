use std::path::{Path, PathBuf};

/// Recursively copy a directory tree using `std::fs`.
/// Preserves symlinks as symlinks.
fn copy_recursively(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in src.read_dir()? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_symlink() {
            let target = std::fs::read_link(&src_path)?;
            std::os::unix::fs::symlink(&target, &dst_path)?;
        } else if ty.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
            copy_recursively(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Migrate the ONNX model from fastembed-rs's default cache (`.fastembed_cache/`)
/// to our custom cache directory if the custom cache is empty.
///
/// fastembed-rs's `InitOptions::with_cache_dir()` takes effect *after* hf-hub
/// resolves the model in the HuggingFace hub cache (`~/.cache/huggingface/hub/`).
/// If the user has the model in a legacy `.fastembed_cache/` location from a
/// previous version, copying it avoids a full re-download of the ~133 MB model.
pub(crate) fn migrate_cache_from_legacy_location(cache_dir: &Path) {
    let model_rel = Path::new("models--Xenova--bge-small-en-v1.5");
    let dest = cache_dir.join(model_rel);

    // Only migrate if the destination snapshot dir is missing the onnx blob.
    if dest.join("snapshots").exists() {
        return;
    }

    // Build list of candidate legacy locations, most specific first.
    let candidates = [
        // CWD-relative default used by earlier zakhor versions
        PathBuf::from(".fastembed_cache"),
        // fastembed-rs documented default (FASTEMBED_CACHE_DIR or env-var fallback)
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join(".fastembed_cache"))
            .unwrap_or_default(),
    ];

    let src = candidates.iter().find_map(|p| {
        let full = p.join(model_rel);
        if full.join("snapshots").exists() {
            Some(full)
        } else {
            None
        }
    });

    let Some(src) = src else {
        return;
    };

    tracing::info!(
        "Migrating embedding model from legacy cache {:?} to {:?}",
        src,
        dest
    );

    if let Err(e) = std::fs::create_dir_all(&dest).and_then(|()| copy_recursively(&src, &dest)) {
        tracing::warn!(
            error = %e,
            "Failed to migrate legacy model cache — will re-download if needed"
        );
    }
}

//! One-time, best-effort migration of legacy model cache directories into the
//! unified shared cache. Never fatal: errors are logged and skipped.

use std::path::{Path, PathBuf};

/// Outcome of a migration pass.
#[derive(Debug, Default, Clone)]
pub struct MigrationReport {
    /// `(source_child, target_child)` pairs moved successfully.
    pub moved: Vec<(PathBuf, PathBuf)>,
    /// Human-readable descriptions of items skipped (e.g. target already present).
    pub skipped: Vec<String>,
    /// Human-readable descriptions of failures (e.g. IO errors).
    pub failed: Vec<String>,
}

impl std::fmt::Display for MigrationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "moved={} skipped={} failed={}",
            self.moved.len(),
            self.skipped.len(),
            self.failed.len()
        )?;
        if !self.moved.is_empty() {
            f.write_str("; moved: ")?;
            let mut first = true;
            for (s, d) in &self.moved {
                if !first {
                    f.write_str(", ")?;
                }
                first = false;
                write!(f, "{} -> {}", s.display(), d.display())?;
            }
        }
        if !self.failed.is_empty() {
            f.write_str("; failed: ")?;
            let mut first = true;
            for e in &self.failed {
                if !first {
                    f.write_str("; ")?;
                }
                first = false;
                f.write_str(e)?;
            }
        }
        Ok(())
    }
}

/// Recursively copy a directory tree using `std::fs`.
/// Preserves symlinks as symlinks (unix).
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in src.read_dir()? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_symlink() {
            let target = std::fs::read_link(&src_path)?;
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&target, &dst_path)?;
            }
            #[cfg(not(unix))]
            {
                // Non-unix: copy through the link target as a regular file/dir.
                if ty.is_dir() {
                    std::fs::create_dir_all(&dst_path)?;
                    copy_dir_recursive(&src_path, &dst_path)?;
                } else {
                    std::fs::copy(&src_path, &dst_path)?;
                }
            }
        } else if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Build the list of legacy model-cache source directories to inspect.
///
/// Returns, in order:
/// 1. `<db_path>/semantic/fastembed-cache` (pre-unification FastEmbed cache).
/// 2. `<cwd>/.fastembed_cache` (fastembed-rs default).
/// 3. Every `<cwd>/models--*` directory (hf-hub content-addressed dirs leaked
///    into the working directory by GLiNER's `.` fallback).
pub fn legacy_model_cache_sources(db_path: &Path, cwd: &Path) -> Vec<PathBuf> {
    let mut sources = vec![
        db_path.join("semantic").join("fastembed-cache"),
        cwd.join(".fastembed_cache"),
    ];

    // Collect stray hf-hub dirs in the working directory.
    let mut stray: Vec<PathBuf> = Vec::new();
    if let Ok(read_dir) = cwd.read_dir() {
        for entry in read_dir.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("models--") {
                let p = entry.path();
                if p.is_dir() {
                    stray.push(p);
                }
            }
        }
    }
    stray.sort();
    sources.extend(stray);
    sources
}

/// Move the top-level children of each `sources` entry into `target`.
///
/// Best-effort and never fatal:
/// - Nonexistent sources are skipped silently.
/// - A source whose canonicalized path equals `target` is skipped entirely.
/// - For each top-level child `E` of a source: if `target/E` already exists the
///   child is recorded as skipped; otherwise it is copied into `target/E` and,
///   on success, removed from the source and recorded as moved.
/// - After processing a source, if it is now empty it is removed.
/// - Any IO error is recorded in `failed` and logged via `tracing::warn!`;
///   processing continues with the next child/source.
pub fn migrate_legacy_model_caches(target: &Path, sources: &[PathBuf]) -> MigrationReport {
    let mut report = MigrationReport::default();

    // Canonicalize target once for the self-comparison guard.
    let target_canon = target.canonicalize().ok();

    if let Err(e) = std::fs::create_dir_all(target) {
        report
            .failed
            .push(format!("create target {}: {e}", target.display()));
        return report;
    }

    for source in sources {
        if !source.exists() {
            continue;
        }
        if let (Some(src_canon), Some(tgt_canon)) =
            (source.canonicalize().ok(), target_canon.as_ref())
            && src_canon == *tgt_canon
        {
            report
                .skipped
                .push(format!("source equals target: {}", source.display()));
            continue;
        }

        let read_dir = match source.read_dir() {
            Ok(rd) => rd,
            Err(e) => {
                report
                    .failed
                    .push(format!("read {}: {e}", source.display()));
                tracing::warn!(error = %e, source = %source.display(), "migration: read_dir failed");
                continue;
            }
        };

        for entry in read_dir.flatten() {
            let child_name = entry.file_name();
            let src_child = entry.path();
            let dst_child = target.join(&child_name);

            if dst_child.exists() {
                report
                    .skipped
                    .push(format!("already present: {}", dst_child.display()));
                continue;
            }

            let copy_result = match std::fs::metadata(&src_child) {
                Ok(meta) if meta.is_dir() => copy_dir_recursive(&src_child, &dst_child),
                Ok(_) => {
                    if let Err(e) = std::fs::create_dir_all(target) {
                        Err(e)
                    } else {
                        std::fs::copy(&src_child, &dst_child).map(|_| ())
                    }
                }
                Err(e) => Err(e),
            };
            match copy_result {
                Ok(()) => {
                    let removal = if src_child.is_dir() {
                        std::fs::remove_dir_all(&src_child)
                    } else {
                        std::fs::remove_file(&src_child)
                    };
                    if let Err(e) = removal {
                        report
                            .failed
                            .push(format!("remove source child {}: {e}", src_child.display()));
                        tracing::warn!(
                            error = %e,
                            child = %src_child.display(),
                            "migration: remove_dir_all failed"
                        );
                    } else {
                        tracing::info!(
                            from = %src_child.display(),
                            to = %dst_child.display(),
                            "migrated model cache entry"
                        );
                        report.moved.push((src_child, dst_child));
                    }
                }
                Err(e) => {
                    report
                        .failed
                        .push(format!("copy {}: {e}", src_child.display()));
                    tracing::warn!(
                        error = %e,
                        child = %src_child.display(),
                        "migration: copy_dir_recursive failed"
                    );
                }
            }
        }

        if source
            .read_dir()
            .map(|mut rd| rd.next().is_none())
            .unwrap_or(false)
            && let Err(e) = std::fs::remove_dir(source)
        {
            tracing::warn!(error = %e, source = %source.display(), "migration: remove empty source failed");
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_tmp(prefix: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "zakhor-migrate-test-{}-{}",
            prefix,
            std::process::id()
        ));
        // Use a per-test subdirectory to avoid collisions between parallel tests.
        let dir = base.join(prefix);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn make_tree(root: &Path, rel: &str, contents: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, contents).unwrap();
    }

    #[test]
    fn happy_path_moves_children_and_empties_source() {
        let target = unique_tmp("happy-target");
        let source = unique_tmp("happy-source");
        // fake hf-hub tree
        make_tree(&source, "models--Org--Repo/snapshots/abc/blob.onnx", "data");
        make_tree(&source, "models--Org--Repo/blobs/abc", "data");
        make_tree(&source, "other-file", "x");

        let report = migrate_legacy_model_caches(&target, std::slice::from_ref(&source));
        assert!(report.failed.is_empty(), "failed: {:?}", report.failed);
        assert_eq!(report.moved.len(), 2); // models--Org--Repo and other-file
        // Source dir should now be removed.
        assert!(!source.exists(), "source should be removed after migration");
        // Target has both top-level entries.
        assert!(target.join("models--Org--Repo").exists());
        assert!(target.join("other-file").exists());
        assert!(
            target
                .join("models--Org--Repo/snapshots/abc/blob.onnx")
                .exists()
        );
    }

    #[test]
    fn idempotent_rerun_moves_nothing() {
        let target = unique_tmp("idempotent-target");
        let source = unique_tmp("idempotent-source");
        make_tree(&source, "models--Org--Repo/snapshots/abc/blob.onnx", "data");
        let _ = migrate_legacy_model_caches(&target, std::slice::from_ref(&source));
        // Source now gone; rerun with a fresh (now-empty) source dir.
        let _ = fs::create_dir_all(&source);
        let report = migrate_legacy_model_caches(&target, std::slice::from_ref(&source));
        assert!(report.moved.is_empty());
        assert!(report.failed.is_empty());
    }

    #[test]
    fn existing_child_skip_retains_source() {
        let target = unique_tmp("existing-target");
        let source = unique_tmp("existing-source");
        // Pre-seed target with the same top-level entry.
        make_tree(&target, "models--Org--Repo/blobs/already", "present");
        // Source has the same top-level entry with a different snapshot.
        make_tree(&source, "models--Org--Repo/snapshots/new/blob.onnx", "data");

        let report = migrate_legacy_model_caches(&target, std::slice::from_ref(&source));
        assert!(
            report.moved.is_empty(),
            "should not move when target exists"
        );
        assert!(!report.skipped.is_empty());
        // Source child is retained (not removed).
        assert!(source.join("models--Org--Repo").exists());
    }

    #[test]
    fn nonexistent_sources_skipped_silently() {
        let target = unique_tmp("nonexist-target");
        let report =
            migrate_legacy_model_caches(&target, &[PathBuf::from("/definitely/not/here/xyz")]);
        assert!(report.moved.is_empty());
        assert!(report.failed.is_empty());
        assert!(report.skipped.is_empty());
    }

    #[test]
    fn source_equal_to_target_skipped() {
        let dir = unique_tmp("self-eq");
        let report = migrate_legacy_model_caches(&dir, std::slice::from_ref(&dir));
        // Self-migration is skipped.
        assert!(report.moved.is_empty());
        assert!(report.failed.is_empty());
    }

    #[test]
    fn legacy_sources_lists_models_dash_dirs() {
        let cwd = unique_tmp("legacy-cwd");
        // Plant two stray models-- dirs and a non-matching dir.
        fs::create_dir_all(cwd.join("models--Org--Repo")).unwrap();
        fs::create_dir_all(cwd.join("models--Other--Repo")).unwrap();
        fs::create_dir_all(cwd.join("not-a-model")).unwrap();
        let db = unique_tmp("legacy-db");

        let got = legacy_model_cache_sources(&db, &cwd);
        assert!(got.contains(&db.join("semantic").join("fastembed-cache")));
        assert!(got.contains(&cwd.join(".fastembed_cache")));
        // Both models--* dirs present, sorted; non-matching excluded.
        let stray: Vec<&PathBuf> = got
            .iter()
            .filter(|p| p.to_string_lossy().contains("models--"))
            .collect();
        assert_eq!(stray.len(), 2);
        assert!(
            !got.iter()
                .any(|p| p.to_string_lossy().ends_with("not-a-model"))
        );
    }

    #[test]
    fn copy_dir_recursive_preserves_file_contents() {
        let src = unique_tmp("copy-src");
        let dst = unique_tmp("copy-dst");
        make_tree(&src, "sub/dir/file.txt", "hello");
        make_tree(&src, "top.txt", "world");
        copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(dst.join("top.txt")).unwrap(), "world");
        assert_eq!(
            fs::read_to_string(dst.join("sub/dir/file.txt")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn migration_report_display_summary() {
        let r = MigrationReport {
            moved: vec![(PathBuf::from("/a"), PathBuf::from("/b"))],
            skipped: vec!["already present: /b/x".to_string()],
            failed: vec![],
        };
        let s = format!("{r}");
        assert!(s.contains("moved=1"));
        assert!(s.contains("skipped=1"));
        assert!(s.contains("failed=0"));
        assert!(s.contains("/a -> /b"));
    }
}

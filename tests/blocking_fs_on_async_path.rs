//! Guard for #60: `std::fs` must not block the async runtime.
//!
//! Migrating the existing `std::fs` call sites to `tokio::fs` turned out to be
//! mostly a non-change, because nearly all of them sit in genuinely synchronous
//! code (Tracker/Tantivy constructors, test fixtures, and the
//! `spawn_blocking` body that #71 introduced for model setup). The two sites
//! that were actually reachable from `async fn` were the `--ephemeral` temp-dir
//! setup in `main.rs`, and those are now `tokio::fs`.
//!
//! That leaves no runtime-blocking filesystem call in production code — but that
//! is a property worth *enforcing*, not one worth rediscovering later. This test
//! fails the build if anyone reintroduces `std::fs` on an async path, which is
//! the failure mode #60 is actually about.
//!
//! Deliberately permissive: it does not flag `std::fs` in plain `fn`s, in
//! `#[cfg(test)]` code, or inside a `spawn_blocking` closure, because none of
//! those block a runtime worker.

use std::path::{Path, PathBuf};

/// Recursively collect every `.rs` file under `dir`, skipping build output.
fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" {
            continue;
        }
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// A tracked scope: the brace depth where it was opened, and whether a body has
/// actually opened yet.
struct Scope {
    depth: i32,
    armed: bool,
}

impl Scope {
    fn new(depth: i32) -> Self {
        Self {
            depth,
            armed: false,
        }
    }

    /// True once the scope's body is definitely closed.
    ///
    /// The two conditions matter separately. `depth < s.depth` catches a scope
    /// that never opened a body at all — a trait's `async fn foo();` signature
    /// would otherwise latch open and mislabel every later `std::fs` call. The
    /// armed case catches a real body closing back to its opening depth, which
    /// is the same depth a bare `#[cfg(test)]` attribute sits at, and is why
    /// `armed` is needed before an equality pop is allowed.
    fn is_closed(&self, depth: i32) -> bool {
        depth < self.depth || (self.armed && depth <= self.depth)
    }
}

/// Report `(line_number, line)` for each `std::fs::` call that sits inside an
/// `async fn` body, is not test code, and is not inside a `spawn_blocking`
/// closure.
///
/// Brace-depth tracking is enough here: `async fn` bodies, `#[cfg(test)]`
/// modules, and `spawn_blocking` closures all delimit themselves with braces we
/// can watch unwind. It is not a Rust parser, and it does not need to be — the
/// failure it must catch (someone reaching for `std::fs` in a handler) always
/// shows up as a call lexically inside an async body.
fn blocking_fs_calls(source: &str) -> Vec<(usize, String)> {
    let mut violations = Vec::new();

    let mut async_fn: Option<Scope> = None;
    let mut test_code: Option<Scope> = None;
    let mut spawn_blocking: Option<Scope> = None;
    let mut depth: i32 = 0;

    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();

        // Record scopes *before* applying this line's braces, so a one-line
        // `async fn foo() { std::fs::write(..) }` is attributed correctly.
        if trimmed.contains("async fn") {
            async_fn = Some(Scope::new(depth));
        }
        if trimmed.starts_with("#[cfg(test)]")
            || trimmed.starts_with("#[test]")
            || trimmed.starts_with("#[tokio::test]")
        {
            test_code = Some(Scope::new(depth));
        }
        if trimmed.contains("spawn_blocking") {
            spawn_blocking = Some(Scope::new(depth));
        }

        let on_async_path = async_fn.is_some() && test_code.is_none() && spawn_blocking.is_none();
        if on_async_path && line.contains("std::fs::") {
            violations.push((idx + 1, line.trim().to_string()));
        }

        depth += line.matches('{').count() as i32 - line.matches('}').count() as i32;

        for scope in [&mut async_fn, &mut test_code, &mut spawn_blocking]
            .into_iter()
            .flatten()
        {
            if depth > scope.depth {
                scope.armed = true;
            }
        }
        for slot in [&mut async_fn, &mut test_code, &mut spawn_blocking] {
            if slot.as_ref().is_some_and(|s| s.is_closed(depth)) {
                *slot = None;
            }
        }
    }

    violations
}

#[test]
fn no_blocking_std_fs_calls_on_async_paths() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    rust_sources(&root.join("crates"), &mut sources);
    rust_sources(&root.join("src"), &mut sources);
    sources.sort();

    assert!(
        !sources.is_empty(),
        "source scan found no .rs files under {} — the guard is not actually \
         scanning anything and would pass vacuously",
        root.display()
    );

    let mut violations = Vec::new();
    for path in &sources {
        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        let relative = path.strip_prefix(root).unwrap_or(path);
        for (line, text) in blocking_fs_calls(&source) {
            violations.push(format!("{}:{line}: {text}", relative.display()));
        }
    }

    assert!(
        violations.is_empty(),
        "found {} `std::fs::` call(s) on an async path, which block a runtime \
         worker for the duration of the I/O:\n\n{}\n\nUse `tokio::fs` instead, or \
         move the work into `spawn_blocking` if it must be synchronous. \
         See #60.",
        violations.len(),
        violations.join("\n")
    );
}

#[test]
fn detector_flags_std_fs_inside_an_async_fn() {
    // The guard is only worth having if it actually fails. Both shapes below
    // must be reported.
    let multiline = "async fn handler() {\n    std::fs::write(\"a\", b\"b\")?;\n}\n";
    assert_eq!(
        blocking_fs_calls(multiline).len(),
        1,
        "missed std::fs inside a multi-line async fn"
    );

    let single_line = "async fn handler() { std::fs::create_dir_all(p)?; }\n";
    assert_eq!(
        blocking_fs_calls(single_line).len(),
        1,
        "missed std::fs inside a one-line async fn"
    );
}

#[test]
fn detector_allows_sync_test_and_spawn_blocking_contexts() {
    // Plain sync fn: no runtime worker to block.
    assert!(blocking_fs_calls("fn build() {\n    std::fs::write(\"a\", b\"b\")?;\n}\n").is_empty());

    // Test module.
    assert!(blocking_fs_calls(
        "#[cfg(test)]\nmod tests {\n    async fn t() { std::fs::write(\"a\", b\"b\").unwrap(); }\n}\n"
    )
    .is_empty());

    // Deliberately-sync work handed to the blocking pool (#71's model setup).
    assert!(blocking_fs_calls(
        "async fn setup() {\n    spawn_blocking(|| {\n        std::fs::create_dir_all(p)?;\n    }).await?;\n}\n"
    )
    .is_empty());
}

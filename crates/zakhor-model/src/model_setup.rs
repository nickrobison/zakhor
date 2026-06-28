//! Model download and setup for the GLiNER-RELEX extraction pipeline.
//!
//! Uses [`hf-hub`] to download model files from HuggingFace Hub on demand.
//! Files are cached in a configurable model directory using hf-hub's
//! content-addressed cache so subsequent starts are instant.

use std::path::{Path, PathBuf};

/// HuggingFace repository that hosts the GLiNER-RELEX ONNX model.
pub const HF_REPO: &str = "nickrobison/gliner-relex-onnx";

/// Files that must be present for the extraction pipeline to function.
#[cfg_attr(not(test), allow(dead_code))]
const REQUIRED_FILES: &[&str] = &["model.onnx", "tokenizer.json"];

/// Paths to the downloaded model and tokenizer files.
#[derive(Clone, Debug)]
pub struct ModelFiles {
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
}

/// Errors that can occur during model setup.
#[derive(Debug)]
pub enum ModelSetupError {
    /// The model directory could not be created or scanned.
    Io(String),
    /// Downloading or resolving a file from HuggingFace failed.
    Download(String),
}

impl std::fmt::Display for ModelSetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelSetupError::Io(msg) => write!(f, "model setup I/O: {msg}"),
            ModelSetupError::Download(msg) => write!(f, "model download: {msg}"),
        }
    }
}

impl std::error::Error for ModelSetupError {}

impl From<std::io::Error> for ModelSetupError {
    fn from(e: std::io::Error) -> Self {
        ModelSetupError::Io(e.to_string())
    }
}

/// Ensure the GLiNER-RELEX model and tokenizer are available on disk.
///
/// `model_dir` is the directory where model files should be stored (it will
/// be created if it does not exist).  Files are resolved through the
/// `hf-hub` content-addressed cache rooted at `model_dir`; if a file is
/// not yet cached it will be downloaded from
/// [`nickrobison/gliner-relex-onnx`](https://huggingface.co/nickrobison/gliner-relex-onnx).
///
/// Returns the absolute paths to `model.onnx` and `tokenizer.json`.
///
/// # Blocking
///
/// This function performs blocking I/O (directory creation, hf-hub cache
/// lookup, and possibly an HTTP download).  Call it from a blocking context
/// or wrap with [`tokio::task::spawn_blocking`].
#[tracing::instrument(skip(model_dir))]
pub fn ensure_model_files(model_dir: &Path) -> Result<ModelFiles, ModelSetupError> {
    std::fs::create_dir_all(model_dir)?;

    let api = hf_hub::api::sync::ApiBuilder::new()
        .with_cache_dir(model_dir.to_path_buf())
        .with_progress(true)
        .build()
        .map_err(|e| ModelSetupError::Download(format!("hf-hub init: {e}")))?;

    let repo = api.model(HF_REPO.to_string());

    let model_path = repo
        .get("model.onnx")
        .map_err(|e| ModelSetupError::Download(format!("model.onnx: {e}")))?;

    let tokenizer_path = repo
        .get("tokenizer.json")
        .map_err(|e| ModelSetupError::Download(format!("tokenizer.json: {e}")))?;

    tracing::info!(
        "GLiNER-RELEX model ready: model={}, tokenizer={}",
        model_path.display(),
        tokenizer_path.display()
    );

    Ok(ModelFiles {
        model_path,
        tokenizer_path,
    })
}

/// Async wrapper around [`ensure_model_files`] that runs the blocking
/// setup on a dedicated thread via [`tokio::task::spawn_blocking`].
#[tracing::instrument(skip(model_dir))]
pub async fn ensure_model_files_async(
    model_dir: PathBuf,
) -> Result<ModelFiles, ModelSetupError> {
    tokio::task::spawn_blocking(move || ensure_model_files(&model_dir))
        .await
        .map_err(|e| ModelSetupError::Io(format!("spawn_blocking join: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_files_debug_and_clone() {
        let files = ModelFiles {
            model_path: PathBuf::from("/m.bin"),
            tokenizer_path: PathBuf::from("/t.json"),
        };
        let cloned = files.clone();
        assert_eq!(cloned.model_path, PathBuf::from("/m.bin"));
        assert_eq!(cloned.tokenizer_path, PathBuf::from("/t.json"));
        let debug = format!("{files:?}");
        assert!(debug.contains("model_path"));
        assert!(debug.contains("tokenizer_path"));
    }

    #[test]
    fn test_model_setup_error_display() {
        let err = ModelSetupError::Io("permission denied".into());
        assert!(
            format!("{err}").contains("model setup I/O: permission denied"),
            "msg: {err}"
        );

        let err = ModelSetupError::Download("network error".into());
        assert!(
            format!("{err}").contains("model download: network error"),
            "msg: {err}"
        );
    }

    #[test]
    fn test_model_setup_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ModelSetupError = io_err.into();
        assert!(format!("{err}").contains("model setup I/O:"));
    }

    #[test]
    fn test_required_files_are_listed() {
        assert!(REQUIRED_FILES.contains(&"model.onnx"));
        assert!(REQUIRED_FILES.contains(&"tokenizer.json"));
        assert_eq!(REQUIRED_FILES.len(), 2);
    }

    #[test]
    fn test_hf_repo_is_correct() {
        assert_eq!(HF_REPO, "nickrobison/gliner-relex-onnx");
    }
}

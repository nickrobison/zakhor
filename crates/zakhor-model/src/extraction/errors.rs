use crate::model_setup;

/// Errors produced by the extraction pipeline.
///
/// Each variant carries:
/// - A human-readable message (for Display / user-facing output).
/// - A `stage_name` identifying which pipeline stage produced the error.
/// - An optional `#[source]` wrapping the underlying error when applicable.
#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    /// Failed to load the ONNX model or tokenizer.
    #[error("model load: {0}")]
    ModelLoad(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// ONNX inference or pipeline processing failed.
    #[error("inference: {0}")]
    Inference(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// Mapping extracted values back to Zakhor types failed.
    #[error("mapping: {0}")]
    Mapping(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),

    /// The async blocking task itself panicked or was cancelled.
    #[error("task join: {0}")]
    TaskJoin(
        String,
        &'static str,
        #[source] Option<Box<dyn std::error::Error + Send + Sync>>,
    ),
}

impl From<model_setup::ModelSetupError> for ExtractionError {
    fn from(e: model_setup::ModelSetupError) -> Self {
        ExtractionError::ModelLoad(e.to_string(), "model_setup", Some(Box::new(e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_error_display() {
        let err = ExtractionError::ModelLoad("file not found".into(), "model_load", None);
        let msg = format!("{}", err);
        assert!(msg.contains("model load: file not found"), "msg: {}", msg);

        let err = ExtractionError::Inference("timeout".into(), "inference", None);
        assert!(format!("{}", err).contains("inference: timeout"));

        let err = ExtractionError::Mapping("bad label".into(), "mapping", None);
        assert!(format!("{}", err).contains("mapping: bad label"));

        let err = ExtractionError::TaskJoin("cancelled".into(), "task_join", None);
        assert!(format!("{}", err).contains("task join: cancelled"));
    }

    #[test]
    fn test_extraction_error_impl_error() {
        let err = ExtractionError::ModelLoad("fail".into(), "model_load", None);
        let err_ref: &dyn std::error::Error = &err;
        assert!(err_ref.to_string().contains("model load: fail"));
    }

    #[test]
    fn test_extraction_error_source_chain() {
        let inner = std::io::Error::other("io failure");
        let err = ExtractionError::ModelLoad(
            "ONNX load failed".into(),
            "model_load",
            Some(Box::new(inner)),
        );
        let source = std::error::Error::source(&err);
        assert!(source.is_some(), "should have a source error");
        let source_msg = source.unwrap().to_string();
        assert!(
            source_msg.contains("io failure"),
            "source should contain inner error message: {source_msg}"
        );
    }

    #[test]
    fn test_extraction_error_source_chain_inference() {
        let inner =
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let err = ExtractionError::Inference("timeout".into(), "inference", Some(Box::new(inner)));
        let source = std::error::Error::source(&err);
        assert!(source.is_some());
        assert!(source.unwrap().to_string().contains("connection refused"));
    }

    #[test]
    fn test_extraction_error_source_none() {
        let err = ExtractionError::Mapping("bad".into(), "mapping", None);
        assert!(
            std::error::Error::source(&err).is_none(),
            "variant without source should return None"
        );
    }

    #[test]
    fn test_extraction_error_stage_names() {
        let mut stages: Vec<(&str, &str)> = Vec::new();

        if let ExtractionError::ModelLoad(_, stage, _) =
            ExtractionError::ModelLoad("".into(), "model_load", None)
        {
            stages.push(("ModelLoad", stage));
        }
        if let ExtractionError::Inference(_, stage, _) =
            ExtractionError::Inference("".into(), "inference", None)
        {
            stages.push(("Inference", stage));
        }
        if let ExtractionError::Mapping(_, stage, _) =
            ExtractionError::Mapping("".into(), "mapping", None)
        {
            stages.push(("Mapping", stage));
        }
        if let ExtractionError::TaskJoin(_, stage, _) =
            ExtractionError::TaskJoin("".into(), "task_join", None)
        {
            stages.push(("TaskJoin", stage));
        }

        assert_eq!(stages.len(), 4, "all 4 variants should be destructured");
        assert_eq!(stages[0], ("ModelLoad", "model_load"));
        assert_eq!(stages[1], ("Inference", "inference"));
        assert_eq!(stages[2], ("Mapping", "mapping"));
        assert_eq!(stages[3], ("TaskJoin", "task_join"));
    }

    #[test]
    fn test_extraction_error_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ExtractionError>();
    }

    #[test]
    fn test_extraction_error_from_model_setup_error() {
        let setup_err = crate::model_setup::ModelSetupError::Io("cannot read file".into());
        let extract_err: ExtractionError = setup_err.into();
        let err_str = extract_err.to_string();
        assert!(
            err_str.contains("model load"),
            "from ModelSetupError should produce ModelLoad: {err_str}"
        );
    }
}

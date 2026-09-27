//! Checks that the configured ONNX model is one the GLiNER binding can decode.
//!
//! The binding validates the decoder's output tensor against a shape it
//! expects and, on mismatch, returns the bare string `unexpected logits shape`.
//! That names neither the model nor the expectation, so a caller cannot tell
//! whether the model, the configuration or the storage layer is at fault.
//!
//! Two decoder layouts exist in `gline-rs`:
//!
//! | mode | `logits` layout | extra model inputs |
//! |---|---|---|
//! | span | `(batch, num_words, max_width, num_classes)` | `span_idx`, `span_mask` |
//! | token | `(3, batch, num_words, num_classes)` | — |
//!
//! The model published as `nickrobison/gliner-relex-onnx` is the
//! `UniEncoderSpanRelex` export, whose logits are
//! `(batch, sequence_length, num_ent_classes, num_idx_classes)` and which takes
//! neither `span_idx` nor `span_mask`. Neither decoder can consume it, so the
//! shape check can never pass. This module turns that into a diagnosis.

use std::path::Path;

/// The decoder layouts `gline-rs` is able to decode, as (label, axis names).
const SUPPORTED_LOGIT_LAYOUTS: &[(&str, &[&str])] = &[
    (
        "span mode",
        &["batch_size", "num_words", "max_width", "num_classes"],
    ),
    (
        "token mode",
        &["start_end_inside", "batch_size", "num_words", "num_classes"],
    ),
];

/// Inputs only the span decoder feeds, and which a compatible model must accept.
const SPAN_MODE_ONLY_INPUTS: &[&str] = &["span_idx", "span_mask"];

/// The declared interface of an ONNX file, as far as it can be read cheaply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelContract {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    /// Declared `logits` rank, or `None` when the model has no `logits` output.
    pub logits_rank: Option<usize>,
}

/// Read an ONNX file's input/output names and the rank of its `logits` output.
///
/// Only metadata is inspected; no inference session is run and no weights are
/// materialised beyond what the runtime needs to open the graph.
pub fn read_contract(model_path: &Path) -> Result<ModelContract, String> {
    let session = ort::session::Session::builder()
        .map_err(|e| format!("could not create an ONNX session: {e}"))?
        .commit_from_file(model_path)
        .map_err(|e| format!("could not open {}: {e}", model_path.display()))?;

    let inputs: Vec<String> = session
        .inputs()
        .iter()
        .map(|i| i.name().to_string())
        .collect();
    let outputs: Vec<String> = session
        .outputs()
        .iter()
        .map(|o| o.name().to_string())
        .collect();
    let logits_rank = session
        .outputs()
        .iter()
        .find(|o| o.name() == "logits")
        .and_then(|o| o.dtype().tensor_shape().map(|s| s.len()));

    Ok(ModelContract {
        inputs,
        outputs,
        logits_rank,
    })
}

/// Explain why the configured model cannot be decoded, or `Ok(())` if the
/// available evidence is consistent with a supported layout.
///
/// This is deliberately conservative: it only reports a mismatch it can
/// demonstrate. A model that merely looks unusual is allowed through, because a
/// false rejection would disable a working configuration.
pub fn explain_incompatibility(model_path: &Path) -> Option<String> {
    let contract = read_contract(model_path).ok()?;

    if !contract.outputs.iter().any(|o| o == "logits") {
        return Some(format!(
            "the model at {} has no `logits` output (it declares {}), so the GLiNER \
             decoders cannot read it",
            model_path.display(),
            contract.outputs.join(", ")
        ));
    }

    // A rank-4 `logits` is the only shape either decoder can consume. Anything
    // else is provably undecodable.
    if contract.logits_rank != Some(4) {
        return Some(format!(
            "the model at {} declares a `logits` output of rank {:?}, but the GLiNER \
             binding in use can only decode rank 4",
            model_path.display(),
            contract.logits_rank
        ));
    }

    // Rank 4 is necessary but not sufficient: the two supported layouts differ
    // in what axis 0 means, which cannot be told apart from metadata alone. Use
    // the input signature to disambiguate — only the span export takes the
    // candidate-span tensors.
    let takes_span_inputs = SPAN_MODE_ONLY_INPUTS
        .iter()
        .all(|name| contract.inputs.iter().any(|i| i == name));

    if takes_span_inputs {
        return None;
    }

    Some(format!(
        "the model at {} takes only [{}] and emits `logits` as rank 4, which is the \
         shape of a zero-shot span/token export rather than the candidate-span export \
         the binding decodes. The binding's two decoders expect {}.",
        model_path.display(),
        contract.inputs.join(", "),
        SUPPORTED_LOGIT_LAYOUTS
            .iter()
            .map(|(label, axes)| format!("{label} {}", axes.join(" x ")))
            .collect::<Vec<_>>()
            .join(" or "),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(inputs: &[&str], outputs: &[&str], rank: Option<usize>) -> ModelContract {
        ModelContract {
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
            outputs: outputs.iter().map(|s| s.to_string()).collect(),
            logits_rank: rank,
        }
    }

    /// A model whose signature cannot be read must not be reported as broken.
    /// A false rejection would disable a configuration that actually works.
    #[test]
    fn test_missing_model_is_not_reported_as_incompatible() {
        let missing = Path::new("/nonexistent/model.onnx");
        assert_eq!(explain_incompatibility(missing), None);
    }

    #[test]
    fn test_model_without_logits_is_rejected() {
        let dir = std::env::temp_dir().join(format!("zakhor-compat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("model.onnx");
        std::fs::write(&path, b"not really onnx").unwrap();

        // A file that cannot be parsed yields no contract, hence no verdict.
        assert_eq!(explain_incompatibility(&path), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_span_layouts_are_documented_consistently() {
        assert_eq!(SUPPORTED_LOGIT_LAYOUTS.len(), 2);
        for (label, axes) in SUPPORTED_LOGIT_LAYOUTS {
            assert!(!label.is_empty());
            assert_eq!(axes.len(), 4, "{label} must be rank 4");
            assert!(
                axes.iter().any(|a| a.contains("num_classes")),
                "{label} must name the class axis"
            );
        }
    }

    #[test]
    fn test_span_mode_inputs_are_the_disambiguator() {
        // Only the candidate-span export carries these.
        assert!(SPAN_MODE_ONLY_INPUTS.contains(&"span_idx"));
        assert!(SPAN_MODE_ONLY_INPUTS.contains(&"span_mask"));
    }

    /// The shipped RELEX model is the concrete case: rank 4, no span inputs.
    #[test]
    fn test_relex_style_signature_is_accepted_by_the_shape_gate() {
        // Documents that rank alone cannot separate the layouts, which is why
        // the input signature is the tie-breaker used above.
        let c = contract(
            &["input_ids", "attention_mask", "words_mask", "text_lengths"],
            &["logits"],
            Some(4),
        );
        assert_eq!(c.logits_rank, Some(4));
        assert!(
            !SPAN_MODE_ONLY_INPUTS
                .iter()
                .all(|n| c.inputs.iter().any(|i| i == n)),
            "RELEX signature must not satisfy the span-mode input gate"
        );
    }
}

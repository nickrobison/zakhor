//! Checks that the configured ONNX model is one the GLiNER binding can decode.
//!
//! The binding validates the decoder's output tensor against a shape it
//! expects and, on mismatch, returns the bare string `unexpected logits shape`.
//! That names neither the model nor the expectation, so a caller cannot tell
//! whether the model, the configuration or the storage layer is at fault.
//!
//! Decoder layouts exist in `gline-rs`:
//!
//! | mode | `logits` layout | extra model inputs | relation outputs |
//! |---|---|---|---|
//! | span | `(batch, num_words, max_width, num_classes)` | `span_idx`, `span_mask` | no |
//! | token + relex (legacy) | `(batch, sequence_length, num_ent_classes, num_idx_classes)` | token-style only | `rel_logits`, `rel_idx`, `rel_mask` — structurally constant |
//! | token + relex (static) | `(batch, sequence_length, num_ent_classes, num_idx_classes)` | token-style + `span_idx`, `span_mask` | `rel_logits`, `rel_idx`, `rel_mask` — data-driven |
//!
//! The model published as `nickrobison/gliner-relex-onnx` is the
//! `UniEncoderSpanRelex` export: token-style inputs
//! (`input_ids`, `attention_mask`, `words_mask`, `text_lengths`), rank-4
//! `logits` of the token+relex shape, and native relation outputs. In that
//! legacy export the relation outputs are useless: the pair-building step is
//! data-dependent, so the ONNX tracer froze `rel_idx` (all `-1`) and
//! `rel_mask` (all `false`) into constants regardless of the input. Entities
//! still decode correctly, so the legacy export runs in a degraded mode.
//! The static-capacity export of the same weights adds `span_idx`/`span_mask`
//! inputs and restores working relations; the runner fills the slots from the
//! decoded entities in a second pass. This
//! module turns mismatches into actionable diagnoses.

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

/// Input names shared by every token-mode export (span and relex variants).
const TOKEN_MODE_INPUTS: &[&str] = &["input_ids", "attention_mask"];

/// The declared interface of an ONNX file, as far as it can be read cheaply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelContract {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    /// Declared `logits` rank, or `None` when the model has no `logits` output.
    pub logits_rank: Option<usize>,
    /// Whether the model also emits relation outputs (`rel_logits`, `rel_idx`,
    /// `rel_mask`), i.e. whether it is a relex export.
    pub has_relations: bool,
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

    let has_relations = session
        .outputs()
        .iter()
        .any(|o| o.name() == "rel_logits" || o.name() == "rel_idx" || o.name() == "rel_mask");

    Ok(ModelContract {
        inputs,
        outputs,
        logits_rank,
        has_relations,
    })
}

/// Read the contract from an already-loaded model instead of opening the ONNX
/// file a second time — a large graph would otherwise be parsed twice on the
/// first extraction call.
pub fn read_contract_from(model: &orp::model::Model) -> ModelContract {
    let outputs = model.output_names();
    ModelContract {
        inputs: model.input_names(),
        logits_rank: model.output_rank("logits"),
        has_relations: outputs
            .iter()
            .any(|o| o == "rel_logits" || o == "rel_idx" || o == "rel_mask"),
        outputs,
    }
}

/// How much of the extraction feature set the configured model can deliver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationSupport {
    /// The graph computes relations from caller-provided entity slots.
    Native,
    /// Entities work, but the graph's relation outputs are structurally
    /// constant (the legacy export's baked `rel_idx`/`rel_mask`), so relation
    /// extraction always comes back empty.
    Degraded,
    /// The graph exposes no relation outputs at all.
    None,
}

/// Classify the relation capabilities of the configured model.
///
/// `None` when the model's interface cannot be read at all.
pub fn relation_support(model_path: &Path) -> Option<RelationSupport> {
    let contract = read_contract(model_path).ok()?;
    Some(relation_support_for(&contract))
}

/// Classify the relation capabilities of an already-loaded model.
pub fn relation_support_of(model: &orp::model::Model) -> RelationSupport {
    relation_support_for(&read_contract_from(model))
}

fn relation_support_for(contract: &ModelContract) -> RelationSupport {
    if !contract.has_relations {
        return RelationSupport::None;
    }
    let takes_span_inputs = SPAN_MODE_ONLY_INPUTS
        .iter()
        .all(|name| contract.inputs.iter().any(|i| i == name));
    if takes_span_inputs {
        RelationSupport::Native
    } else {
        RelationSupport::Degraded
    }
}

/// Decide whether a declared model interface is one the binding can decode.
///
/// Returns the actionable reason the model cannot be used, or `None` when the
/// available evidence is consistent with a supported layout.
fn verdict(contract: &ModelContract) -> Option<String> {
    if !contract.outputs.iter().any(|o| o == "logits") {
        return Some(format!(
            "the model declares no `logits` output (it declares {}), so the GLiNER \
             decoders cannot read it",
            contract.outputs.join(", ")
        ));
    }

    // A rank-4 `logits` is the only shape either decoder can consume. Anything
    // else is provably undecodable.
    if contract.logits_rank != Some(4) {
        return Some(format!(
            "the model declares a `logits` output of rank {:?}, but the GLiNER \
             binding in use can only decode rank 4",
            contract.logits_rank
        ));
    }

    // Rank 4 is necessary but not sufficient: the supported layouts differ
    // in what axis 0 means, which cannot be told apart from metadata alone. Use
    // the input signature to disambiguate.
    let takes_span_inputs = SPAN_MODE_ONLY_INPUTS
        .iter()
        .all(|name| contract.inputs.iter().any(|i| i == name));

    if takes_span_inputs {
        // Candidate-span export (plain, or static-capacity relex when relation
        // outputs are present too): both are supported.
        return None;
    }

    // Token-mode export that also emits native relation outputs is the
    // `UniEncoderSpanRelex` (relex) export the fork decodes through the token
    // RELEX pipeline. Its rank-4 `logits` layout is exactly what that decoder
    // expects, so it is a supported configuration — with the caveat that the
    // legacy export's relation outputs are structurally constant
    // (see `relation_support`).
    let takes_token_inputs = TOKEN_MODE_INPUTS
        .iter()
        .all(|name| contract.inputs.iter().any(|i| i == name));
    if contract.has_relations && takes_token_inputs {
        return None;
    }

    let expected = SUPPORTED_LOGIT_LAYOUTS
        .iter()
        .map(|(label, axes)| format!("{label} {}", axes.join(" x ")))
        .collect::<Vec<_>>()
        .join(" or ");
    let relation_note = if contract.has_relations {
        " plus relation outputs"
    } else {
        ""
    };
    Some(format!(
        "the model takes only [{}] and emits `logits` as rank 4, which is the \
         shape of a token-mode export the binding does not decode{relation_note}. \
         The binding's decoders expect {expected}.",
        contract.inputs.join(", "),
    ))
}

/// Explain why the configured model cannot be decoded, or `None` if the
/// available evidence is consistent with a supported layout.
///
/// This is deliberately conservative: it only reports a mismatch it can
/// demonstrate. A model that merely looks unusual is allowed through, because a
/// false rejection would disable a working configuration.
pub fn explain_incompatibility(model_path: &Path) -> Option<String> {
    let contract = read_contract(model_path).ok()?;
    verdict(&contract)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(
        inputs: &[&str],
        outputs: &[&str],
        rank: Option<usize>,
        has_relations: bool,
    ) -> ModelContract {
        ModelContract {
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
            outputs: outputs.iter().map(|s| s.to_string()).collect(),
            logits_rank: rank,
            has_relations,
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

    /// The shipped RELEX model is the concrete case: rank 4, token-style inputs,
    /// native relation outputs, no candidate-span inputs — and it must be
    /// accepted now that the fork decodes the token RELEX layout.
    #[test]
    fn test_relex_style_signature_is_accepted_by_the_shape_gate() {
        // Documents that rank alone cannot separate the layouts, which is why
        // the input signature is the tie-breaker used above.
        let c = contract(
            &["input_ids", "attention_mask", "words_mask", "text_lengths"],
            &["logits", "rel_logits", "rel_idx", "rel_mask"],
            Some(4),
            true,
        );
        assert_eq!(c.logits_rank, Some(4));
        assert!(c.has_relations);
        assert!(
            !SPAN_MODE_ONLY_INPUTS
                .iter()
                .all(|n| c.inputs.iter().any(|i| i == n)),
            "RELEX signature must not satisfy the span-mode input gate"
        );
        assert!(
            TOKEN_MODE_INPUTS
                .iter()
                .all(|n| c.inputs.iter().any(|i| i == n)),
            "the RELEX token export must feed the token-mode decoder"
        );

        // The fork's token RELEX pipeline consumes this layout, so a model
        // declaring it is not reported as incompatible.
        assert_eq!(verdict(&c), None);
    }

    /// The candidate-span export stays supported through the span decoder.
    #[test]
    fn test_span_signature_is_accepted() {
        let c = contract(
            &[
                "input_ids",
                "attention_mask",
                "words_mask",
                "text_lengths",
                "span_idx",
                "span_mask",
            ],
            &["logits"],
            Some(4),
            false,
        );
        assert_eq!(c.logits_rank, Some(4));
        assert!(!c.has_relations);
        assert!(SPAN_MODE_ONLY_INPUTS
            .iter()
            .all(|n| c.inputs.iter().any(|i| i == n)));
        assert_eq!(verdict(&c), None);
    }

    /// The legacy relex export keeps entities but cannot deliver relations:
    /// its relation outputs are structurally constant, so it must be
    /// classified degraded rather than native.
    #[test]
    fn test_legacy_relex_signature_is_degraded_for_relations() {
        let c = contract(
            &["input_ids", "attention_mask", "words_mask", "text_lengths"],
            &["logits", "rel_logits", "rel_idx", "rel_mask"],
            Some(4),
            true,
        );
        assert_eq!(verdict(&c), None);
        assert_eq!(
            relation_support_for(&c),
            RelationSupport::Degraded,
            "the legacy export's baked relation outputs must not be reported as native"
        );
    }

    /// The static-capacity export (token inputs + slot inputs + relation
    /// outputs) restores native relations.
    #[test]
    fn test_static_relex_signature_is_native_for_relations() {
        let c = contract(
            &[
                "input_ids",
                "attention_mask",
                "words_mask",
                "text_lengths",
                "span_idx",
                "span_mask",
            ],
            &["logits", "rel_logits", "rel_idx", "rel_mask"],
            Some(4),
            true,
        );
        assert_eq!(verdict(&c), None);
        assert_eq!(relation_support_for(&c), RelationSupport::Native);
    }

    /// A token-mode export without relation outputs is still not something the
    /// binding decodes yet, and must stay reported as such.
    #[test]
    fn test_token_only_signature_without_relations_is_rejected() {
        let c = contract(
            &["input_ids", "attention_mask", "words_mask", "text_lengths"],
            &["logits"],
            Some(4),
            false,
        );
        assert!(!c.has_relations);
        assert!(!SPAN_MODE_ONLY_INPUTS
            .iter()
            .all(|n| c.inputs.iter().any(|i| i == n)));
        assert!(
            verdict(&c).is_some(),
            "token-only export without relations must be rejected"
        );
    }
}

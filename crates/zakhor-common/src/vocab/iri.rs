use iref::Iri;
use static_iref::iri;

// --- Class IRIs ---

pub fn entity_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Entity")
}

pub fn decision_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Decision")
}

pub fn project_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Project")
}

pub fn repository_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Repository")
}

pub fn issue_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Issue")
}

pub fn constraint_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Constraint")
}

pub fn observation_iri() -> &'static Iri {
    iri!("http://zakhor/ns/Observation")
}

pub fn tool_call_iri() -> &'static Iri {
    iri!("http://zakhor/ns/ToolCall")
}

pub fn code_container_iri() -> &'static Iri {
    iri!("http://zakhor/ns/CodeContainer")
}

pub fn code_symbol_iri() -> &'static Iri {
    iri!("http://zakhor/ns/CodeSymbol")
}

// --- Predicate IRIs ---

pub fn has_entity_iri() -> &'static Iri {
    iri!("http://zakhor/ns/hasEntity")
}

pub fn has_relation_iri() -> &'static Iri {
    iri!("http://zakhor/ns/hasRelation")
}

/// IRI of the ontology property that persists an extracted relation labelled
/// `label`.
///
/// Extraction maps a zero-shot relation label to a property in the `relation`
/// namespace. The store ontology (`99-zakhor.ontology`, see
/// `zakhor_storage::schema::ontology_file_content`) declares exactly the
/// properties for the default relation labels, because Tracker rejects
/// `INSERT DATA` triples whose predicate is not a declared property. A label
/// outside that set therefore has no property: the extraction boundary drops
/// such relations instead of failing the whole observation persist.
pub fn relation_predicate_iri(label: &str) -> String {
    format!("http://zakhor/ns/relation/{label}")
}

/// IRI for an extracted entity with surface form `label`.
///
/// The graph convention is `http://zakhor/ns/entity/<name>` (see
/// `is_memory_node` consumers); the extracted span text is the name. Encoding
/// by label rather than minting a per-mention UUID means the same surface form
/// resolves to one node across observations, so edges accumulate on it instead
/// of fragmenting per mention.
///
/// The label is percent-encoded into a single path segment: bytes that are
/// illegal in an IRI path (space, quotes, `%`, `/`, …) are escaped, while
/// non-ASCII text passes through unescaped — RDF IRIs (RFC 3987) allow it, and
/// `nie`-style labels stay readable.
pub fn entity_mention_iri(label: &str) -> String {
    let mut encoded = String::with_capacity(label.len());
    for ch in label.chars() {
        match ch {
            // `ipchar` minus `/` (a label stays one path segment).
            'A'..='Z' | 'a'..='z' | '0'..='9' => encoded.push(ch),
            '-' | '.' | '_' | '~' | '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';'
            | '=' | ':' | '@' => encoded.push(ch),
            // Non-ASCII is legal in an IRI (RFC 3987) and keeps labels readable.
            '\u{80}'..='\u{D7FF}' | '\u{E000}'..='\u{10FFFF}' => encoded.push(ch),
            // Everything else (space, quotes, `%`, `/`, `<`, `>`, `#`, `?`,
            // control characters) is percent-encoded.
            _ => {
                let mut buf = [0u8; 4];
                for byte in ch.encode_utf8(&mut buf).as_bytes() {
                    encoded.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    format!("http://zakhor/ns/entity/{encoded}")
}

pub fn provenance_graph_iri() -> &'static Iri {
    iri!("http://zakhor/ns/provenanceGraph")
}

pub fn decision_context_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionContext")
}

pub fn decision_outcome_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionOutcome")
}

pub fn decision_alternative_iri() -> &'static Iri {
    iri!("http://zakhor/ns/alternative")
}

pub fn decision_rationale_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionRationale")
}

pub fn decision_status_iri() -> &'static Iri {
    iri!("http://zakhor/ns/decisionStatus")
}

pub fn conflicts_with_iri() -> &'static Iri {
    iri!("http://zakhor/ns/conflictsWith")
}

pub fn depends_on_iri() -> &'static Iri {
    iri!("http://zakhor/ns/dependsOn")
}

pub fn supersedes_iri() -> &'static Iri {
    iri!("http://zakhor/ns/supersedes")
}

pub fn evidence_for_iri() -> &'static Iri {
    iri!("http://zakhor/ns/evidenceFor")
}

pub fn belongs_to_project_iri() -> &'static Iri {
    iri!("http://zakhor/ns/belongsToProject")
}

pub fn belongs_to_repository_iri() -> &'static Iri {
    iri!("http://zakhor/ns/belongsToRepository")
}

pub fn code_location_iri() -> &'static Iri {
    iri!("http://zakhor/ns/codeLocation")
}

// --- Observation properties ---

pub fn observation_content_iri() -> &'static Iri {
    iri!("http://zakhor/ns/observationContent")
}

pub fn observation_created_at_iri() -> &'static Iri {
    iri!("http://zakhor/ns/observationCreatedAt")
}

// --- Tool-call properties ---

pub fn tool_name_iri() -> &'static Iri {
    iri!("http://zakhor/ns/toolName")
}

pub fn tool_arguments_iri() -> &'static Iri {
    iri!("http://zakhor/ns/toolArguments")
}

pub fn session_id_iri() -> &'static Iri {
    iri!("http://zakhor/ns/sessionId")
}

pub fn timestamp_iri() -> &'static Iri {
    iri!("http://zakhor/ns/timestamp")
}

// --- Ranking ---

pub fn graph_importance_iri() -> &'static Iri {
    iri!("http://zakhor/ns/graphImportance")
}

pub fn provenance_quality_iri() -> &'static Iri {
    iri!("http://zakhor/ns/provenanceQuality")
}

pub fn tombstone_iri() -> &'static Iri {
    iri!("http://zakhor/ns/tombstone")
}

pub fn code_language_iri() -> &'static Iri {
    iri!("http://zakhor/ns/codeLanguage")
}

pub fn code_symbol_kind_iri() -> &'static Iri {
    iri!("http://zakhor/ns/codeSymbolKind")
}

pub fn code_line_start_iri() -> &'static Iri {
    iri!("http://zakhor/ns/codeLineStart")
}

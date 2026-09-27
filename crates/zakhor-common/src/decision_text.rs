//! Projections of stored records into plain text for the search indexes.
//!
//! Records that carry no `nie:plainTextContent` — decisions, for example — are
//! invisible to the index rebuilds unless their fields are explicitly composed
//! into a single blob. Both the write path and the rebuild path must agree on
//! that composition, so it lives here rather than in either consumer.

fn non_blank(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Compose a decision's narrative fields into one indexable blob.
///
/// Context, decision, rationale, and alternatives are all included because any
/// of them may be what an agent searches for when recalling the decision later.
/// Blank fields and an empty alternative list are omitted rather than emitting
/// dangling labels.
pub fn decision_index_text(
    context: &str,
    outcome: &str,
    rationale: &str,
    alternatives: &[String],
) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(context) = non_blank(context) {
        parts.push(format!("Context: {context}"));
    }
    if let Some(outcome) = non_blank(outcome) {
        parts.push(format!("Decision: {outcome}"));
    }
    if let Some(rationale) = non_blank(rationale) {
        parts.push(format!("Rationale: {rationale}"));
    }

    let alternatives: Vec<&str> = alternatives.iter().filter_map(|a| non_blank(a)).collect();
    if !alternatives.is_empty() {
        parts.push(format!("Alternatives: {}", alternatives.join(", ")));
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_every_non_blank_field() {
        let alts = vec!["Kafka".to_string(), "RabbitMQ".to_string()];
        let text = decision_index_text("choose a bus", "Adopt NATS", "simpler", &alts);
        assert!(text.contains("choose a bus"));
        assert!(text.contains("Adopt NATS"));
        assert!(text.contains("simpler"));
        assert!(text.contains("Kafka"));
        assert!(text.contains("RabbitMQ"));
    }

    #[test]
    fn omits_empty_alternatives_section() {
        let text = decision_index_text("ctx", "out", "why", &[]);
        assert!(!text.contains("Alternatives"));
    }

    #[test]
    fn omits_blank_fields_and_trims() {
        let text = decision_index_text("   ", "  Adopt NATS  ", "", &[]);
        assert!(text.contains("Adopt NATS"));
        assert!(!text.contains("Context:"));
        assert!(!text.contains("Rationale:"));
    }

    #[test]
    fn all_blank_yields_empty_string() {
        assert!(decision_index_text("", "", "", &[]).is_empty());
    }

    #[test]
    fn blank_alternatives_are_dropped() {
        let alts = vec!["Kafka".to_string(), "   ".to_string()];
        let text = decision_index_text("c", "o", "r", &alts);
        assert!(text.contains("Alternatives: Kafka"));
        assert!(!text.contains("Alternatives: Kafka,   "));
    }
}

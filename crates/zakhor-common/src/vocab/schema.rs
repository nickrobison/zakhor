//! Typed vocabulary schema — the single source of truth for Zakhor's ontology.
//!
//! The IRI factories in [`super::iri`] mint class/property IRIs, and the const
//! tables below (`CLASSES`, `PROPERTIES`) are the authoritative shape of those
//! terms (name, domain, range). The storage layer renders
//! [`crate::storage::schema::ontology_file_content`] from these tables into the
//! Turtle/N3 ontology written into the store, so the store ontology can never
//! drift from the declared vocabulary again.

/// Namespace every Zakhor class and property lives under.
pub const ZAKHOR_NS: &str = "http://zakhor/ns/";

/// A class declared in the ontology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassDef {
    /// Local name under [`ZAKHOR_NS`], e.g. `Entity`.
    pub name: &'static str,
    /// Human-readable label, e.g. `Entity`.
    pub label: &'static str,
}

impl ClassDef {
    /// Full IRI of the class.
    pub fn iri(&self) -> String {
        format!("{ZAKHOR_NS}{}", self.name)
    }
}

/// A property declared in the ontology.
///
/// `domain`/`range` use Turtle-style tokens (`rdfs:Resource`, `zakhor:Decision`,
/// `xsd:string`, `xsd:integer`, …); `rdfs:Resource` means the predicate can
/// link an instance IRI or an embedded literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyDef {
    /// Local name under [`ZAKHOR_NS`], e.g. `hasEntity`.
    pub name: &'static str,
    /// Human-readable label, e.g. `hasEntity`.
    pub label: &'static str,
    /// Domain token, e.g. `rdfs:Resource` or `zakhor:Decision`.
    pub domain: &'static str,
    /// Range token, e.g. `xsd:string` or `zakhor:Entity`.
    pub range: &'static str,
}

impl PropertyDef {
    /// Full IRI of the property.
    pub fn iri(&self) -> String {
        format!("{ZAKHOR_NS}{}", self.name)
    }
}

/// Every class the ontology declares, in emission order.
pub const CLASSES: &[ClassDef] = &[
    ClassDef {
        name: "Entity",
        label: "Entity",
    },
    ClassDef {
        name: "Decision",
        label: "Decision",
    },
    ClassDef {
        name: "Project",
        label: "Project",
    },
    ClassDef {
        name: "Repository",
        label: "Repository",
    },
    ClassDef {
        name: "Issue",
        label: "Issue",
    },
    ClassDef {
        name: "Constraint",
        label: "Constraint",
    },
    ClassDef {
        name: "Observation",
        label: "Observation",
    },
    ClassDef {
        name: "ToolCall",
        label: "ToolCall",
    },
    ClassDef {
        name: "CodeContainer",
        label: "CodeContainer",
    },
    ClassDef {
        name: "CodeSymbol",
        label: "CodeSymbol",
    },
];

/// Every schema property declared in the ontology, in emission order.
pub const PROPERTIES: &[PropertyDef] = &[
    PropertyDef {
        name: "hasEntity",
        label: "hasEntity",
        domain: "rdfs:Resource",
        range: "zakhor:Entity",
    },
    PropertyDef {
        name: "hasRelation",
        label: "hasRelation",
        domain: "rdfs:Resource",
        range: "rdfs:Resource",
    },
    PropertyDef {
        name: "provenanceGraph",
        label: "provenanceGraph",
        domain: "rdfs:Resource",
        range: "rdfs:Resource",
    },
    PropertyDef {
        name: "decisionContext",
        label: "decisionContext",
        domain: "zakhor:Decision",
        range: "xsd:string",
    },
    PropertyDef {
        name: "decisionOutcome",
        label: "decisionOutcome",
        domain: "zakhor:Decision",
        range: "xsd:string",
    },
    PropertyDef {
        name: "alternative",
        label: "alternative",
        domain: "zakhor:Decision",
        range: "xsd:string",
    },
    PropertyDef {
        name: "decisionRationale",
        label: "decisionRationale",
        domain: "zakhor:Decision",
        range: "xsd:string",
    },
    PropertyDef {
        name: "decisionStatus",
        label: "decisionStatus",
        domain: "zakhor:Decision",
        range: "xsd:string",
    },
    PropertyDef {
        name: "conflictsWith",
        label: "conflictsWith",
        domain: "zakhor:Decision",
        range: "rdfs:Resource",
    },
    PropertyDef {
        name: "dependsOn",
        label: "dependsOn",
        domain: "zakhor:Decision",
        range: "rdfs:Resource",
    },
    PropertyDef {
        name: "supersedes",
        label: "supersedes",
        domain: "zakhor:Decision",
        range: "rdfs:Resource",
    },
    PropertyDef {
        name: "belongsToProject",
        label: "belongsToProject",
        domain: "rdfs:Resource",
        range: "zakhor:Project",
    },
    PropertyDef {
        name: "belongsToRepository",
        label: "belongsToRepository",
        domain: "rdfs:Resource",
        range: "zakhor:Repository",
    },
    PropertyDef {
        name: "evidenceFor",
        label: "evidenceFor",
        domain: "rdfs:Resource",
        range: "rdfs:Resource",
    },
    PropertyDef {
        name: "toolName",
        label: "toolName",
        domain: "zakhor:ToolCall",
        range: "xsd:string",
    },
    PropertyDef {
        name: "toolArguments",
        label: "toolArguments",
        domain: "zakhor:ToolCall",
        range: "xsd:string",
    },
    PropertyDef {
        name: "sessionId",
        label: "sessionId",
        domain: "zakhor:ToolCall",
        range: "xsd:string",
    },
    PropertyDef {
        name: "timestamp",
        label: "timestamp",
        domain: "zakhor:ToolCall",
        range: "xsd:integer",
    },
    PropertyDef {
        name: "graphImportance",
        label: "graphImportance",
        domain: "rdfs:Resource",
        range: "xsd:double",
    },
    PropertyDef {
        name: "provenanceQuality",
        label: "provenanceQuality",
        domain: "rdfs:Resource",
        range: "xsd:double",
    },
    PropertyDef {
        name: "codeLocation",
        label: "codeLocation",
        domain: "rdfs:Resource",
        range: "rdfs:Resource",
    },
    PropertyDef {
        name: "codeLanguage",
        label: "codeLanguage",
        domain: "zakhor:CodeContainer",
        range: "xsd:string",
    },
    PropertyDef {
        name: "codeSymbolKind",
        label: "codeSymbolKind",
        domain: "zakhor:CodeSymbol",
        range: "xsd:string",
    },
    PropertyDef {
        name: "codeLineStart",
        label: "codeLineStart",
        domain: "zakhor:CodeSymbol",
        range: "xsd:integer",
    },
    PropertyDef {
        name: "tombstone",
        label: "tombstone",
        domain: "rdfs:Resource",
        range: "xsd:boolean",
    },
];

/// FNV-1a 64-bit hash — deterministic across platforms and Rust versions.
pub fn fnv1a64(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Fixed epoch the hash-derived `nrl:lastModified` values are anchored to.
const LAST_MODIFIED_EPOCH_SECS: i64 = 1_767_225_600; // 2026-01-01T00:00:00Z
const LAST_MODIFIED_WINDOW_SECS: u64 = 316_224_000; // ~10 years

/// Civil-from-days algorithm (Howard Hinnant's) — days since 1970-01-01 → (y, m, d).
fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// Derive an `xsd:dateTime` last-modified value from a content hash.
///
/// Any change to the canonical schema content changes the hash, which changes
/// the timestamp — so the ontology can never go silently stale.
pub fn derived_last_modified(hash: u64) -> String {
    let secs = LAST_MODIFIED_EPOCH_SECS + hash.rem_euclid(LAST_MODIFIED_WINDOW_SECS) as i64;
    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// Hash of the canonical schema content (classes, properties, relation labels).
pub fn canonical_schema_hash(relation_labels: &[String]) -> u64 {
    let mut content = String::with_capacity(1024);
    content.push_str("zakhor-ontology-v1\n");
    for class in CLASSES {
        content.push_str(&format!("class {} label {}\n", class.name, class.label));
    }
    for prop in PROPERTIES {
        content.push_str(&format!(
            "property {} label {} domain {} range {}\n",
            prop.name, prop.label, prop.domain, prop.range
        ));
    }
    for label in relation_labels {
        content.push_str(&format!("relation {label}\n"));
    }
    fnv1a64(&content)
}

/// The `nrl:lastModified` value for the ontology generated from `relation_labels`.
pub fn ontology_last_modified(relation_labels: &[String]) -> String {
    derived_last_modified(canonical_schema_hash(relation_labels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a64_is_deterministic_and_collides_in_own_hash_space() {
        assert_eq!(fnv1a64("abc"), fnv1a64("abc"));
        assert_ne!(fnv1a64("abc"), fnv1a64("abd"));
    }

    #[test]
    fn fnv1a64_detects_schema_changes() {
        let mut labels = vec!["worksAt".to_string()];
        let base = canonical_schema_hash(&labels);
        labels.push("locatedIn".to_string());
        assert_ne!(base, canonical_schema_hash(&labels));
    }

    #[test]
    fn derived_last_modified_is_valid_xsd_date_time() {
        assert_eq!(derived_last_modified(0), "2026-01-01T00:00:00Z");
        let ts = derived_last_modified(1);
        // YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(ts.len(), 20, "bad timestamp length: {ts}");
        assert!(ts.starts_with("2026-"), "anchored near epoch: {ts}");
        assert!(ts.ends_with('Z'), "must end in Z: {ts}");
    }

    #[test]
    fn ontology_last_modified_is_deterministic() {
        let labels = vec!["worksAt".to_string()];
        assert_eq!(
            ontology_last_modified(&labels),
            ontology_last_modified(&labels)
        );
        let other_labels = vec!["worksAt".to_string(), "locatedIn".to_string()];
        assert_ne!(
            ontology_last_modified(&labels),
            ontology_last_modified(&other_labels)
        );
    }
}

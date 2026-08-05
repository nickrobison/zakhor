use axum::{Json, extract::Path, extract::State};
use serde::Serialize;
use tracker::prelude::SparqlCursorExtManual;

use super::ApiState;
use crate::api::error::{ApiError, ApiResult};
use zakhor_storage::sparql::prefix_declarations;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ObservationDetail {
    pub id: String,
    pub content: String,
    pub created_at: Option<String>,
    pub entity_refs: Vec<String>,
}

// ---------------------------------------------------------------------------
// SPARQL query builders
// ---------------------------------------------------------------------------

/// SELECT query returning entities referenced by an observation.
fn build_observation_entities_query(obs_id: &str) -> String {
    let safe_id = super::sanitize_id(obs_id);
    let prefixes = prefix_declarations();
    format!(
        "{prefixes}SELECT DISTINCT ?entity
WHERE {{
  <{id}> zakhor:hasEntity ?entity .
}}",
        prefixes = prefixes,
        id = safe_id,
    )
}

/// SELECT query returning a single observation's detail fields.
fn build_observation_detail_query(obs_id: &str) -> String {
    let safe_id = super::sanitize_id(obs_id);
    let prefixes = prefix_declarations();
    format!(
        "{prefixes}SELECT ?identifier ?text ?created
WHERE {{
  <{id}> rdf:type nie:InformationElement ;
         nie:identifier ?identifier ;
         nie:plainTextContent ?text .
  OPTIONAL {{ <{id}> nie:contentCreated ?created . }}
}}",
        prefixes = prefixes,
        id = safe_id,
    )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/observations/{id}",
    params(
        ("id" = String, Path, description = "Observation ID (SPARQL subject URI)")
    ),
    responses(
        (status = OK, description = "Observation detail", body = ObservationDetail),
        (status = NOT_FOUND, description = "Observation not found", body = crate::api::error::ErrorBody)
    )
)]
pub async fn get_observation(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ObservationDetail>> {
    let id = id.trim();
    if id.is_empty() {
        return Err(ApiError::bad_request("id is required"));
    }

    // Fetch detail fields
    let cursor = state
        .connection()
        .query(
            &build_observation_detail_query(id),
            None::<&gio::Cancellable>,
        )
        .map_err(|e| ApiError::internal(format!("SPARQL error: {e}")))?;

    let mut content = String::new();
    let mut created_at: Option<String> = None;
    let mut found = false;

    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        found = true;
        content = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        created_at = cursor.string(2).map(|s| s.to_string());
    }

    if !found {
        return Err(ApiError::not_found(format!("Observation not found: {id}")));
    }

    // Fetch entity references
    let entity_refs = {
        let cursor = state
            .connection()
            .query(
                &build_observation_entities_query(id),
                None::<&gio::Cancellable>,
            )
            .map_err(|e| ApiError::internal(format!("SPARQL error: {e}")))?;
        let mut refs = Vec::new();
        while cursor
            .next(None::<&gio::Cancellable>)
            .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
        {
            if let Some(entity) = cursor.string(0).map(|s| s.to_string()) {
                refs.push(entity);
            }
        }
        refs
    };

    Ok(Json(ObservationDetail {
        id: id.to_string(),
        content,
        created_at,
        entity_refs,
    }))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_observation_entities_query() {
        let q = build_observation_entities_query("urn:uuid:obs-1");
        assert!(q.contains("zakhor:hasEntity"));
        assert!(q.contains("<urn:uuid:obs-1>"));
    }

    #[test]
    fn test_build_observation_detail_query() {
        let q = build_observation_detail_query("urn:uuid:obs-1");
        assert!(q.contains("nie:plainTextContent"));
        assert!(q.contains("nie:contentCreated"));
        assert!(q.contains("<urn:uuid:obs-1>"));
    }

    #[test]
    fn test_observation_detail() {
        let d = ObservationDetail {
            id: "obs-1".into(),
            content: "detail".into(),
            created_at: None,
            entity_refs: vec!["urn:entity:1".into()],
        };
        assert_eq!(d.entity_refs.len(), 1);
    }
}

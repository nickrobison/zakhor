use super::ApiState;
use crate::api::error::{ApiError, ApiResult};
use crate::server::EntityResult;
use crate::tools;
use axum::{Json, extract::Path, extract::Query, extract::State};
use serde::{Deserialize, Serialize};
use tracker::prelude::SparqlCursorExtManual;

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

fn default_limit() -> u32 {
    20
}

fn clamp_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
}

fn is_missing_schema_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("unknown class") || lower.contains("unknown property")
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct EntityListQuery {
    /// Label pattern to search for
    #[serde(default)]
    q: String,
    #[serde(default = "default_limit")]
    limit: u32,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EntityListResponse {
    pub entities: Vec<EntityResult>,
    pub count: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EntityDetail {
    pub uri: String,
    pub label: String,
    pub types: Vec<String>,
    pub related_decisions: Vec<EntityRef>,
    pub related_observations: Vec<ObservationRef>,
    pub relationships: Vec<crate::ingestion::Relation>,
    pub source_locations: Vec<SourceLocation>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EntityRef {
    pub uri: String,
    pub label: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ObservationRef {
    pub uri: String,
    pub text: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SourceLocation {
    pub uri: String,
    pub label: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EntityDecisionsResponse {
    pub decisions: Vec<DecisionRef>,
    pub count: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DecisionRef {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EntityObservationsResponse {
    pub observations: Vec<ObservationRef>,
    pub count: usize,
}

// ---------------------------------------------------------------------------
// SPARQL query builders
// ---------------------------------------------------------------------------

/// Build a SELECT query returning all properties of an entity.
fn build_entity_properties_query(entity_id: &str) -> String {
    let safe_id = entity_id.replace(['>', '<'], "");
    format!(
        "PREFIX zakhor: <https://zakhor.example/>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?p ?o
WHERE {{
  <{id}> ?p ?o .
}}",
        id = safe_id,
    )
}

/// Build a SELECT query returning observations that reference an entity.
fn build_entity_observations_query(entity_id: &str, limit: u32) -> String {
    let safe_id = entity_id.replace(['>', '<'], "");
    format!(
        "PREFIX zakhor: <https://zakhor.example/>
PREFIX nie: <http://www.semanticdesktop.org/ontologies/2007/01/19/nie#>
SELECT DISTINCT ?obs ?text
WHERE {{
  ?obs zakhor:hasEntity <{id}> .
  ?obs nie:plainTextContent ?text .
}}
LIMIT {limit}",
        id = safe_id,
        limit = limit,
    )
}

/// Build a SELECT query returning relations that involve an entity
/// (as either subject or object).
fn build_entity_relations_query(entity_id: &str) -> String {
    let safe_id = entity_id.replace(['>', '<'], "");
    format!(
        "PREFIX zakhor: <https://zakhor.example/>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?s ?p ?o ?label
WHERE {{
  {{ <{id}> ?p ?o . BIND(<{id}> AS ?s) }}
  UNION
  {{ ?s ?p <{id}> . BIND(<{id}> AS ?o) }}
  OPTIONAL {{ ?p rdfs:label ?label . }}
}}",
        id = safe_id,
    )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/entities",
    params(EntityListQuery),
    responses(
        (status = OK, description = "Entity list", body = EntityListResponse),
        (status = BAD_REQUEST, description = "Invalid query", body = crate::api::error::ErrorBody)
    )
)]
pub async fn list_entities(
    State(state): State<ApiState>,
    Query(query): Query<EntityListQuery>,
) -> ApiResult<Json<EntityListResponse>> {
    let limit = clamp_limit(query.limit);
    let pattern = query.q.trim();

    let sparql = tools::build_entity_query(pattern, limit);
    let cursor = match state.connection().query(&sparql, None::<&gio::Cancellable>) {
        Ok(cursor) => cursor,
        Err(error) if is_missing_schema_error(&error.to_string()) => {
            return Ok(Json(EntityListResponse {
                entities: vec![],
                count: 0,
            }));
        }
        Err(e) => return Err(ApiError::internal(format!("SPARQL error: {e}"))),
    };

    let mut entities = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        let uri = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let label = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        entities.push(EntityResult { uri, label });
    }

    let count = entities.len();
    Ok(Json(EntityListResponse { entities, count }))
}

#[utoipa::path(
    get,
    path = "/api/v1/entities/{id}",
    params(
        ("id" = String, Path, description = "Entity ID (URI)")
    ),
    responses(
        (status = OK, description = "Entity detail", body = EntityDetail),
        (status = NOT_FOUND, description = "Entity not found", body = crate::api::error::ErrorBody)
    )
)]
pub async fn get_entity(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<EntityDetail>> {
    let id = id.trim();
    if id.is_empty() {
        return Err(ApiError::bad_request("id is required"));
    }

    // Get entity properties
    let sparql = build_entity_properties_query(id);
    let cursor = state
        .connection()
        .query(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("SPARQL error: {e}")))?;

    let zakhor_prefix = "https://zakhor.example/";
    let rdfs_label = "http://www.w3.org/2000/01/rdf-schema#label";
    let rdf_type = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let mut label = String::new();
    let mut types = Vec::new();
    let mut found_entity = false;

    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        let p = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let o = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();

        if p == rdfs_label {
            label = o;
        } else if p == rdf_type && o == format!("{}Entity", zakhor_prefix) {
            found_entity = true;
        } else if p == rdf_type && o != format!("{}Entity", zakhor_prefix) {
            // Capture additional type badges
            let type_name = o
                .trim_start_matches(zakhor_prefix)
                .trim_start_matches("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
                .to_string();
            if !type_name.is_empty() && type_name != "Class" && type_name != "Resource" {
                types.push(type_name);
            }
        }
    }

    if !found_entity && label.is_empty() {
        return Err(ApiError::bad_request(format!("Entity not found: {id}")));
    }

    if types.is_empty() {
        types.push("Entity".to_string());
    }

    // Fetch related observations
    let observations = fetch_entity_observations(state.connection(), id)?;

    // Fetch relationships
    let relationships = fetch_entity_relations(state.connection(), id)?;

    Ok(Json(EntityDetail {
        uri: id.to_string(),
        label,
        types,
        related_decisions: vec![],
        related_observations: observations,
        relationships,
        source_locations: vec![],
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/entities/{id}/decisions",
    params(
        ("id" = String, Path, description = "Entity ID (URI)")
    ),
    responses(
        (status = OK, description = "Related decisions", body = EntityDecisionsResponse)
    )
)]
pub async fn get_entity_decisions(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
) -> ApiResult<Json<EntityDecisionsResponse>> {
    // Entity-to-decision relationships are not modeled in the current
    // SPARQL store. Return an empty list — this endpoint will return
    // real data once the link is modeled.
    Ok(Json(EntityDecisionsResponse {
        decisions: vec![],
        count: 0,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/entities/{id}/observations",
    params(
        ("id" = String, Path, description = "Entity ID (URI)")
    ),
    responses(
        (status = OK, description = "Related observations", body = EntityObservationsResponse)
    )
)]
pub async fn get_entity_observations(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<EntityObservationsResponse>> {
    let id = id.trim();
    if id.is_empty() {
        return Err(ApiError::bad_request("id is required"));
    }

    let observations = fetch_entity_observations(state.connection(), id)?;
    let count = observations.len();

    Ok(Json(EntityObservationsResponse {
        observations,
        count,
    }))
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn fetch_entity_observations(
    conn: &tracker::SparqlConnection,
    entity_id: &str,
) -> Result<Vec<ObservationRef>, ApiError> {
    let sparql = build_entity_observations_query(entity_id, 50);
    let cursor = match conn.query(&sparql, None::<&gio::Cancellable>) {
        Ok(cursor) => cursor,
        Err(error) if is_missing_schema_error(&error.to_string()) => return Ok(Vec::new()),
        Err(e) => return Err(ApiError::internal(format!("SPARQL error: {e}"))),
    };

    let mut observations = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        let uri = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let text = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        observations.push(ObservationRef { uri, text });
    }
    Ok(observations)
}

fn fetch_entity_relations(
    conn: &tracker::SparqlConnection,
    entity_id: &str,
) -> Result<Vec<crate::ingestion::Relation>, ApiError> {
    let sparql = build_entity_relations_query(entity_id);
    let cursor = conn
        .query(&sparql, None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("SPARQL error: {e}")))?;

    let mut relations = Vec::new();
    while cursor
        .next(None::<&gio::Cancellable>)
        .map_err(|e| ApiError::internal(format!("Cursor error: {e}")))?
    {
        let s = cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
        let p = cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
        let o = cursor.string(2).map(|s| s.to_string()).unwrap_or_default();
        let label = cursor.string(3).map(|s| s.to_string()).unwrap_or_default();

        // Filter out rdf:type and rdfs:label triples (already captured in types/label)
        if p == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
            || p == "http://www.w3.org/2000/01/rdf-schema#label"
        {
            continue;
        }

        relations.push(crate::ingestion::Relation {
            subject_uri: s,
            predicate_uri: p,
            object_uri: o,
            label,
        });
    }
    Ok(relations)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 20);
    }

    #[test]
    fn test_clamp_limit() {
        assert_eq!(clamp_limit(0), 1);
        assert_eq!(clamp_limit(50), 50);
        assert_eq!(clamp_limit(500), 100);
    }

    #[test]
    fn test_is_missing_schema_error() {
        assert!(is_missing_schema_error(
            "Unknown class 'https://zakhor.example/Entity'"
        ));
        assert!(is_missing_schema_error(
            "Unknown property 'zakhor:hasEntity'"
        ));
        assert!(!is_missing_schema_error("Cursor error"));
    }

    #[test]
    fn test_build_entity_properties_query() {
        let q = build_entity_properties_query("urn:uuid:abc");
        assert!(q.contains("SELECT ?p ?o"));
        assert!(q.contains("<urn:uuid:abc>"));
    }

    #[test]
    fn test_build_entity_observations_query() {
        let q = build_entity_observations_query("urn:uuid:entity-1", 10);
        assert!(q.contains("zakhor:hasEntity"));
        assert!(q.contains("nie:plainTextContent"));
        assert!(q.contains("LIMIT 10"));
        assert!(q.contains("<urn:uuid:entity-1>"));
    }

    #[test]
    fn test_build_entity_relations_query() {
        let q = build_entity_relations_query("urn:uuid:rel-test");
        assert!(q.contains("UNION"));
        assert!(q.contains("<urn:uuid:rel-test>"));
        assert!(q.contains("rdfs:label"));
    }

    #[test]
    fn test_entity_detail_defaults() {
        let detail = EntityDetail {
            uri: "urn:uuid:abc".to_string(),
            label: "Test Entity".to_string(),
            types: vec!["Entity".to_string()],
            related_decisions: vec![],
            related_observations: vec![],
            relationships: vec![],
            source_locations: vec![],
        };
        assert_eq!(detail.uri, "urn:uuid:abc");
        assert_eq!(detail.types.len(), 1);
        assert_eq!(detail.types[0], "Entity");
    }

    #[test]
    fn test_entity_list_response() {
        let resp = EntityListResponse {
            entities: vec![EntityResult {
                uri: "urn:uuid:e1".to_string(),
                label: "Entity One".to_string(),
            }],
            count: 1,
        };
        assert_eq!(resp.count, 1);
        assert_eq!(resp.entities[0].label, "Entity One");
    }

    #[test]
    fn test_entity_observations_response_empty() {
        let resp = EntityObservationsResponse {
            observations: vec![],
            count: 0,
        };
        assert_eq!(resp.count, 0);
        assert!(resp.observations.is_empty());
    }

    #[test]
    fn test_entity_decisions_response_empty() {
        let resp = EntityDecisionsResponse {
            decisions: vec![],
            count: 0,
        };
        assert_eq!(resp.count, 0);
        assert!(resp.decisions.is_empty());
    }

    #[test]
    fn test_observation_ref() {
        let obs = ObservationRef {
            uri: "urn:uuid:obs-1".to_string(),
            text: "Some observation".to_string(),
        };
        assert_eq!(obs.uri, "urn:uuid:obs-1");
        assert_eq!(obs.text, "Some observation");
    }
}

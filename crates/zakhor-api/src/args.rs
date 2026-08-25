use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct RebuildIndexesArgs {}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct QueryEntitiesArgs {
    pub pattern: String,
    pub limit: u32,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct TraverseGraphArgs {
    pub start_id: String,
    pub depth: u32,
    pub edge_types: Vec<String>,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct SearchHybridArgs {
    pub query: String,
    pub limit: u32,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct RecordDecisionArgs {
    pub context: String,
    pub decision: String,
    pub alternatives: Vec<String>,
    pub rationale: String,
    /// Optional project URI to associate this decision with.
    pub project_uri: Option<String>,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct ExtractAndStoreArgs {
    pub uri: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct ExtractAndStoreResponse {
    pub observation_uri: String,
    pub entity_count: u64,
    pub relation_count: u64,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct StoreObservationResponse {
    pub observation_uri: String,
    pub triple_count: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct EntityResult {
    pub uri: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct QueryEntitiesResponse {
    pub entities: Vec<EntityResult>,
    pub count: u64,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct TripleResult {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct TraverseGraphResponse {
    pub triples: Vec<TripleResult>,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct SearchResult {
    pub id: String,
    pub score: f64,
    pub text: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct SearchHybridResponse {
    pub results: Vec<SearchResult>,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct RecordDecisionResponse {
    pub decision_uri: String,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct AdminInjectToolCallArgs {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct AdminInjectToolCallResponse {
    pub uri: String,
}

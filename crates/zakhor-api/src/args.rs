use oxiri::Iri;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use zakhor_common::vocab::{EntityUri, ObservationUri, ProjectUri, RepositoryUri};

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
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub uri: EntityUri,
    pub text: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct ExtractAndStoreResponse {
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub observation_uri: ObservationUri,
    pub entity_count: u64,
    pub relation_count: u64,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct StoreObservationResponse {
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub observation_uri: ObservationUri,
    pub triple_count: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct EntityResult {
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub uri: EntityUri,
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
    /// True when the traversal hit [`MAX_TRAVERSE_TRIPLES`] and stopped early.
    pub truncated: bool,
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
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub decision_uri: Iri<String>,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct CreateProjectArgs {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct LinkToProjectArgs {
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub entity_uri: EntityUri,
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub project_uri: ProjectUri,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct CreateProjectResponse {
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub project_uri: ProjectUri,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct CreateRepositoryArgs {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct LinkToRepositoryArgs {
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub entity_uri: EntityUri,
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub repository_uri: RepositoryUri,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct CreateRepositoryResponse {
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub repository_uri: RepositoryUri,
}

#[derive(Deserialize, Serialize, JsonSchema, utoipa::ToSchema)]
pub struct AdminInjectToolCallArgs {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub session_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema, utoipa::ToSchema)]
pub struct AdminInjectToolCallResponse {
    // A ToolCall node IRI. Not one of the six vocabulary instance-IRI shapes,
    // so it stays a validated-by-construction String rather than gaining a
    // newtype; the plan forbids new vocabulary definitions.
    pub uri: String,
}

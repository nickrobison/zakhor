use axum::{Json, extract::Query, extract::State};
use serde::{Deserialize, Serialize};

use super::ApiState;
use crate::api::error::ApiResult;

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CodeQuery {
    /// Search query (optional — returns all code when empty).
    #[serde(default)]
    q: String,
    /// Filter by repository name (optional).
    #[serde(default)]
    repo: String,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CodeResponse {
    pub repositories: Vec<CodeRepository>,
    pub files: Vec<CodeFile>,
    pub symbols: Vec<CodeSymbol>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CodeRepository {
    pub name: String,
    pub url: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CodeFile {
    pub path: String,
    pub repository: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CodeSymbol {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line: u32,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/code",
    params(CodeQuery),
    responses(
        (status = OK, description = "Code references", body = CodeResponse)
    )
)]
pub async fn get_code(
    State(_state): State<ApiState>,
    Query(query): Query<CodeQuery>,
) -> ApiResult<Json<CodeResponse>> {
    let _ = (&query.q, &query.repo);

    // Code indexing is not yet implemented. Return empty arrays until a
    // code-indexing pipeline is added to the synchronisation layer.
    Ok(Json(CodeResponse {
        repositories: vec![],
        files: vec![],
        symbols: vec![],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_response_empty() {
        let r = CodeResponse {
            repositories: vec![],
            files: vec![],
            symbols: vec![],
        };
        assert!(r.repositories.is_empty());
        assert!(r.files.is_empty());
        assert!(r.symbols.is_empty());
    }

    #[test]
    fn test_code_repository() {
        let r = CodeRepository {
            name: "my-repo".into(),
            url: Some("https://github.com/user/repo".into()),
            description: None,
        };
        assert_eq!(r.name, "my-repo");
    }

    #[test]
    fn test_code_file() {
        let f = CodeFile {
            path: "src/main.rs".into(),
            repository: None,
            language: Some("rust".into()),
        };
        assert_eq!(f.path, "src/main.rs");
    }

    #[test]
    fn test_code_symbol() {
        let s = CodeSymbol {
            name: "main".into(),
            kind: "function".into(),
            file_path: "src/main.rs".into(),
            line: 1,
        };
        assert_eq!(s.name, "main");
        assert_eq!(s.line, 1);
    }
}

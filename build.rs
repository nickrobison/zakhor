use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let manifest_dir = env::var("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .expect("CARGO_MANIFEST_DIR should be set");
            manifest_dir.join("target")
        });
    fs::create_dir_all(&target_dir)?;
    fs::write(target_dir.join("openapi.json"), OPENAPI_JSON)?;
    println!("cargo:rerun-if-changed=build.rs");

    // When tracker-sys was compiled with the `vendored` feature it builds
    // TinySPARQL from source and exposes the library directory via Cargo's
    // DEP_ metadata mechanism.  Embed that directory as an RPATH so the
    // zakhor binary can find the shared library at runtime without requiring
    // LD_LIBRARY_PATH / DYLD_LIBRARY_PATH to be set.
    if let Ok(lib_dir) = env::var("DEP_TRACKER_SPARQL_3_0_LIB_DIR") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
    }

    Ok(())
}

const OPENAPI_JSON: &str = r##"{
  "openapi": "3.1.0",
  "info": {
    "title": "Zakhor API",
    "version": "0.1.0"
  },
  "paths": {
    "/api/v1/health": {
      "get": {
        "operationId": "health",
        "responses": {
          "200": {
            "description": "Tracker health check",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/HealthResponse" }
              }
            }
          },
          "503": {
            "description": "Tracker unavailable",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/HealthResponse" }
              }
            }
          }
        }
      }
    },
    "/api/v1/search": {
      "get": {
        "operationId": "search",
        "parameters": [
          { "name": "q", "in": "query", "required": true, "schema": { "type": "string" } },
          { "name": "limit", "in": "query", "required": false, "schema": { "type": "integer", "format": "uint32", "default": 20 } },
          { "name": "mode", "in": "query", "required": false, "schema": { "type": "string", "default": "hybrid", "enum": ["hybrid", "lexical", "semantic"] }, "example": "hybrid" }
        ],
        "responses": {
          "200": {
            "description": "Hybrid search results",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/SearchHybridResponse" }
              }
            }
          },
          "400": {
            "description": "Missing search query",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/graph/traverse": {
      "get": {
        "operationId": "traverseGraph",
        "parameters": [
          { "name": "start_id", "in": "query", "required": true, "schema": { "type": "string" } },
          { "name": "depth", "in": "query", "required": false, "schema": { "type": "integer", "format": "uint32", "default": 1 } },
          { "name": "edge_types", "in": "query", "required": false, "schema": { "type": "string" } }
        ],
        "responses": {
          "200": {
            "description": "Graph traversal triples",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/TraverseGraphResponse" }
              }
            }
          },
          "400": {
            "description": "Invalid graph query",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/openapi.json": {
      "get": {
        "operationId": "openapiJson",
        "responses": {
          "200": { "description": "OpenAPI 3.1 document" }
        }
      }
    },
    "/api/v1/decisions": {
      "get": {
        "operationId": "listDecisions",
        "parameters": [
          { "name": "q", "in": "query", "required": false, "schema": { "type": "string" } },
          { "name": "status", "in": "query", "required": false, "schema": { "type": "string" } },
          { "name": "sort", "in": "query", "required": false, "schema": { "type": "string" } },
          { "name": "limit", "in": "query", "required": false, "schema": { "type": "integer", "format": "uint32", "default": 20 } },
          { "name": "offset", "in": "query", "required": false, "schema": { "type": "integer", "format": "uint32", "default": 0 } }
        ],
        "responses": {
          "200": {
            "description": "Decision list",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/DecisionListResponse" }
              }
            }
          },
          "400": {
            "description": "Invalid query",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/decisions/{id}": {
      "get": {
        "operationId": "getDecision",
        "parameters": [
          { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
        ],
        "responses": {
          "200": {
            "description": "Decision detail",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/DecisionDetail" }
              }
            }
          },
          "400": {
            "description": "Decision not found or invalid id",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/decisions/{id}/provenance": {
      "get": {
        "operationId": "getDecisionProvenance",
        "parameters": [
          { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
        ],
        "responses": {
          "200": {
            "description": "Provenance chain",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ProvenanceResponse" }
              }
            }
          },
          "400": {
            "description": "Invalid decision id",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/entities": {
      "get": {
        "operationId": "listEntities",
        "parameters": [
          { "name": "q", "in": "query", "required": false, "schema": { "type": "string" } },
          { "name": "limit", "in": "query", "required": false, "schema": { "type": "integer", "format": "uint32", "default": 20 } }
        ],
        "responses": {
          "200": {
            "description": "Entity list",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/EntityListResponse" }
              }
            }
          },
          "400": {
            "description": "Invalid query",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/entities/{id}": {
      "get": {
        "operationId": "getEntity",
        "parameters": [
          { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
        ],
        "responses": {
          "200": {
            "description": "Entity detail",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/EntityDetail" }
              }
            }
          },
          "400": {
            "description": "Entity not found or invalid id",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/entities/{id}/decisions": {
      "get": {
        "operationId": "getEntityDecisions",
        "parameters": [
          { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
        ],
        "responses": {
          "200": {
            "description": "Related decisions",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/EntityDecisionsResponse" }
              }
            }
          },
          "400": {
            "description": "Invalid entity id",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/entities/{id}/observations": {
      "get": {
        "operationId": "getEntityObservations",
        "parameters": [
          { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
        ],
        "responses": {
          "200": {
            "description": "Related observations",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/EntityObservationsResponse" }
              }
            }
          },
          "400": {
            "description": "Invalid entity id",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/observations": {
      "get": {
        "operationId": "listObservations",
        "parameters": [
          { "name": "entity_id", "in": "query", "required": false, "schema": { "type": "string" } },
          { "name": "offset", "in": "query", "required": false, "schema": { "type": "integer", "format": "uint32", "default": 0 } },
          { "name": "limit", "in": "query", "required": false, "schema": { "type": "integer", "format": "uint32", "default": 20 } }
        ],
        "responses": {
          "200": {
            "description": "Observation list",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ObservationListResponse" }
              }
            }
          },
          "400": {
            "description": "Invalid query",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/observations/{id}": {
      "get": {
        "operationId": "getObservation",
        "parameters": [
          { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
        ],
        "responses": {
          "200": {
            "description": "Observation detail",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ObservationDetail" }
              }
            }
          },
          "404": {
            "description": "Observation not found",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/admin/rebuild-indexes": {
      "post": {
        "operationId": "rebuildIndexes",
        "responses": {
          "202": {
            "description": "Rebuild accepted",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/RebuildResponse" }
              }
            }
          },
          "409": {
            "description": "Rebuild already in progress",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/ErrorBody" }
              }
            }
          }
        }
      }
    },
    "/api/v1/admin/status": {
      "get": {
        "operationId": "adminStatus",
        "responses": {
          "200": {
            "description": "Admin status",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/AdminStatusResponse" }
              }
            }
          }
        }
      }
    },
    "/api/v1/code": {
      "get": {
        "operationId": "getCode",
        "parameters": [
          { "name": "q", "in": "query", "required": false, "schema": { "type": "string" } },
          { "name": "repo", "in": "query", "required": false, "schema": { "type": "string" } }
        ],
        "responses": {
          "200": {
            "description": "Code references",
            "content": {
              "application/json": {
                "schema": { "$ref": "#/components/schemas/CodeResponse" }
              }
            }
          }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "HealthResponse": {
        "type": "object",
        "required": ["status"],
        "properties": {
          "status": { "type": "string", "enum": ["ok", "tracker_unavailable"] }
        }
      },
      "ErrorBody": {
        "type": "object",
        "required": ["error"],
        "properties": {
          "error": { "type": "string" }
        }
      },
      "EntityRef": {
        "type": "object",
        "required": ["uri", "label"],
        "properties": {
          "uri": { "type": "string" },
          "label": { "type": "string" }
        }
      },
      "Relation": {
        "type": "object",
        "required": ["subject_uri", "predicate_uri", "object_uri", "label"],
        "properties": {
          "subject_uri": { "type": "string" },
          "predicate_uri": { "type": "string" },
          "object_uri": { "type": "string" },
          "label": { "type": "string" }
        }
      },
      "StoreObservationArgs": {
        "type": "object",
        "required": ["text", "entities", "relations"],
        "properties": {
          "text": { "type": "string" },
          "entities": { "type": "array", "items": { "$ref": "#/components/schemas/EntityRef" } },
          "relations": { "type": "array", "items": { "$ref": "#/components/schemas/Relation" } }
        }
      },
      "StoreObservationResponse": {
        "type": "object",
        "required": ["observation_uri", "triple_count"],
        "properties": {
          "observation_uri": { "type": "string" },
          "triple_count": { "type": "integer", "format": "uint", "minimum": 0 }
        }
      },
      "RebuildIndexesArgs": { "type": "object", "properties": {} },
      "QueryEntitiesArgs": {
        "type": "object",
        "required": ["pattern", "limit"],
        "properties": {
          "pattern": { "type": "string" },
          "limit": { "type": "integer", "format": "uint32", "minimum": 0 }
        }
      },
      "EntityResult": {
        "type": "object",
        "required": ["uri", "label"],
        "properties": {
          "uri": { "type": "string" },
          "label": { "type": "string" }
        }
      },
      "QueryEntitiesResponse": {
        "type": "object",
        "required": ["entities", "count"],
        "properties": {
          "entities": { "type": "array", "items": { "$ref": "#/components/schemas/EntityResult" } },
          "count": { "type": "integer", "format": "uint", "minimum": 0 }
        }
      },
      "TraverseGraphArgs": {
        "type": "object",
        "required": ["start_id", "depth", "edge_types"],
        "properties": {
          "start_id": { "type": "string" },
          "depth": { "type": "integer", "format": "uint32", "minimum": 0 },
          "edge_types": { "type": "array", "items": { "type": "string" } }
        }
      },
      "TripleResult": {
        "type": "object",
        "required": ["subject", "predicate", "object"],
        "properties": {
          "subject": { "type": "string" },
          "predicate": { "type": "string" },
          "object": { "type": "string" }
        }
      },
      "TraverseGraphResponse": {
        "type": "object",
        "required": ["triples", "count"],
        "properties": {
          "triples": { "type": "array", "items": { "$ref": "#/components/schemas/TripleResult" } },
          "count": { "type": "integer", "format": "uint", "minimum": 0 },
          "warning": { "type": ["string", "null"] }
        }
      },
      "SearchHybridArgs": {
        "type": "object",
        "required": ["query", "limit"],
        "properties": {
          "query": { "type": "string" },
          "limit": { "type": "integer", "format": "uint32", "minimum": 0 }
        }
      },
      "SearchResult": {
        "type": "object",
        "required": ["id", "score"],
        "properties": {
          "id": { "type": "string" },
          "score": { "type": "number", "format": "double" }
        }
      },
      "SearchHybridResponse": {
        "type": "object",
        "required": ["results", "count"],
        "properties": {
          "results": { "type": "array", "items": { "$ref": "#/components/schemas/SearchResult" } },
          "count": { "type": "integer", "format": "uint", "minimum": 0 },
          "warning": { "type": ["string", "null"] }
        }
      },
      "RecordDecisionArgs": {
        "type": "object",
        "required": ["context", "decision", "alternatives", "rationale"],
        "properties": {
          "context": { "type": "string" },
          "decision": { "type": "string" },
          "alternatives": { "type": "array", "items": { "type": "string" } },
          "rationale": { "type": "string" }
        }
      },
      "RecordDecisionResponse": {
        "type": "object",
        "required": ["decision_uri"],
        "properties": {
          "decision_uri": { "type": "string" }
        }
      },
      "DecisionSummary": {
        "type": "object",
        "required": ["id", "title", "status"],
        "properties": {
          "id": { "type": "string" },
          "title": { "type": "string" },
          "status": { "type": "string" },
          "created": { "type": "string", "format": "date-time", "nullable": true },
          "modified": { "type": "string", "format": "date-time", "nullable": true }
        }
      },
      "DecisionListResponse": {
        "type": "object",
        "required": ["decisions", "count", "total"],
        "properties": {
          "decisions": { "type": "array", "items": { "$ref": "#/components/schemas/DecisionSummary" } },
          "count": { "type": "integer", "format": "uint", "minimum": 0 },
          "total": { "type": "integer", "format": "uint", "minimum": 0 }
        }
      },
      "DecisionDetail": {
        "type": "object",
        "required": ["id", "title", "status", "context", "outcome", "rationale", "alternatives", "evidence", "entities", "related_decision_ids"],
        "properties": {
          "id": { "type": "string" },
          "title": { "type": "string" },
          "status": { "type": "string" },
          "created": { "type": "string", "format": "date-time", "nullable": true },
          "modified": { "type": "string", "format": "date-time", "nullable": true },
          "context": { "type": "string" },
          "outcome": { "type": "string" },
          "rationale": { "type": "string" },
          "alternatives": { "type": "array", "items": { "type": "string" } },
          "evidence": { "type": "array", "items": { "$ref": "#/components/schemas/EvidenceItem" } },
          "entities": { "type": "array", "items": { "$ref": "#/components/schemas/EntityRef" } },
          "related_decision_ids": { "type": "array", "items": { "type": "string" } }
        }
      },
      "EvidenceItem": {
        "type": "object",
        "required": ["source", "content"],
        "properties": {
          "source": { "type": "string" },
          "content": { "type": "string" }
        }
      },
      "ProvenanceItem": {
        "type": "object",
        "required": ["step", "label", "source"],
        "properties": {
          "step": { "type": "string" },
          "label": { "type": "string" },
          "source": { "type": "string" }
        }
      },
      "ProvenanceResponse": {
        "type": "object",
        "required": ["chain", "count"],
        "properties": {
          "chain": { "type": "array", "items": { "$ref": "#/components/schemas/ProvenanceItem" } },
          "count": { "type": "integer", "format": "uint", "minimum": 0 }
        }
      },
      "EntityListResponse": {
        "type": "object",
        "required": ["entities", "count"],
        "properties": {
          "entities": { "type": "array", "items": { "$ref": "#/components/schemas/EntityResult" } },
          "count": { "type": "integer", "format": "uint", "minimum": 0 }
        }
      },
      "EntityDetail": {
        "type": "object",
        "required": ["uri", "label", "types", "related_decisions", "related_observations", "relationships", "source_locations"],
        "properties": {
          "uri": { "type": "string" },
          "label": { "type": "string" },
          "types": { "type": "array", "items": { "type": "string" } },
          "related_decisions": { "type": "array", "items": { "$ref": "#/components/schemas/EntityRef" } },
          "related_observations": { "type": "array", "items": { "$ref": "#/components/schemas/ObservationRef" } },
          "relationships": { "type": "array", "items": { "$ref": "#/components/schemas/Relation" } },
          "source_locations": { "type": "array", "items": { "$ref": "#/components/schemas/SourceLocation" } }
        }
      },
      "ObservationRef": {
        "type": "object",
        "required": ["uri", "text"],
        "properties": {
          "uri": { "type": "string" },
          "text": { "type": "string" }
        }
      },
      "SourceLocation": {
        "type": "object",
        "required": ["uri", "label"],
        "properties": {
          "uri": { "type": "string" },
          "label": { "type": "string" }
        }
      },
      "EntityDecisionsResponse": {
        "type": "object",
        "required": ["decisions", "count"],
        "properties": {
          "decisions": { "type": "array", "items": { "$ref": "#/components/schemas/DecisionRef" } },
          "count": { "type": "integer", "format": "uint", "minimum": 0 }
        }
      },
      "DecisionRef": {
        "type": "object",
        "required": ["id", "title", "status"],
        "properties": {
          "id": { "type": "string" },
          "title": { "type": "string" },
          "status": { "type": "string" }
        }
      },
      "EntityObservationsResponse": {
        "type": "object",
        "required": ["observations", "count"],
        "properties": {
          "observations": { "type": "array", "items": { "$ref": "#/components/schemas/ObservationRef" } },
          "count": { "type": "integer", "format": "uint", "minimum": 0 }
        }
      },
      "ObservationSummary": {
        "type": "object",
        "required": ["id", "text"],
        "properties": {
          "id": { "type": "string" },
          "text": { "type": "string" },
          "created_at": { "type": ["string", "null"] }
        }
      },
      "ObservationListResponse": {
        "type": "object",
        "required": ["observations", "count", "total"],
        "properties": {
          "observations": {
            "type": "array",
            "items": { "$ref": "#/components/schemas/ObservationSummary" }
          },
          "count": { "type": "integer", "format": "uint", "minimum": 0 },
          "total": { "type": "integer", "format": "uint", "minimum": 0 }
        }
      },
      "ObservationDetail": {
        "type": "object",
        "required": ["id", "content", "entity_refs"],
        "properties": {
          "id": { "type": "string" },
          "content": { "type": "string" },
          "created_at": { "type": ["string", "null"] },
          "entity_refs": {
            "type": "array",
            "items": { "type": "string" }
          }
        }
      },
      "RebuildResponse": {
        "type": "object",
        "required": ["status", "message"],
        "properties": {
          "status": { "type": "string" },
          "message": { "type": "string" }
        }
      },
      "AdminStatusResponse": {
        "type": "object",
        "required": ["rebuild_in_progress", "lexical_docs", "semantic_vectors", "indexes_available"],
        "properties": {
          "rebuild_in_progress": { "type": "boolean" },
          "lexical_docs": { "type": "integer", "format": "uint64", "minimum": 0 },
          "semantic_vectors": { "type": "integer", "format": "uint", "minimum": 0 },
          "last_rebuild_at_ms": { "type": ["integer", "null"], "format": "uint64" },
          "indexes_available": { "type": "boolean" }
        }
      },
      "CodeRepository": {
        "type": "object",
        "required": ["name"],
        "properties": {
          "name": { "type": "string" },
          "url": { "type": ["string", "null"] },
          "description": { "type": ["string", "null"] }
        }
      },
      "CodeFile": {
        "type": "object",
        "required": ["path"],
        "properties": {
          "path": { "type": "string" },
          "repository": { "type": ["string", "null"] },
          "language": { "type": ["string", "null"] }
        }
      },
      "CodeSymbol": {
        "type": "object",
        "required": ["name", "kind", "file_path", "line"],
        "properties": {
          "name": { "type": "string" },
          "kind": { "type": "string" },
          "file_path": { "type": "string" },
          "line": { "type": "integer", "format": "uint32", "minimum": 0 }
        }
      },
      "CodeResponse": {
        "type": "object",
        "required": ["files", "symbols", "repositories"],
        "properties": {
          "files": {
            "type": "array",
            "items": { "$ref": "#/components/schemas/CodeFile" }
          },
          "symbols": {
            "type": "array",
            "items": { "$ref": "#/components/schemas/CodeSymbol" }
          },
          "repositories": {
            "type": "array",
            "items": { "$ref": "#/components/schemas/CodeRepository" }
          }
        }
      }
    }
  }
}
"##;

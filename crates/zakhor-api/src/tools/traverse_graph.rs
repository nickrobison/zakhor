use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use std::collections::HashSet;
use std::time::Instant;
use tracing::info_span;
use tracker::prelude::SparqlCursorExtManual;

use crate::args::{TraverseGraphArgs, TraverseGraphResponse, TripleResult};
use crate::handler::{args_hash, is_resource_iri, query_depth1, MemoryHandler};

#[tool_router(router = tool_router_traverse_graph, vis = "pub(crate)")]
impl MemoryHandler {
    #[tool(description = "Traverse the knowledge graph from a starting node")]
    async fn traverse_graph(
        &self,
        Parameters(args): Parameters<TraverseGraphArgs>,
    ) -> Result<Json<TraverseGraphResponse>, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "traverse_graph",
            correlation_id = %crate::new_correlation_id(),
            args_hash = %args_hash(&args),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let _guard = span.enter();
        let start = Instant::now();

        let result = (|| -> Result<Json<TraverseGraphResponse>, String> {
            if args.depth <= 1 {
                let sparql =
                    crate::tools::build_traverse_query(&args.start_id, args.depth, &args.edge_types);
                match self.conn.query(&sparql, None::<&gio::Cancellable>) {
                    Ok(cursor) => {
                        let mut triples: Vec<TripleResult> = Vec::new();
                        loop {
                            match cursor.next(None::<&gio::Cancellable>) {
                                Ok(true) => {
                                    let s =
                                        cursor.string(0).map(|s| s.to_string()).unwrap_or_default();
                                    let p =
                                        cursor.string(1).map(|s| s.to_string()).unwrap_or_default();
                                    let o =
                                        cursor.string(2).map(|s| s.to_string()).unwrap_or_default();
                                    triples.push(TripleResult {
                                        subject: s,
                                        predicate: p,
                                        object: o,
                                    });
                                }
                                Ok(false) => break,
                                Err(e) => return Err(format!("Cursor error: {e}")),
                            }
                        }
                        let count = triples.len() as u64;
                        Ok(Json(TraverseGraphResponse {
                            triples,
                            count,
                            warning: None,
                        }))
                    }
                    Err(e) => Ok(Json(TraverseGraphResponse {
                        triples: vec![],
                        count: 0,
                        warning: Some(format!("Query issue: {e}")),
                    })),
                }
            } else {
                let mut all_triples: Vec<TripleResult> = Vec::new();
                let mut seen_sop: HashSet<(String, String, String)> = HashSet::new();
                let mut visited_iris: HashSet<String> = HashSet::new();
                let mut frontier: Vec<String> = vec![args.start_id.clone()];
                visited_iris.insert(args.start_id.clone());

                for _ in 0..args.depth {
                    let mut next_frontier: Vec<String> = Vec::new();
                    for node in &frontier {
                        let triples = query_depth1(&self.conn, node, &args.edge_types)?;
                        for triple in &triples {
                            let key = (
                                triple.subject.clone(),
                                triple.predicate.clone(),
                                triple.object.clone(),
                            );
                            if seen_sop.insert(key) {
                                all_triples.push(triple.clone());
                            }
                            let obj_iri = is_resource_iri(&triple.object)
                                && visited_iris.insert(triple.object.clone());
                            if obj_iri {
                                next_frontier.push(triple.object.clone());
                            }
                            let subj_iri = is_resource_iri(&triple.subject)
                                && visited_iris.insert(triple.subject.clone());
                            if subj_iri {
                                next_frontier.push(triple.subject.clone());
                            }
                        }
                    }
                    frontier = next_frontier;
                    if frontier.is_empty() {
                        break;
                    }
                }

                let count = all_triples.len() as u64;
                Ok(Json(TraverseGraphResponse {
                    triples: all_triples,
                    count,
                    warning: None,
                }))
            }
        })();

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", if result.is_ok() { "success" } else { "error" });
        span.record("duration_ms", duration_ms);
        result
    }
}

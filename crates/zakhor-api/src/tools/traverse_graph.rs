use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};
use std::time::Instant;
use tracing::info_span;
use tracker::prelude::SparqlCursorExtManual;

use crate::args::{TraverseGraphArgs, TraverseGraphResponse, TripleResult};
use crate::handler::{MAX_TRAVERSE_TRIPLES, MemoryHandler, args_hash, traverse_bfs};

#[tool_router(router = tool_router_traverse_graph, vis = "pub(crate)")]
impl MemoryHandler {
    #[tool(
        description = "Traverse the memory graph from a starting node. `edge_types` filters by \
                       predicate IRI (pass [] for no filter), `depth` is the number of hops. \
                       Only the Zakhor memory vocabulary is expanded, so schema and ontology \
                       axioms are not walked. Results are capped; `truncated` is true when the \
                       cap was reached, meaning the result set is incomplete."
    )]
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
                let sparql = crate::tools::build_traverse_query(
                    &args.start_id,
                    args.depth,
                    &args.edge_types,
                )?;
                match self.conn.query(&sparql, None::<&gio::Cancellable>) {
                    Ok(cursor) => {
                        let mut triples: Vec<TripleResult> = Vec::new();
                        let mut truncated = false;
                        loop {
                            match cursor.next(None::<&gio::Cancellable>) {
                                Ok(true) => {
                                    if triples.len() >= MAX_TRAVERSE_TRIPLES {
                                        truncated = true;
                                        break;
                                    }
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
                            truncated,
                            warning: None,
                        }))
                    }
                    Err(e) => Ok(Json(TraverseGraphResponse {
                        triples: vec![],
                        count: 0,
                        truncated: false,
                        warning: Some(format!("Query issue: {e}")),
                    })),
                }
            } else {
                let outcome =
                    traverse_bfs(&self.conn, &args.start_id, args.depth, &args.edge_types)?;
                let count = outcome.triples.len() as u64;
                Ok(Json(TraverseGraphResponse {
                    triples: outcome.triples,
                    count,
                    truncated: outcome.truncated,
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

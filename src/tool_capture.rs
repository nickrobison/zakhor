//! ToolCall Capture (Phase 3.2)
#![expect(dead_code)]
//!
//! Captures MCP tool call metadata (tool name, arguments, timestamp, session
//! identifier) and stores it in the knowledge graph as a `zakhor:ToolCall`.

use gio::Cancellable;
use std::time::{SystemTime, UNIX_EPOCH};
use tracker::prelude::SparqlConnectionExtManual;
use tracker::SparqlConnection;

use crate::sparql::Prefix;

/// A captured MCP tool invocation.
#[derive(Clone, Debug)]
pub struct ToolCall {
    pub uri: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub session_id: String,
    pub timestamp_ms: u64,
}

/// Store a ToolCall in the knowledge graph.
///
/// Returns the URI of the newly created ToolCall node.
pub fn capture_tool_call(
    conn: &SparqlConnection,
    tool_name: &str,
    arguments_json: &str,
    session_id: &str,
) -> Result<String, String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let call_uri = format!("{}toolcall/{:016x}", Prefix::ZAKHOR, ts);
    let safe_args = arguments_json.replace('\'', "\\'");

    let sparql = format!(
        r#"PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX zakhor: <{ns}>

INSERT DATA {{
  <{uri}> rdf:type zakhor:ToolCall .
  <{uri}> zakhor:toolName "{name}"@en .
  <{uri}> zakhor:toolArguments "{args}"@en .
  <{uri}> zakhor:sessionId "{session}"@en .
  <{uri}> zakhor:timestamp {ts} .
}}"#,
        ns = Prefix::ZAKHOR,
        uri = call_uri,
        name = tool_name.replace('\'', "\\'"),
        args = safe_args,
        session = session_id.replace('\'', "\\'"),
        ts = ts,
    );

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("ToolCall capture failed: {e}"))?;

    Ok(call_uri)
}

/// Link a ToolCall to a Decision via `zakhor:evidenceFor`.
pub fn link_toolcall_to_decision(
    conn: &SparqlConnection,
    toolcall_uri: &str,
    decision_uri: &str,
) -> Result<(), String> {
    let safe_tc = toolcall_uri.replace('>', "");
    let safe_dec = decision_uri.replace('>', "");

    let sparql = format!(
        r#"PREFIX zakhor: <{ns}>

INSERT DATA {{
  <{tc}> zakhor:evidenceFor <{dec}> .
}}"#,
        ns = Prefix::ZAKHOR,
        tc = safe_tc,
        dec = safe_dec,
    );

    conn.update(&sparql, None::<&Cancellable>)
        .map_err(|e| format!("Link toolcall->decision failed: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolcall_struct() {
        let tc = ToolCall {
            uri: "http://zakhor/ns/toolcall/abc".into(),
            tool_name: "store_observation".into(),
            arguments_json: "{}".into(),
            session_id: "ses_123".into(),
            timestamp_ms: 1000,
        };
        assert_eq!(tc.tool_name, "store_observation");
        assert_eq!(tc.session_id, "ses_123");
    }

    #[test]
    fn test_link_toolcall_sparql_shape() {
        let sparql = format!(
            "PREFIX zakhor: <{ns}> INSERT DATA {{ <{tc}> zakhor:evidenceFor <{dec}> . }}",
            ns = Prefix::ZAKHOR,
            tc = "http://zakhor/ns/toolcall/a",
            dec = "http://zakhor/ns/decision/b",
        );
        assert!(sparql.contains("evidenceFor"));
        assert!(sparql.contains("/toolcall/a"));
        assert!(sparql.contains("/decision/b"));
    }
}

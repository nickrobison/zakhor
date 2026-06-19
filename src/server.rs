use crate::config::Config;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Instant;
use tracing::info_span;

const MAX_INPUT_LEN: usize = 10_240;

fn validate_not_empty(s: &str, name: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err(format!("'{}' must not be empty", name));
    }
    if s.len() > MAX_INPUT_LEN {
        return Err(format!(
            "'{}' too long ({} bytes, max {})",
            name,
            s.len(),
            MAX_INPUT_LEN
        ));
    }
    Ok(())
}

fn args_hash<T: Serialize>(args: &T) -> String {
    let json = serde_json::to_string(args).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Clone)]
pub struct MemoryHandler {
    conn: tracker::SparqlConnection,
}

impl MemoryHandler {
    pub fn new() -> Self {
        let conn = crate::tracker_db::init_db("./zakhor-db");
        Self { conn }
    }

    pub fn new_with_config(cfg: &Config) -> Self {
        let db_path = cfg.database.path.to_str().unwrap_or("./zakhor-db");
        let conn = crate::tracker_db::init_db(db_path);
        Self { conn }
    }
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct StoreMemoryArgs {
    text: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ReadMemoryArgs {
    id: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct UpdateMemoryArgs {
    id: String,
    text: String,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct DeleteMemoryArgs {
    id: String,
}

#[tool_router(server_handler)]
impl MemoryHandler {
    #[tool(
        name = "store_memory",
        description = "Store knowledge in GNOME Tracker"
    )]
    async fn store_memory(&self, args: Parameters<StoreMemoryArgs>) -> Result<String, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "store_memory",
            correlation_id = &crate::new_correlation_id(),
            args_hash = %args_hash(&args.0),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let start = Instant::now();

        let text = args.0.text;
        validate_not_empty(&text, "text")?;

        let propagate_span = span.clone();
        let this = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = propagate_span.enter();
            crate::tracker_db::store_memory(&this.conn, &text)
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", "success");
        span.record("duration_ms", &duration_ms);
        result
    }

    #[tool(
        name = "read_memory",
        description = "Read knowledge from GNOME Tracker by ID"
    )]
    async fn read_memory(&self, args: Parameters<ReadMemoryArgs>) -> Result<String, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "read_memory",
            correlation_id = &crate::new_correlation_id(),
            args_hash = %args_hash(&args.0),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let start = Instant::now();

        let id = args.0.id;
        validate_not_empty(&id, "id")?;

        let propagate_span = span.clone();
        let this = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = propagate_span.enter();
            crate::tracker_db::read_memory(&this.conn, &id)
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", "success");
        span.record("duration_ms", &duration_ms);
        result
    }

    #[tool(
        name = "update_memory",
        description = "Update existing knowledge in GNOME Tracker"
    )]
    async fn update_memory(&self, args: Parameters<UpdateMemoryArgs>) -> Result<String, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "update_memory",
            correlation_id = &crate::new_correlation_id(),
            args_hash = %args_hash(&args.0),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let start = Instant::now();

        let id = args.0.id;
        let text = args.0.text;
        validate_not_empty(&id, "id")?;
        validate_not_empty(&text, "text")?;

        let propagate_span = span.clone();
        let this = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = propagate_span.enter();
            crate::tracker_db::update_memory(&this.conn, &id, &text)
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", "success");
        span.record("duration_ms", &duration_ms);
        result
    }

    #[tool(
        name = "delete_memory",
        description = "Delete knowledge from GNOME Tracker by ID"
    )]
    async fn delete_memory(&self, args: Parameters<DeleteMemoryArgs>) -> Result<String, String> {
        let span = info_span!(
            "mcp_tool",
            tool = "delete_memory",
            correlation_id = &crate::new_correlation_id(),
            args_hash = %args_hash(&args.0),
            duration_ms = tracing::field::Empty,
            result = tracing::field::Empty,
        );
        let start = Instant::now();

        let id = args.0.id;
        validate_not_empty(&id, "id")?;

        let propagate_span = span.clone();
        let this = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _guard = propagate_span.enter();
            crate::tracker_db::delete_memory(&this.conn, &id)?;
            Ok(format!("Deleted: {}", id))
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        span.record("result", "success");
        span.record("duration_ms", &duration_ms);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_hash_deterministic() {
        let args = StoreMemoryArgs {
            text: "hello".into(),
        };
        let h1 = args_hash(&args);
        let h2 = args_hash(&args);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16, "hash should be 16 hex chars");
    }

    #[test]
    fn test_args_hash_different_args_differ() {
        let a = StoreMemoryArgs { text: "foo".into() };
        let b = StoreMemoryArgs { text: "bar".into() };
        assert_ne!(args_hash(&a), args_hash(&b));
    }

    #[test]
    fn test_args_hash_different_types_differ() {
        let store = StoreMemoryArgs { text: "x".into() };
        let read = ReadMemoryArgs { id: "x".into() };
        assert_ne!(args_hash(&store), args_hash(&read));
    }

    #[test]
    fn test_correlation_id_unique() {
        let id1 = crate::new_correlation_id();
        let id2 = crate::new_correlation_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("mcp-"));
        assert!(id2.starts_with("mcp-"));
    }

    #[test]
    fn test_correlation_id_monotonic() {
        let id1 = crate::new_correlation_id();
        let id2 = crate::new_correlation_id();
        assert!(
            id1 < id2,
            "correlation IDs should increase: {} < {}",
            id1,
            id2
        );
    }

    #[test]
    fn test_mcp_tool_span_has_required_fields() {
        use std::sync::{Arc, Mutex};
        use tracing::span::{Attributes, Id};
        use tracing::subscriber::with_default;
        use tracing::{Event, Metadata, Subscriber};

        #[derive(Default, Clone)]
        struct CaptureSub {
            new_span_fields: Arc<Mutex<Vec<String>>>,
            recorded_fields: Arc<Mutex<Vec<String>>>,
        }

        impl Subscriber for CaptureSub {
            fn enabled(&self, _: &Metadata<'_>) -> bool {
                true
            }

            fn new_span(&self, attrs: &Attributes<'_>) -> Id {
                let mut fields = self.new_span_fields.lock().unwrap();
                let mut visitor = FieldCapture(&mut fields);
                attrs.record(&mut visitor);
                Id::from_u64(1)
            }

            fn record(&self, _: &Id, record: &tracing::span::Record<'_>) {
                let mut fields = self.recorded_fields.lock().unwrap();
                let mut visitor = FieldCapture(&mut fields);
                record.record(&mut visitor);
            }

            fn record_follows_from(&self, _: &Id, _: &Id) {}
            fn event(&self, _: &Event<'_>) {}
            fn enter(&self, _: &Id) {}
            fn exit(&self, _: &Id) {}
        }

        struct FieldCapture<'a>(&'a mut Vec<String>);

        impl tracing::field::Visit for FieldCapture<'_> {
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                self.0.push(format!("{}={}", field.name(), value));
            }
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                self.0.push(format!("{}={:?}", field.name(), value));
            }
            fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
                self.0.push(format!("{}={:?}", field.name(), value));
            }
            fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
                self.0.push(format!("{}={}", field.name(), value));
            }
            fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
                self.0.push(format!("{}={}", field.name(), value));
            }
            fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
                self.0.push(format!("{}={}", field.name(), value));
            }
        }

        let sub = CaptureSub::default();
        let create_fields = sub.new_span_fields.clone();
        let rec_fields = sub.recorded_fields.clone();

        with_default(sub, || {
            let span = info_span!(
                "mcp_tool",
                tool = "test_tool",
                correlation_id = "mcp-000001",
                args_hash = "abc123",
                duration_ms = tracing::field::Empty,
                result = tracing::field::Empty,
            );
            // Exercise late-binding field recording (the real tool
            // methods call span.record for duration_ms and result).
            span.record("duration_ms", &1.5f64);
            span.record("result", "success");
        });

        let created = create_fields.lock().unwrap();
        let created_all = created.join(" | ");
        assert!(
            created_all.contains("tool=test_tool"),
            "should contain tool field at create: {}",
            created_all
        );
        assert!(
            created_all.contains("correlation_id=mcp-000001"),
            "should contain correlation_id at create: {}",
            created_all
        );
        assert!(
            created_all.contains("args_hash=abc123"),
            "should contain args_hash at create: {}",
            created_all
        );

        let recorded = rec_fields.lock().unwrap();
        let recorded_all = recorded.join(" | ");
        assert!(
            recorded_all.contains("duration_ms=1.5"),
            "should record duration_ms: {}",
            recorded_all
        );
        assert!(
            recorded_all.contains("result=success"),
            "should record result: {}",
            recorded_all
        );
    }
}

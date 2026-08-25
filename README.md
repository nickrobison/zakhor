# Zakhor

MCP (Model Context Protocol) server for persistent memory backed by GNOME Tracker SPARQL storage.

## Overview

Zakhor exposes a Tracker SPARQL database through the MCP protocol, giving AI agents
persistent read/write access to knowledge stored in Tracker's RDF store. Each memory
is stored as a [NIE](https://en.wikipedia.org/wiki/Nepomuk) information element with
plain-text content.

## Prerequisites

- Rust 1.85+ (2024 edition)
- GNOME Tracker 3 (`tracker3`) — typically pre-installed on GNOME desktops
- Running Tracker endpoint (`tracker3 endpoint`) on the same machine
  - Zakhor reads `TRACKER_ENDPOINT` env var or defaults to `http://127.0.0.1:7878`

## Usage

```bash
# Start with default DB path (./zakhor-db/) over stdio:
cargo run

# Start with a specific DB path:
cargo run -- --db-path /path/to/db

# Start with a custom config file:
cargo run -- --config /path/to/zakhor.toml

# Or set via env var:
ZAKHOR_DB_PATH=/path/to/db cargo run

# Start over MCP Streamable HTTP/SSE:
cargo run -- --http
```

By default, stdio mode listens on stdin/stdout. Use `--http` to expose both
MCP tools and REST API endpoints on a single HTTP server at `http://127.0.0.1:3000`.

HTTP configuration can be overridden with environment variables:

- `ZAKHOR_HTTP_HOST` — bind host, default `127.0.0.1`
- `ZAKHOR_HTTP_PORT` — bind port, default `3000`

Example:

```bash
ZAKHOR_HTTP_HOST=0.0.0.0 ZAKHOR_HTTP_PORT=4000 cargo run -- --http
```

Once running in HTTP mode, the server exposes:
- MCP tools at the root (`/`) via Streamable HTTP/SSE
- REST API at `/api/v1/` (with OpenAPI docs at `/api/v1/docs`)

Once running, the MCP server listens on stdin/stdout or HTTP/SSE — connect any
MCP-compatible host (Claude Desktop, OpenCode, etc.) to use the tools.

## Configuration

Zakhor discovers its TOML config file with the following precedence (first match wins):

1. `-c/--config PATH` — explicit path. Required to exist; a missing explicit path is an error.
2. `./zakhor.toml` — working-directory config (legacy). Existing local setups keep working because the working directory is checked before XDG.
3. `$XDG_CONFIG_HOME/zakhor/zakhor.toml` (default `~/.config/zakhor/zakhor.toml`) — user config.
4. No file found — built-in defaults plus `ZAKHOR_*` environment variables are used.

### Model Cache

FastEmbed (embeddings) and GLiNER (extraction) share a single model cache directory so
neither re-downloads its ONNX model when the other's cache moves:

- Default: `$XDG_CACHE_HOME/zakhor/models` (i.e. `~/.cache/zakhor/models`).
- Override via `[models] cache_dir = "..."` in the config file, or the
  `ZAKHOR_MODELS_CACHE_DIR` environment variable.
- GLiNER-specific legacy knobs still work at lower precedence:
  `[extraction] model_dir` beats `[models].cache_dir`, and the `HF_HUB_CACHE`
  environment variable is still honored as a further fallback.

### One-Time Automatic Migration

On startup, Zakhor best-effort migrates legacy model cache locations into the shared
cache directory above:

- `<database.path>/semantic/fastembed-cache/*` (pre-unification FastEmbed cache)
- `./.fastembed_cache/*` (fastembed-rs default)
- stray `./models--*` directories in the working directory (hf-hub content-addressed
  dirs leaked by GLiNER's working-directory fallback)

Migration is never fatal. Items already present at the destination are skipped and
logged. A failure during one entry does not abort migration of the others.

### Environment Variables

| Variable | Effect |
|----------|--------|
| `ZAKHOR_DB_PATH` | Override the Tracker DB path |
| `ZAKHOR_HTTP_HOST` | HTTP bind host (default `127.0.0.1`) |
| `ZAKHOR_HTTP_PORT` | HTTP bind port (default `3000`) |
| `ZAKHOR_MODELS_CACHE_DIR` | Override the shared model cache directory (same as `[models].cache_dir`) |

The SQLite/graph database path (`[database] path`, default `./zakhor-db`) intentionally
stays where it is and is not moved under XDG.

### MCP Tools

| Tool | Args | Description |
|------|------|-------------|
| `store_observation` | `content`, `created_at`, `metadata` | Store an observation with optional structured metadata |
| `extract_and_store` | `uri`, `text` | Auto-extract entities and relations from text (GLiNER) and store them |
| `query_entities` | `pattern`, `limit` | Query entities by label pattern in the knowledge graph |
| `traverse_graph` | `uri`, `limit` | Traverse outgoing RDF edges from an entity |
| `search_hybrid` | `query`, `limit` | Hybrid lexical/semantic search using RRF fusion |
| `record_decision` | `context`, `decision`, `alternatives`, `rationale`, `project_uri?` | Record a decision with context and rationale, optionally linked to a project |
| `rebuild_indexes` | none | Rebuild all search indexes from Tracker |
| `create_project` | `name`, `description?` | Create a project node and return its URI |
| `link_to_project` | `entity_uri`, `project_uri` | Link an entity or decision to a project (`zakhor:belongsToProject`) |
| `create_repository` | `name`, `description?` | Create a repository node and return its URI |
| `link_to_repository` | `entity_uri`, `repository_uri` | Link an entity to a repository (`zakhor:belongsToRepository`) |

Admin tools (`admin_rebuild_indexes`, `admin_inject_tool_call`) are registered
only when the server runs with `--ephemeral`.

## Architecture

```
┌────────────────────┐     MCP stdio or Streamable HTTP/SSE     ┌──────────────┐
│  MCP Host          │ ◄──────────────────────────────────────► │   Zakhor     │
│  (Claude, OpenCode)│                                           │  (rmcp)      │
└────────────────────┘                                           └──────┬───────┘
                                                                        │
                                                               spawn_blocking
                                                                        │
                                                                 ┌──────┴───────┐
                                                                 │ tracker-rs   │
                                                                 │ (SPARQL FFI) │
                                                                 └──────┬───────┘
                                                                        │
                                                                 ┌──────┴───────┐
                                                                 │ GNOME Tracker│
                                                                 │  SPARQL DB   │
                                                                 └──────────────┘
```

## Project Structure

```
crates/
├── zakhor-common/    — Shared config, error types, vocab/URI constants
├── zakhor-storage/   — SPARQL CRUD, schema, tracker_db (GNOME Tracker FFI)
├── zakhor-search/    — Lexical (Tantivy) + semantic (fastembed) search, index sync
├── zakhor-model/     — Ingestion pipeline, extraction (GLiNER), decision model, ranking
├── zakhor-code/      — Code indexing (symbol extraction, container tracking)
└── zakhor-api/       — MCP server, HTTP REST API, tool handlers
src/main.rs           — Root binary entry point (CLI args, tracing init)
```

## Development

```bash
cargo check        # Static analysis
cargo test         # Run unit tests
cargo clippy       # Lint
cargo build        # Release build: cargo build --release
```

### Integration Tests

Python integration tests are in [`tests/python/`](tests/python/) and exercise the full
MCP tool surface against a live Zakhor server. They require a debug build, Python 3.12+,
and the `uv` package manager.

```bash
# Prerequisite: compile the debug binary
cargo build

# Run full integration test suite (via Make):
make test-integration

# Or run directly:
./tests/python/run_tests.sh

# Filter by test keyword:
./tests/python/run_tests.sh traverse

# Pass extra arguments to pytest:
./tests/python/run_tests.sh -- -x --tb=short
```

Each integration test starts an ephemeral Zakhor server on a random port, runs
assertions over MCP and SPARQL, and tears down the server afterward. The suite
requires a running Tracker SPARQL endpoint (`tracker3 endpoint` or `tinysparql`).

See [`tests/python/pyproject.toml`](tests/python/pyproject.toml) for test configuration and dependencies.

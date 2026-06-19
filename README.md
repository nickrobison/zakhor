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

# Or set via env var:
ZAKHOR_DB_PATH=/path/to/db cargo run

# Start over MCP Streamable HTTP/SSE:
cargo run -- --http
```

By default, stdio mode listens on stdin/stdout. Use `--http` to expose the same
MCP tools over Streamable HTTP/SSE at `http://127.0.0.1:3000`.

HTTP configuration can be overridden with environment variables:

- `ZAKHOR_HTTP_HOST` — bind host, default `127.0.0.1`
- `ZAKHOR_HTTP_PORT` — bind port, default `3000`

Example:

```bash
ZAKHOR_HTTP_HOST=0.0.0.0 ZAKHOR_HTTP_PORT=4000 cargo run -- --http
```

Once running, the MCP server listens on stdin/stdout or HTTP/SSE — connect any
MCP-compatible host (Claude Desktop, OpenCode, etc.) to use the tools.

### MCP Tools

| Tool | Args | Description |
|------|------|-------------|
| `store_observation` | `content`, `created_at`, `metadata` | Store an observation with optional structured metadata |
| `query_entities` | `pattern`, `limit` | Query entities by label pattern in the knowledge graph |
| `traverse_graph` | `uri`, `limit` | Traverse outgoing RDF edges from an entity |
| `search_hybrid` | `query`, `limit` | Hybrid lexical/semantic search using RRF fusion |
| `record_decision` | `context`, `decision`, `alternatives`, `rationale` | Record a decision with context and rationale |
| `rebuild_indexes` | none | Rebuild all search indexes from Tracker |

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
src/
├── main.rs         — Entry point, tracing init, CLI arg parsing
├── server.rs       — MCP tool handler (rmcp router)
├── tracker_db.rs   — SPARQL CRUD operations
├── config.rs       — Config struct with TOML + env var support
└── error.rs        — ZakhorError type, Display/Error impls, retry logic
```

## Development

```bash
cargo check        # Static analysis
cargo test         # Run unit tests
cargo clippy       # Lint
cargo build        # Release build: cargo build --release
```

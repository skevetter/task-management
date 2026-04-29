# MCP Server Architecture Design — task-management CLI

## Status
Draft

## Context
Agents cannot reliably consume the task-management CLI's human-readable output, and concurrent multi-agent access to the SQLite database risks corruption. An MCP (Model Context Protocol) server interface serializes access through a single process, returns structured JSON natively, and lets agents call tools without shelling out to a CLI binary.

## Decision
Add an MCP server mode to the existing `task-management` binary, invoked via `task-management serve`. The server uses **stdio transport** (JSON-RPC over stdin/stdout), delegates all operations to the existing `db::Database` layer, and is built on the **`rmcp` crate** (the official Rust MCP SDK). No separate binary — the MCP server is a new subcommand on the same Cargo target.

---

## 1. MCP Tool Surface

Nine tools, one per CLI operation. Each tool receives typed JSON parameters and returns a JSON content block.

### 1.1 create_task

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `title` | `string` | yes | — |
| `description` | `string` | no | `null` |
| `priority` | `enum("low","medium","high","critical")` | no | `"medium"` |
| `assignee` | `string` | no | `null` |
| `tags` | `string[]` | no | `[]` |
| `parent` | `string` | no | `null` |
| `namespace` | `string` | no | server default |
| `actor` | `string` | no | MCP client identity |

**Output**: Full `Task` object (id, title, description, status, priority, assignee, tags, parent_task_id, created_at, updated_at).

**Errors**: Database write failure, invalid parent ID.

### 1.2 update_task

| Field | Type | Required |
|-------|------|----------|
| `id` | `string` | yes (full UUID or 4+ char prefix) |
| `title` | `string` | no |
| `description` | `string` | no |
| `status` | `enum("open","in-progress","blocked","done","closed")` | no |
| `priority` | `enum("low","medium","high","critical")` | no |
| `assignee` | `string` | no |
| `tags` | `string[]` | no |
| `namespace` | `string` | no |
| `actor` | `string` | no |

**Output**: Updated `Task` object.

**Errors**: Task not found, ambiguous ID prefix, no fields provided.

### 1.3 close_task

| Field | Type | Required |
|-------|------|----------|
| `id` | `string` | yes |
| `namespace` | `string` | no |
| `actor` | `string` | no |

**Output**: Closed `Task` object (status = "closed").

**Errors**: Task not found, ambiguous ID prefix.

### 1.4 list_tasks

| Field | Type | Required |
|-------|------|----------|
| `status` | `enum("open","in-progress","blocked","done","closed")` | no |
| `assignee` | `string` | no |
| `priority` | `enum("low","medium","high","critical")` | no |
| `tag` | `string` | no |
| `parent` | `string` | no |
| `blocked_by` | `string` | no |
| `blocks` | `string` | no |
| `namespace` | `string` | no |

**Output**: Array of `Task` objects.

**Errors**: Database read failure.

### 1.5 show_task

| Field | Type | Required |
|-------|------|----------|
| `id` | `string` | yes |
| `namespace` | `string` | no |

**Output**: `TaskDetail` object — task fields flattened, plus `notes: TaskNote[]`, `timeline: TimelineEvent[]`, `links: TaskLink[]`.

**Errors**: Task not found, ambiguous ID prefix.

### 1.6 add_note

| Field | Type | Required |
|-------|------|----------|
| `id` | `string` | yes |
| `message` | `string` | yes |
| `author` | `string` | no |
| `namespace` | `string` | no |

**Output**: `TaskNote` object (id, task_id, body, author, created_at).

**Errors**: Task not found, ambiguous ID prefix.

### 1.7 task_history

| Field | Type | Required |
|-------|------|----------|
| `id` | `string` | yes |
| `namespace` | `string` | no |

**Output**: Array of `TimelineEvent` objects.

**Errors**: Task not found, ambiguous ID prefix.

### 1.8 link_tasks

| Field | Type | Required |
|-------|------|----------|
| `source_id` | `string` | yes |
| `relationship` | `enum("parent","child","blocked_by","blocks","related_to")` | yes |
| `target_id` | `string` | yes |
| `namespace` | `string` | no |

**Output**: `TaskLink` object (link_id, relationship, related_task_id, related_task_title).

**Errors**: Source or target task not found, ambiguous ID prefix.

### 1.9 list_links

| Field | Type | Required |
|-------|------|----------|
| `id` | `string` | yes |
| `namespace` | `string` | no |

**Output**: Array of `TaskLink` objects.

**Errors**: Task not found (via prefix resolution), database read failure.

---

## 2. Transport Choice

| Transport | Pros | Cons |
|-----------|------|------|
| **stdio** | Zero network config, works inside any agent sandbox, Claude Code / Cursor native support, single process owns the DB | No remote access, one client per server process |
| SSE | Multiple clients, push notifications | Requires HTTP listener, port management, CORS, more complex |
| Streamable HTTP | Latest MCP spec transport, stateless | Same HTTP complexity as SSE, overkill for local agent use |

**Decision: stdio.** The primary use case is a local agent (Claude Code, Cursor, Copilot) connecting to a task database on the same machine. stdio gives zero-config setup — the agent launches `task-management serve` as a child process and communicates over stdin/stdout. One server process per database file serializes all writes through a single SQLite connection, eliminating concurrent-write concerns entirely.

If remote or multi-client access is needed later, streamable HTTP can be added as a second transport behind a feature flag without changing the tool handler layer.

---

## 3. Architecture

### 3.1 Request Flow

```
MCP Client (Claude Code / Cursor / etc.)
    │
    │  JSON-RPC over stdin/stdout
    ▼
task-management serve
    │
    │  rmcp ServerHandler trait
    ▼
TaskMcpServer (src/mcp/server.rs)
    │
    │  Direct function calls
    ▼
db::Database (src/db.rs)
    │
    │  rusqlite
    ▼
SQLite file (WAL mode)
```

### 3.2 Component Responsibilities

| Component | File | Responsibility |
|-----------|------|---------------|
| CLI entry point | `src/main.rs` | Parse `serve` subcommand, construct `Database`, launch MCP server |
| MCP server | `src/mcp/server.rs` | Implement `rmcp::ServerHandler`, register tools, dispatch to `Database` |
| Tool parameters | `src/mcp/tools.rs` | Define typed parameter structs with `serde::Deserialize` + `schemars::JsonSchema` |
| Database layer | `src/db.rs` | Unchanged — all CRUD operations already exist |
| Models | `src/models.rs` | Unchanged — already has `Serialize` on all types |
| Module root | `src/mcp/mod.rs` | Re-export `server` and `tools` |

### 3.3 How MCP Tool Handlers Call db.rs

Each `#[tool]` method on `TaskMcpServer` holds a reference to `Database` and calls the existing public methods directly:

```rust
// src/mcp/server.rs (illustrative)

use rmcp::prelude::*;

pub struct TaskMcpServer {
    db: Database,
}

#[tool_router]
impl TaskMcpServer {
    #[tool(description = "Create a new task")]
    async fn create_task(&self, params: CreateTaskParams) -> Result<CallToolResult, McpError> {
        let task = self.db.create_task(
            &params.title,
            params.description.as_deref(),
            params.priority.unwrap_or(TaskPriority::Medium),
            params.assignee.as_deref(),
            &params.tags.unwrap_or_default(),
            params.parent.as_deref(),
        ).map_err(|e| McpError::internal(e.to_string()))?;

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string(&task).unwrap())
        ]))
    }

    // ... same pattern for all 9 tools
}
```

> **Note**: The rmcp API shown above is illustrative. Verify actual API surface against docs.rs/rmcp during implementation.

The `Database` struct uses synchronous `rusqlite`. Since rmcp tool handlers are async, each handler calls `Database` methods synchronously (they are fast in-process SQLite calls — no I/O wait). If needed, wrap in `tokio::task::spawn_blocking`, but SQLite local-file operations complete in microseconds and blocking the tokio thread briefly is acceptable for this workload. If profiling reveals P99 latency > 10ms for any operation, wrap the Database call in `tokio::task::spawn_blocking`.

### 3.4 Server Startup (in main.rs)

```rust
// Added to the Commands enum:
Serve {
    #[arg(long, default_value = "stdio")]
    transport: String,
}

// In match cli.command:
Commands::Serve { transport } => {
    if transport != "stdio" {
        eprintln!("Only stdio transport is supported");
        std::process::exit(1);
    }
    let server = TaskMcpServer::new(db);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let transport = rmcp::transport::io::stdio();
        server.serve(transport).await.unwrap();
    });
}
```

---

## 4. Rust MCP SDK Selection

| Crate | Version | Downloads | MCP Spec | Official | Verdict |
|-------|---------|-----------|----------|----------|---------|
| **rmcp** | 1.5.0 | 8,660,007 | 2025-11-25 | Yes (modelcontextprotocol org) | **Selected** |
| rust-mcp-sdk | 0.9.0 | 117,654 | 2025-11-25 | No | Too few users |
| mcp-attr | 0.0.7 | 6,565 | Unknown | No | Pre-alpha |
| turul-mcp-server | 0.3.37 | 1,818 | Unknown | No | Niche |

**Why rmcp**: Official Rust MCP SDK under `modelcontextprotocol/rust-sdk`. 73x more downloads than the next competitor. 3,347 GitHub stars, 505 forks. Active development (v1.5.0 released April 16, 2026). Supports stdio via `transport-io` feature. Provides `#[tool]` and `#[tool_router]` proc macros for ergonomic tool registration. Async tokio-based. Rust Edition 2024 (our toolchain is rustc 1.94.1, compatible).

**Required features**: `["server", "transport-io"]` — minimal footprint, no SSE/HTTP/auth dependencies.

---

## 5. Namespace/Org Scoping

### Current Behavior
The CLI has no `--namespace` flag today. All tasks live in a single flat space within one SQLite file.

### MCP Server Approach
Each tool accepts an optional `namespace` parameter. The server resolves it in priority order:

1. **Tool parameter** `namespace` — explicit per-request override
2. **Server config default** — set via `--namespace` flag on `task-management serve`
3. **None** — all tasks visible (current behavior)

Namespace filtering appends `AND namespace = ?` to queries in `db.rs`. This requires a schema migration to add a `namespace TEXT` column to the `tasks` table (nullable, default `NULL` = global).

### Migration Path
1. Add `namespace` column to tasks table (schema version 3)
2. Existing tasks get `NULL` namespace (global, visible to all)
3. `list_tasks` with no namespace filter returns all tasks (backward-compatible)
4. `list_tasks` with namespace filter returns only matching tasks

This is a **separate initiative** — the MCP server ships first with namespace as a pass-through parameter that is stored but not enforced, then a follow-up adds server-side filtering.

---

## 6. Actor Tracking

### Current Behavior
The CLI accepts `--actor` on `update` and `close` subcommands (`src/main.rs:70-83`). The actor value is passed to `insert_timeline_event` and stored in the `timeline_events.actor` column.

### MCP Server Approach
Every tool that mutates state (`create_task`, `update_task`, `close_task`, `add_note`, `link_tasks`) accepts an optional `actor` parameter. Resolution order:

1. **Tool parameter** `actor` — explicit per-call identity
2. **MCP client metadata** — if the MCP client sends `clientInfo.name` during initialization, use that as the default actor
3. **None** — actor is `NULL` in timeline events

The MCP server captures `clientInfo` from the MCP `initialize` handshake and stores it on the `TaskMcpServer` struct. Tool handlers use `params.actor.unwrap_or(self.default_actor.clone())`.

### What Changes in db.rs
The `create_task` function currently does not accept an `actor` parameter — it passes `None` to `insert_timeline_event`. Update the signature to: `pub fn create_task(&self, title: &str, description: Option<&str>, priority: TaskPriority, assignee: Option<&str>, tags: &[String], parent_task_id: Option<&str>, actor: Option<&str>) -> Result<Task>` with actor as the last parameter. Same for `add_note` (which currently accepts `author` — the MCP tool maps `actor` to `author`).

---

## 7. Concurrency Model

### Current State
`Database::open` in `src/db.rs:12-105` enables WAL mode (added in v0.2.1). The `Database` struct holds a single `rusqlite::Connection`. Write operations use `BEGIN IMMEDIATE` transactions (`create_task` at line 120, `add_note` at line 448, `create_link` at line 547, `remove_link` at line 583).

### stdio Serialization
With stdio transport, the MCP server is a single process with a single `Database` instance. rmcp processes JSON-RPC messages sequentially from stdin — there is no multiplexing or concurrent dispatch on stdio. Each tool call completes before the next is read.

This means:
- **No concurrent SQLite access** — one tool call at a time, one connection, one thread
- **No locking needed** — `BEGIN IMMEDIATE` is sufficient (and technically unnecessary with serial access, but harmless)
- **WAL mode still beneficial** — if a separate process reads the DB (e.g., CLI for ad-hoc queries), WAL allows concurrent reads without blocking the MCP server's writes

### If Concurrent Access Is Needed Later
If the server moves to HTTP transport with concurrent requests:
1. Wrap `Database` in `Arc<Mutex<Database>>` (simplest)
2. Or use `r2d2` connection pool with WAL mode (better throughput)
3. `BEGIN IMMEDIATE` already handles write serialization at the SQLite level

For stdio, none of this is needed.

---

## 8. Error Handling

### CLI Error → MCP Error Mapping

| CLI Error | Source | MCP Error Code | MCP Error Message |
|-----------|--------|---------------|-------------------|
| Task not found | `get_task` returns `None` | `-32602` (Invalid params) | `"Task not found: {id}"` |
| Ambiguous ID prefix | `resolve_short_id` matches > 1 | `-32602` (Invalid params) | `"Ambiguous prefix '{p}' matches: {ids}"` |
| Prefix too short | `resolve_short_id` prefix < 4 chars | `-32602` (Invalid params) | `"Prefix must be at least 4 characters"` |
| No task matching prefix | `resolve_short_id` matches 0 | `-32602` (Invalid params) | `"No task found matching prefix '{p}'"` |
| Database write failure | `rusqlite::Error` | `-32603` (Internal error) | `"Database error: {e}"` |
| Missing required param | serde deserialization | `-32602` (Invalid params) | `"Missing required parameter: {field}"` |
| Invalid enum value | serde deserialization | `-32602` (Invalid params) | `"Invalid value for {field}: {value}"` |

### Implementation Pattern

```rust
fn resolve_id(&self, prefix: &str) -> Result<String, McpError> {
    self.db.resolve_short_id(prefix).map_err(|e| {
        McpError::invalid_params(e)
    })
}
```

Tool handlers return `Result<CallToolResult, McpError>`. rmcp serializes `McpError` into a JSON-RPC error response automatically. For "soft" errors (task not found), return a successful `CallToolResult` with `is_error: true` and a descriptive text content block — this lets agents distinguish between protocol errors and domain errors.

---

## 9. Cargo.toml Changes

```toml
[dependencies]
# Existing (unchanged)
clap = { version = "4", features = ["derive"] }
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }

# New
rmcp = { version = "1.5", features = ["server", "transport-io"] }
tokio = { version = "1", features = ["rt", "macros", "io-std"] }
schemars = "0.8"
```

**Why each new dependency**:
- `rmcp` — MCP protocol implementation, server handler, tool macros
- `tokio` — async runtime required by rmcp. Features kept minimal: `rt` (runtime), `macros` (for `#[tokio::main]` or `Runtime::new()`), `io-std` (stdin/stdout async adapters)
- `schemars` — generates JSON Schema from Rust types, required by rmcp's `#[tool]` macro to produce `inputSchema` for each tool

---

## 10. File Structure

### New Files

| Path | Purpose |
|------|---------|
| `src/mcp/mod.rs` | Module declaration: `pub mod server; pub mod tools;` |
| `src/mcp/server.rs` | `TaskMcpServer` struct, `#[tool_router]` impl with all 9 tool handlers, server startup logic |
| `src/mcp/tools.rs` | Parameter structs for each tool (`CreateTaskParams`, `UpdateTaskParams`, etc.) with `Deserialize` + `JsonSchema` derives |

### Modified Files

| Path | Change |
|------|--------|
| `src/main.rs` | Add `mod mcp;` (or use lib.rs). Add `Serve` variant to `Commands` enum. Add match arm that constructs `TaskMcpServer` and runs the stdio event loop. |
| `src/lib.rs` | Add `pub mod mcp;` to expose MCP module for testing |
| `src/db.rs` | Add `actor: Option<&str>` parameter to `create_task` signature. No other changes needed — all other methods already exist. |
| `src/models.rs` | Add `Deserialize` derive to `TaskDetail` and `TaskLink` (currently only have `Serialize`). Add `schemars::JsonSchema` derive to enums used in tool params (`TaskStatus`, `TaskPriority`, `LinkType`). |
| `Cargo.toml` | Add `rmcp`, `tokio`, `schemars` dependencies as specified in section 9. |

### Files NOT Changed

| Path | Why |
|------|-----|
| `tests/integration_test.rs` | Existing CLI integration tests remain. New MCP tests go in a separate file. |

### New Test Files

| Path | Purpose |
|------|---------|
| `tests/mcp_test.rs` | Integration tests that start the MCP server as a child process, send JSON-RPC requests over stdin, and assert JSON-RPC responses on stdout |

---

## 11. CLI Integration

### Subcommand: `serve`

```
task-management serve [--transport stdio] [--db <path>] [--namespace <ns>]
```

- `--transport stdio` — only supported transport (default). Validates and rejects others.
- `--db <path>` — inherited global flag, same as all other subcommands.
- `--namespace <ns>` — optional default namespace for all operations on this server instance.

### Agent Configuration

Claude Code `mcp_servers` config in `.claude/settings.json`:

```json
{
  "mcpServers": {
    "task-management": {
      "command": "task-management",
      "args": ["serve"],
      "env": {}
    }
  }
}
```

Cursor `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "task-management": {
      "command": "task-management",
      "args": ["serve", "--db", "/path/to/tasks.db"]
    }
  }
}
```

### Server Metadata

The MCP server identifies itself during the `initialize` handshake:

```json
{
  "name": "task-management",
  "version": "0.3.0"
}
```

### Startup Behavior
1. Parse `serve` subcommand and flags
2. Open `Database` (same path resolution as CLI: `--db` > `XDG_DATA_HOME` > `~/.local/share`)
3. Construct `TaskMcpServer` with the database and optional default namespace
4. Create rmcp stdio transport
5. Start serving — blocks until stdin closes (client disconnects)
6. On stdin close, drop `Database` (closes SQLite connection), exit 0

---

## Alternatives Considered

| Approach | Pros | Cons | Why Not |
|----------|------|------|---------|
| Separate MCP binary (`task-management-mcp`) | Clean separation, independent release cycle | Two binaries to build/install/version, code duplication for DB setup | Unnecessary complexity — one binary with a subcommand is simpler |
| HTTP transport (SSE or streamable) | Multi-client support, remote access | Port management, auth, firewall config, CORS | Agents run locally — stdio is zero-config |
| Generic MCP framework (turul, mcp-attr) | Potentially simpler API | Tiny community, uncertain maintenance, missing features | rmcp has 73x more adoption and is the official SDK |
| Shim approach (MCP server shells out to CLI) | No code changes to existing binary | Process spawn overhead per call, stdout parsing fragile, loses type safety | Defeats the purpose — MCP exists to avoid CLI parsing |

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| rmcp API changes in next minor version | Medium | Medium — tool handler signatures change | Pin `rmcp = "1.5"` exactly, not `"1"`. Update deliberately. |
| tokio dependency bloats binary size | Low | Low — tokio is ~3MB in release builds | Use minimal features (`rt`, `macros`, `io-std`). No `full`. |
| `Database` sync methods block tokio thread | Low | Low — SQLite local ops are microsecond-scale | Monitor. If slow queries appear, wrap in `spawn_blocking`. |
| MCP spec evolves past rmcp support | Low | Medium — tools stop working with newer clients | rmcp tracks spec closely (official SDK). Update when needed. |
| Namespace migration breaks existing DBs | Low | High — data loss if migration is buggy | Namespace column is nullable with `NULL` default. Existing rows unaffected. Ship migration as schema version 3 with same pattern as existing v1→v2 migration in `db.rs:83-102`. |

---

## Migration Path

### Phase 1: MCP Server (this initiative)
- Add `serve` subcommand with stdio transport
- 9 tools mapping to existing CLI operations
- Actor tracking from tool params + MCP client metadata
- Namespace parameter accepted and stored but not enforced
- Version bump to 0.3.0

### Phase 2: Namespace Enforcement (future)
- Schema migration v3: add `namespace` column to `tasks` table
- Server-side namespace filtering on all queries
- `serve --namespace` sets the default for the server instance

### Phase 3: HTTP Transport (future, if needed)
- Add `transport-streamable-http-server` feature to rmcp
- `task-management serve --transport http --port 8080`
- Auth via MCP OAuth or bearer token

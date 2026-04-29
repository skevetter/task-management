# task-management

Rust CLI task manager with SQLite persistence, XDG-compliant paths, and task linking.

## Install

```bash
brew install skevetter/tap/task-management
```

Or build from source:

```bash
cargo build --release
```

## Database Path

Resolved in order:

1. `--db <path>` flag overrides everything
2. `$XDG_DATA_HOME/task-management/tasks.db`
3. `~/.local/share/task-management/tasks.db` (default)

## Commands

| Command | Description |
|---------|-------------|
| `create --title "..." [--priority P] [--assignee A] [--tag T] [--parent ID]` | Create a task |
| `list [--status S] [--priority P] [--tag T] [--parent ID] [--blocked-by ID] [--blocks ID]` | List/filter tasks |
| `show <ID>` | Show task details and links |
| `update <ID> [--status S] [--priority P] [--assignee A] [--tag T]` | Update a task |
| `close <ID>` | Close a task |
| `note <ID> "message" [--author A]` | Add a note |
| `history <ID>` | View task timeline |
| `link add <ID> <REL> <TARGET>` | Add a relationship |
| `link remove <LINK_ID>` | Remove a relationship |
| `link list <ID>` | List task relationships |

## Relationship Types

| Type | Inverse | Meaning |
|------|---------|---------|
| `parent` | `child` | Hierarchical parent |
| `child` | `parent` | Hierarchical child |
| `blocked_by` | `blocks` | Cannot proceed until target completes |
| `blocks` | `blocked_by` | Blocking the target |
| `related_to` | `related_to` | General association (symmetric) |

Links are bidirectional — querying from either end shows the correct perspective.

## JSON Output

All subcommands accept `--json` for machine-readable output; human-readable is the default.

```bash
task-management list --status open --json
task-management show <ID> --json
```

## Short ID Prefix

Commands accepting a task ID also accept a unique 4+ character prefix.

```bash
task-management show a3f1
```

Ambiguous prefixes exit with an error listing all matches.

## MCP Server

The MCP server exposes all task operations as tools over stdio, enabling structured JSON access for AI agents.

```bash
task-management serve
task-management serve --db /path/to/tasks.db --namespace my-project
```

### Agent Configuration

Claude Code (`.claude/settings.json`):

```json
{
  "mcpServers": {
    "task-management": {
      "command": "task-management",
      "args": ["serve"]
    }
  }
}
```

Cursor (`.cursor/mcp.json`):

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

### Available Tools

| Tool | Description |
|------|-------------|
| create_task | Create a new task |
| list_tasks | List/filter tasks |
| show_task | Show task detail with notes, timeline, links |
| update_task | Update task fields |
| close_task | Close a task |
| add_note | Add a note to a task |
| task_history | Get task timeline events |
| link_tasks | Create a link between tasks |
| list_links | List links for a task |

Tool parameters are auto-discovered via the MCP protocol.

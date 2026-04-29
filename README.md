# task-management

Rust CLI task manager with SQLite persistence, XDG-compliant paths, task linking, and namespace scoping.

## Install

```bash
brew install skevetter/tap/task-management
```

Or build from source:

```bash
cargo build --release
```

## What's New in v0.5.0

- **Namespace isolation fix**: Namespace filtering is now applied consistently across all DB queries, preventing tasks from leaking across namespaces.
- **`unlink_tasks` MCP tool**: Remove task relationships directly via MCP without the CLI.
- **Cancelled status**: Tasks can now be set to `cancelled` (alias: `closed`). Cancelled tasks are excluded from open listings by default.
- **Bulk close**: Close multiple tasks in one command with `close --all` or `close --tag <tag>` via CLI and MCP.
- **Task templates**: Create reusable task templates and instantiate them via `template apply`. Templates support all task fields and are stored per-namespace.

## Database Path

Resolved in order:

1. `--db <path>` flag overrides everything
2. `$XDG_DATA_HOME/task-management/tasks.db`
3. `~/.local/share/task-management/tasks.db` (default)

## Namespace Scoping

All commands support `--namespace` (`-n`) to isolate tasks into separate scopes. Default namespace is `default`.

```bash
task-management create --title "Fix bug" --namespace my-project
task-management list --namespace my-project
task-management show abc1 --namespace my-project
task-management update abc1 --status closed --namespace my-project
```

## List Pagination

The `list` command supports `--limit` and `--offset` for pagination. Default limit is 50, offset is 0.

```bash
task-management list --limit 10
task-management list --limit 10 --offset 20
task-management list --namespace my-project --limit 5
```

## Commands

| Command | Description |
|---------|-------------|
| `create --title "..." [--priority P] [--assignee A] [--tag T] [--parent ID] [-n NS]` | Create a task |
| `list [--status S] [--priority P] [--tag T] [--limit N] [--offset N] [-n NS]` | List/filter tasks |
| `show <ID> [-n NS]` | Show task details and links |
| `update <ID> [--status S] [--priority P] [--assignee A] [--tag T] [-n NS]` | Update a task |
| `close <ID> [-n NS]` | Close a task |
| `close --all [-n NS]` | Bulk close all open tasks |
| `close --tag <TAG> [-n NS]` | Bulk close tasks by tag |
| `note <ID> "message" [--author A] [-n NS]` | Add a note |
| `history <ID> [-n NS]` | View task timeline |
| `link add <ID> <REL> <TARGET> [-n NS]` | Add a relationship |
| `link remove <LINK_ID> [-n NS]` | Remove a relationship |
| `link list <ID> [-n NS]` | List task relationships |
| `template create --name "..." [--title T] [--priority P] [--tag T] [-n NS]` | Create a task template |
| `template list [-n NS]` | List templates |
| `template apply <TEMPLATE_ID> [-n NS]` | Instantiate a template as a task |
| `template delete <TEMPLATE_ID> [-n NS]` | Delete a template |

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

The `list --json` command returns a pagination envelope:

```json
{"tasks": [...], "total": 42, "limit": 50, "offset": 0}
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
| bulk_close_tasks | Close multiple tasks by filter |
| add_note | Add a note to a task |
| task_history | Get task timeline events |
| link_tasks | Create a link between tasks |
| unlink_tasks | Remove a link between tasks |
| list_links | List links for a task |
| create_template | Create a task template |
| list_templates | List available templates |
| apply_template | Instantiate a template as a task |
| delete_template | Delete a template |

All tools accept a `namespace` parameter (defaults to `"default"`). The `list_tasks` tool also accepts `limit` and `offset` for pagination.

Tool parameters are auto-discovered via the MCP protocol.

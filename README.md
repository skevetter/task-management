# task-management

Rust CLI task manager with SQLite persistence, XDG-compliant paths, and task linking.

## Install

```bash
brew install --HEAD git@github.com:skevetter/task-management.git
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

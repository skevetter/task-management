# task-management

A command-line task management tool built in Rust with SQLite persistence for creating, tracking, and querying tasks.

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/task-management`.

## Database Path

The CLI resolves the database file in this order:

1. `--db <path>` flag — overrides everything
2. `$XDG_DATA_HOME/task-management/tasks.db` — used when `XDG_DATA_HOME` is set and non-empty
3. `~/.local/share/task-management/tasks.db` — the default fallback

The directory is created automatically on first run.

```bash
# Explicit path
task-management --db /tmp/dev.db list

# Use a custom XDG location
XDG_DATA_HOME=/mnt/data task-management list
# → opens /mnt/data/task-management/tasks.db

# Default (XDG_DATA_HOME unset)
task-management list
# → opens ~/.local/share/task-management/tasks.db
```

## Usage

### Create a task

```bash
task-management create --title "Implement auth" --priority high --assignee alice --tag backend --tag security
```

With a parent task:

```bash
task-management create --title "Write unit tests" --parent <PARENT_TASK_ID>
```

### Show a task

```bash
task-management show <TASK_ID>
```

When the task has relationships, the output includes a **Links** section:

```
ID:          abc123...
Title:       Write unit tests
Status:      open
...

Links:
  blocked_by  def456  (Implement auth)
  related_to  ffe012  (Update API docs)
```

### Update a task

```bash
task-management update <TASK_ID> --status in-progress --assignee bob
task-management update <TASK_ID> --priority critical --tag urgent
```

### List tasks

List all tasks:

```bash
task-management list
```

Filter by status:

```bash
task-management list --status open
task-management list --status done
```

Filter by assignee:

```bash
task-management list --assignee alice
```

Filter by priority:

```bash
task-management list --priority high
```

Filter by tag:

```bash
task-management list --tag backend
```

Filter by parent task (resolved via the link system):

```bash
task-management list --parent <PARENT_TASK_ID>
```

Filter by blocking relationship:

```bash
task-management list --blocked-by <TASK_ID>   # tasks blocked by <TASK_ID>
task-management list --blocks <TASK_ID>        # tasks that block <TASK_ID>
```

Combine multiple filters (AND logic):

```bash
task-management list --status open --priority high --assignee alice --tag backend
```

### Close a task

```bash
task-management close <TASK_ID>
```

### Link tasks

Links record directed relationships between tasks. Every link is bidirectional — querying from either end shows the correct perspective.

#### Relationship types

| Type | Inverse | Meaning |
|------|---------|---------|
| `parent` | `child` | One task is the parent of another |
| `child` | `parent` | One task is a child of another |
| `blocked_by` | `blocks` | This task cannot proceed until the target is done |
| `blocks` | `blocked_by` | This task is blocking the target |
| `related_to` | `related_to` | General association (symmetric) |

#### Add a link

```bash
task-management link add <TASK_ID> <RELATIONSHIP> <TARGET_ID>
```

Example:

```bash
task-management link add abc123 blocked_by def456
# abc123 is now blocked by def456
# querying def456's links shows it "blocks" abc123
```

#### Remove a link

```bash
task-management link remove <LINK_ID>
```

The link ID appears in `link list` output and in the Links section of `show`.

Example:

```bash
task-management link remove 9f1a2b3c
```

#### List links for a task

```bash
task-management link list <TASK_ID>
```

Shows all relationships from both directions, each with the correct perspective label:

```
LINK ID    RELATIONSHIP   RELATED TASK
────────   ────────────   ─────────────────────────────────
9f1a2b3c   blocked_by     def456  (Implement auth)
7e3d4c5b   related_to     ffe012  (Update API docs)

2 link(s).
```

### Add a note to a task

```bash
task-management note <TASK_ID> "<message>" [--author <name>]
```

Example:

```bash
task-management note a1b2c3d4-... "Blocked waiting for DB credentials" --author alice
```

Output:

```
Note ID:    e9f0a1b2-5c6d-7e8f-9012-3456789abcde
Task:       a1b2c3d4-5678-90ab-cdef-1234567890ab
Author:     alice
Body:       Blocked waiting for DB credentials
Created:    2026-04-28T23:00:00Z
```

### View task history

```bash
task-management history <TASK_ID>
```

Example output:

```
History for task a1b2c3d4-5678-90ab-cdef-1234567890ab
──────────────────────────────────────────────────────
2026-04-28T22:00:00Z  [created]          Implement auth
2026-04-28T22:05:00Z  [status_changed]   open → in-progress
2026-04-28T22:30:00Z  [assignee_changed] (none) → alice
2026-04-28T23:00:00Z  [note_added]       Blocked waiting for DB credentials (by alice)
──────────────────────────────────────────────────────
4 event(s)
```

The timeline automatically tracks:
- Task creation
- Status changes
- Assignee changes
- Priority changes
- Notes added

These events are recorded automatically — no manual action needed.

## Task Fields

| Field | Description |
|-------|-------------|
| `id` | Auto-generated UUID |
| `title` | Task title (required) |
| `description` | Optional description |
| `status` | Current status (see below) |
| `priority` | Priority level (see below) |
| `assignee` | Optional assignee name |
| `tags` | List of string tags |
| `parent_task_id` | Optional parent task UUID for subtasks |
| `created_at` | Creation timestamp (RFC 3339) |
| `updated_at` | Last update timestamp (RFC 3339) |

## Status Values

- `open` — Task is created but not started
- `in-progress` — Task is actively being worked on
- `blocked` — Task is blocked by a dependency
- `done` — Task is completed
- `closed` — Task is closed/archived

## Priority Values

- `low`
- `medium` (default)
- `high`
- `critical`

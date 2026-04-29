# task-management

A command-line task management tool built in Rust with SQLite persistence for creating, tracking, and querying tasks.

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/task-management`.

## Usage

All commands accept an optional `--db <path>` flag to specify the database file (defaults to `tasks.db`).

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

Filter by parent task:

```bash
task-management list --parent <PARENT_TASK_ID>
```

Combine multiple filters (AND logic):

```bash
task-management list --status open --priority high --assignee alice --tag backend
```

### Close a task

```bash
task-management close <TASK_ID>
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

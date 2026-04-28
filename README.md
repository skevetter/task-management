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

# Notes and Timeline Feature — Design

## 1. Data Model

Two tables extend the existing `tasks` table. Both use the same UUID v4 + RFC 3339 timestamp conventions established in `src/db.rs`.

### `task_notes`

Stores free-text comments attached to a task.

```sql
CREATE TABLE IF NOT EXISTS task_notes (
    id          TEXT PRIMARY KEY,          -- UUID v4, same as tasks.id
    task_id     TEXT NOT NULL,             -- FK → tasks.id
    body        TEXT NOT NULL,             -- Note text, cannot be empty
    author      TEXT,                      -- Optional agent/user identifier
    created_at  TEXT NOT NULL,             -- RFC 3339 via chrono::Utc
    FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_task_notes_task_id ON task_notes (task_id);
```

### `timeline_events`

Append-only log of all state changes and notes for a task, ordered by timestamp.

```sql
CREATE TABLE IF NOT EXISTS timeline_events (
    id          TEXT PRIMARY KEY,          -- UUID v4
    task_id     TEXT NOT NULL,             -- FK → tasks.id
    event_type  TEXT NOT NULL,             -- 'created' | 'status_changed' | 'assignee_changed'
                                           -- | 'priority_changed' | 'note_added'
    old_value   TEXT,                      -- Previous value (NULL for 'created' and 'note_added')
    new_value   TEXT NOT NULL,             -- New value or note body
    actor       TEXT,                      -- Optional agent/user who triggered the event
    occurred_at TEXT NOT NULL,             -- RFC 3339 via chrono::Utc
    FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_timeline_task_id ON timeline_events (task_id);
CREATE INDEX IF NOT EXISTS idx_timeline_occurred ON timeline_events (occurred_at);
```

### Event type values

| `event_type`       | `old_value`         | `new_value`           |
|--------------------|---------------------|-----------------------|
| `created`          | NULL                | task title            |
| `status_changed`   | previous status str | new status str        |
| `assignee_changed` | previous assignee   | new assignee          |
| `priority_changed` | previous priority   | new priority          |
| `note_added`       | NULL                | note body text        |

### Rust model additions (`src/models.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNote {
    pub id: String,
    pub task_id: String,
    pub body: String,
    pub author: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    pub task_id: String,
    pub event_type: String,
    pub old_value: Option<String>,
    pub new_value: String,
    pub actor: Option<String>,
    pub occurred_at: String,
}
```

Both structs must be `pub` and added to `src/models.rs`. No changes to `src/lib.rs` are required — the CLI binary does not expose a library API.

## 2. CLI Interface Design

Two subcommands extend the existing `Commands` enum in `src/main.rs`. They follow the same positional-id + optional flags pattern used by `Show` and `Close`.

### `note` — add a note to a task

```
task-management note <TASK_ID> "<message>" [--author <name>]
```

**Example:**

```
$ task-management note a1b2c3d4-... "Blocked waiting for DB credentials" --author alice
```

**Output** (mirrors `Task::fmt` style — labeled fields, one per line):

```
Note ID:    e9f0a1b2-5c6d-7e8f-9012-3456789abcde
Task:       a1b2c3d4-5678-90ab-cdef-1234567890ab
Author:     alice
Body:       Blocked waiting for DB credentials
Created:    2026-04-28T23:00:00Z
```

If `<TASK_ID>` does not exist, the command exits with code 1 and prints to stderr:

```
Task not found: a1b2c3d4-...
```

### `history` — show chronological activity for a task

```
task-management history <TASK_ID>
```

**Example:**

```
$ task-management history a1b2c3d4-...
```

**Output:**

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

If no events exist (task was never mutated and has no notes), the output is:

```
History for task a1b2c3d4-5678-90ab-cdef-1234567890ab
──────────────────────────────────────────────────────
(no events)
──────────────────────────────────────────────────────
```

### Clap additions (`src/main.rs`)

```rust
Note {
    id: String,
    message: String,
    #[arg(long)]
    author: Option<String>,
},
History {
    id: String,
},
```

Both commands use `Database::open(&cli.db)` exactly as the existing commands do, keeping the `--db` global flag functional.

## 3. Notes-to-Tasks Relationship

### Foreign key

`task_notes.task_id` references `tasks.id` with `ON DELETE CASCADE`. When a task is deleted, all its notes are deleted automatically by SQLite without any application-layer code.

rusqlite enables foreign key enforcement at connection time. The current `Database::open` in `src/db.rs` does NOT include this pragma — it must be added as part of INI-107. Add this call immediately after `Connection::open`:

```rust
conn.execute_batch("PRAGMA foreign_keys = ON;")?;
```

Without this, SQLite ignores foreign key constraints by default and CASCADE deletes will not fire.

### Ordering

Notes are ordered by `created_at ASC` in all queries — oldest first. This matches human reading order for a comment thread and is consistent with timeline event ordering.

```sql
SELECT id, task_id, body, author, created_at
FROM task_notes
WHERE task_id = ?1
ORDER BY created_at ASC;
```

### Uniqueness

There is no uniqueness constraint on `(task_id, body)`. Two identical notes on the same task are allowed — an agent may legitimately post the same status update twice.

### `Database` method signatures

```rust
impl Database {
    pub fn add_note(
        &self,
        task_id: &str,
        body: &str,
        author: Option<&str>,
    ) -> Result<TaskNote>;

    pub fn get_notes(&self, task_id: &str) -> Result<Vec<TaskNote>>;
}
```

`add_note` must:
1. Verify the task exists: `let task = get_task(task_id)?.ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)?` — `get_task` returns `Result<Option<Task>>`, so `ok_or_else` converts `None` into a distinguishable error. The CLI handler checks for this error and prints `"Task not found: {id}"` to stderr with exit code 1.
2. Insert into `task_notes`.
3. Insert a `note_added` event into `timeline_events` with `new_value = body`.
4. Return the constructed `TaskNote`.

Both the note insert and the timeline insert must be atomic. Wrap them in an explicit transaction: call `self.conn.execute("BEGIN IMMEDIATE", [])?;` before the first insert and `self.conn.execute("COMMIT", [])?;` after both succeed. On error, issue `ROLLBACK` to undo partial writes. Without an explicit transaction, each `execute` auto-commits independently and a crash between the two inserts would leave a note without a timeline event.

## 4. Timeline Aggregation

The `history` command reads all rows from `timeline_events` for a given task, sorted by `occurred_at ASC`, and renders them as a flat chronological list. There is no join to `task_notes` — notes appear in the timeline because `add_note` writes a `note_added` event to `timeline_events` at insert time.

### Query

```sql
SELECT id, event_type, old_value, new_value, actor, occurred_at
FROM timeline_events
WHERE task_id = ?1
ORDER BY occurred_at ASC;
```

### `Database` method

```rust
pub fn get_timeline(&self, task_id: &str) -> Result<Vec<TimelineEvent>>;
```

`get_timeline` does not verify the task exists — callers (the `history` command handler) verify via `get_task` first and exit with an error if not found.

### Rendering logic

Each row renders as a single line:

```
{occurred_at}  [{event_type}]  {description}
```

`description` is derived from `event_type`:

| `event_type`       | Description format                                      |
|--------------------|---------------------------------------------------------|
| `created`          | `{new_value}` (the task title)                          |
| `status_changed`   | `{old_value} → {new_value}`                             |
| `assignee_changed` | `{old_value or "(none)"} → {new_value or "(none)"}`     |
| `priority_changed` | `{old_value} → {new_value}`                             |
| `note_added`       | `{new_value}` followed by ` (by {actor})` if actor set  |

Column widths: `occurred_at` is 20 chars (truncated RFC 3339 to second precision), `event_type` is left-padded to the longest type width (18 chars including brackets).

### Event sources

Events enter `timeline_events` from three code paths:

1. `create_task` — inserts one `created` event immediately after the tasks row insert.
2. `update_task` — compares old vs new values for `status`, `assignee`, and `priority`; inserts one event per changed field.
3. `add_note` — inserts one `note_added` event after the `task_notes` insert.

## 5. Edge Cases

### Notes on closed tasks

**Allow.** `task-management note <id> "..."` succeeds regardless of the task's current status. Agents frequently need to add post-close commentary (e.g., recording the resolution, linking to a follow-up). Blocking notes on closed tasks would require a status check before every insert and would break legitimate workflows. The CLI does not warn that the task is closed.

### Note editing and deletion

**Not supported in v1.** Notes are append-only. There is no `note edit` or `note delete` subcommand. The `task_notes` schema has no `updated_at` column. Rationale: an audit-safe coordination log must be immutable. Agents that need to correct a note should add a new note with the correction.

If note deletion is added later, it must also delete the corresponding `timeline_events` row (matched by `task_id`, `event_type = 'note_added'`, and `new_value = body`).

### Timeline for tasks with no events

`get_timeline` returns an empty `Vec<TimelineEvent>`. The `history` command renders:

```
History for task <id>
──────────────────────────────────────────────────────
(no events)
──────────────────────────────────────────────────────
```

This can occur for tasks that existed before the migration (see Section 7 — those tasks have no `created` event in the timeline).

### DB migration from existing tasks.db files

Existing databases contain only the `tasks` table. Running the new binary against an old `tasks.db`:

1. `Database::open` calls `execute_batch` with `CREATE TABLE IF NOT EXISTS` for both new tables — the `IF NOT EXISTS` guard makes this a no-op if the tables already exist and safe to run if they do not.
2. No rows are backfilled. Tasks created before the migration have no `created` event in `timeline_events`. Their `history` output shows `(no events)` for pre-migration activity, then shows events from the moment they are next modified.
3. The `tasks` table schema is unchanged — no column additions, no `ALTER TABLE`. All existing task rows remain valid.

See Section 7 for the full migration strategy, including an optional schema version table for future-proofing.

## 6. Auto-Tracking

**Recommendation: yes, auto-track status, assignee, and priority changes.**

Manual event insertion is error-prone — callers would have to remember to call `record_event` after every mutation. Centralizing event recording inside `create_task` and `update_task` makes it impossible to mutate a task without producing a timeline entry.

### How it works

`update_task` already reads the current task before applying changes (`get_task(id)` at `src/db.rs:120`). The diff is available before the `UPDATE` executes. Add event inserts immediately after the `UPDATE` succeeds:

```rust
// inside update_task, after the UPDATE executes:
if task.status != new_status {
    self.insert_timeline_event(id, "status_changed",
        Some(&task.status.to_string()), &new_status.to_string(), None, &now)?;
}
if task.assignee.as_deref() != new_assignee {
    self.insert_timeline_event(id, "assignee_changed",
        task.assignee.as_deref(), new_assignee, None, &now)?;
}
if task.priority != new_priority {
    self.insert_timeline_event(id, "priority_changed",
        Some(&task.priority.to_string()), &new_priority.to_string(), None, &now)?;
}
```

`create_task` inserts one `created` event after the `tasks` row is inserted:

```rust
self.insert_timeline_event(&id, "created", None, title, None, &now)?;
```

### Private helper

```rust
fn insert_timeline_event(
    &self,
    task_id: &str,
    event_type: &str,
    old_value: Option<&str>,
    new_value: &str,
    actor: Option<&str>,
    occurred_at: &str,
) -> Result<()>;
```

This keeps event-insert logic in one place and avoids repeating the UUID + SQL boilerplate at every call site.

### What is not auto-tracked

- Title changes: excluded from auto-tracking in v1. Title is rarely changed and clutters the timeline. Can be added later by checking `task.title != new_title` in `update_task`.
- Description changes: excluded for the same reason. Descriptions can be long and diffing them in the timeline output is not useful at this scope.
- Tag changes: excluded. Tags are auxiliary metadata, not coordination state.

## 7. DB Migration Strategy

### Mechanism: `CREATE TABLE IF NOT EXISTS`

All three new DDL statements use `IF NOT EXISTS`, matching the pattern in `Database::open` at `src/db.rs:14`. On first open against an old `tasks.db`, the two new tables are created. On subsequent opens, the statements are no-ops. No manual migration script, no external tooling.

The following shows the complete `Database::open` implementation after INI-107. The existing `tasks` table DDL is unchanged; only the `PRAGMA foreign_keys`, `task_notes`, and `timeline_events` blocks are new additions. `Database::open` becomes:

```rust
conn.execute_batch(
    "PRAGMA foreign_keys = ON;

     CREATE TABLE IF NOT EXISTS tasks (
         id TEXT PRIMARY KEY,
         title TEXT NOT NULL,
         description TEXT,
         status TEXT NOT NULL DEFAULT 'open',
         priority TEXT NOT NULL DEFAULT 'medium',
         assignee TEXT,
         tags TEXT NOT NULL DEFAULT '[]',
         parent_task_id TEXT,
         created_at TEXT NOT NULL,
         updated_at TEXT NOT NULL
     );

     CREATE TABLE IF NOT EXISTS task_notes (
         id TEXT PRIMARY KEY,
         task_id TEXT NOT NULL,
         body TEXT NOT NULL,
         author TEXT,
         created_at TEXT NOT NULL,
         FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
     );
     CREATE INDEX IF NOT EXISTS idx_task_notes_task_id ON task_notes (task_id);

     CREATE TABLE IF NOT EXISTS timeline_events (
         id TEXT PRIMARY KEY,
         task_id TEXT NOT NULL,
         event_type TEXT NOT NULL,
         old_value TEXT,
         new_value TEXT NOT NULL,
         actor TEXT,
         occurred_at TEXT NOT NULL,
         FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
     );
     CREATE INDEX IF NOT EXISTS idx_timeline_task_id ON timeline_events (task_id);
     CREATE INDEX IF NOT EXISTS idx_timeline_occurred ON timeline_events (occurred_at);",
)?;
```

### Schema version table (future-proofing)

For future migrations that require column additions or data transforms, add a `schema_versions` table:

```sql
CREATE TABLE IF NOT EXISTS schema_versions (
    version   INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
```

`Database::open` reads `MAX(version)` from this table and applies any pending migrations in order. For this release (INI-107), insert version 1 if `MAX(version)` returns NULL:

```sql
INSERT OR IGNORE INTO schema_versions (version, applied_at)
VALUES (1, '<now>');
```

Future migrations increment the version. This pattern allows safe ALTER TABLE statements (adding nullable columns) and data backfills in subsequent releases without breaking existing installs.

### What is not migrated

Pre-existing `tasks` rows are not backfilled with `created` timeline events. The cost of scanning and inserting events for potentially thousands of tasks at open time is not justified for v1. Agents using `history` on pre-migration tasks see `(no events)` until the task is next modified.

# XDG + Task Linking Design Document (INI-108)

## 1. Overview

Two independent features ship together under INI-108:

**XDG-Compliant Default Database Path** — The CLI currently defaults to `tasks.db` in the working directory (`src/main.rs:9`, `DEFAULT_DB_PATH`). This is replaced with `$XDG_DATA_HOME/task-management/tasks.db`, falling back to `~/.local/share/task-management/tasks.db` when `XDG_DATA_HOME` is unset. The `--db` flag continues to override the default. Directory creation is automatic.

**Rich Task Linking / Relationships** — Tasks currently support a single `parent_task_id` column in the `tasks` table. This feature replaces that flat column with a `task_links` join table that models directed relationships: `parent`, `child`, `blocked_by`, `blocks`, and `related_to`. Three new subcommands (`link add`, `link remove`, `link list`) manage links. The `show` command displays links inline; `list` gains relationship filters. Existing `parent_task_id` data migrates to `task_links` at startup.

## 2. XDG-Compliant Default Database Path

### 2.1 Current Behavior

`src/main.rs:9` defines `const DEFAULT_DB_PATH: &str = "tasks.db"`. The clap `Cli` struct uses this as the default value for the `--db` global argument (`src/main.rs:14`). `Database::open` receives the path string and calls `rusqlite::Connection::open(path)` directly (`src/db.rs:13`). No path resolution or directory creation occurs. Running the binary from `/home/user/projects/work` creates `tasks.db` in that directory; running it from `/tmp` creates a separate unrelated database.

### 2.2 New Default Path Logic

Replace `DEFAULT_DB_PATH` with a runtime function `default_db_path() -> PathBuf` in `src/main.rs`:

```
1. Read std::env::var("XDG_DATA_HOME").
2. If set and non-empty: use $XDG_DATA_HOME/task-management/tasks.db.
3. Otherwise: use $HOME/.local/share/task-management/tasks.db.
   - If HOME is also unset, fall back to a relative path ./task-management/tasks.db
     and emit a warning to stderr.
```

The clap `--db` argument loses its `default_value` attribute. Instead, `main()` calls `default_db_path()` when `cli.db` is `None`. The `Cli.db` field type changes from `String` to `Option<String>`.

### 2.3 Directory Creation

Before passing the path to `Database::open`, `main()` calls `std::fs::create_dir_all(parent)` on the resolved path's parent directory. This creates `task-management/` (and any missing XDG prefix directories) atomically. `create_dir_all` is a no-op if the directory already exists, so no guard is needed. If `create_dir_all` fails (e.g., permission denied), the binary prints `"Failed to create database directory: {e}"` to stderr and exits with code 1 — the same pattern used throughout `main()` today.

### 2.4 `--db` Override and Environment Variable Handling

When `--db <path>` is provided, it takes unconditional precedence. `default_db_path()` is not called. Directory creation still runs against the explicitly-provided path so `--db /new/dir/tasks.db` works without the user pre-creating `/new/dir/`.

Environment variable edge cases for `XDG_DATA_HOME`:

| Condition | Behaviour |
|-----------|-----------|
| Set to a non-empty absolute path | Use as-is: `$XDG_DATA_HOME/task-management/tasks.db` |
| Set to empty string (`XDG_DATA_HOME=`) | Treat as unset; fall back to `~/.local/share/` |
| Set to a relative path (e.g., `data`) | Resolve relative to cwd and use: `<cwd>/data/task-management/tasks.db`. Log a warning to stderr: `"XDG_DATA_HOME is a relative path; resolving against cwd"` |
| Unset | Use `~/.local/share/task-management/tasks.db` |

The XDG Base Directory Specification requires `XDG_DATA_HOME` to be absolute when set; the relative-path warning enforces this without hard-failing.

### 2.5 Files Affected

| File | Change |
|------|--------|
| `src/main.rs` | Remove `DEFAULT_DB_PATH` constant; add `default_db_path() -> PathBuf`; change `Cli.db` to `Option<String>`; add `create_dir_all` call before `Database::open` |
| `src/db.rs` | `Database::open` signature unchanged (`path: &str`); no changes required |

## 3. Rich Task Linking / Relationships

### 3.1 Schema: `task_links` Table

```sql
CREATE TABLE IF NOT EXISTS task_links (
    id         TEXT PRIMARY KEY,              -- UUID v4
    source_id  TEXT NOT NULL,                 -- FK to tasks.id
    target_id  TEXT NOT NULL,                 -- FK to tasks.id
    link_type  TEXT NOT NULL,                 -- see §3.2
    created_at TEXT NOT NULL,                 -- RFC-3339 timestamp
    FOREIGN KEY (source_id) REFERENCES tasks (id) ON DELETE CASCADE,
    FOREIGN KEY (target_id) REFERENCES tasks (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_task_links_source ON task_links (source_id);
CREATE INDEX IF NOT EXISTS idx_task_links_target ON task_links (target_id);
```

Links are directed: `(source_id, link_type, target_id)` encodes the relationship from the source task's perspective. The inverse view is computed at query time (see §3.2); no mirror row is stored.

### 3.2 Relationship Types and Inverses

`link_type` stores the relationship from the source task's point of view. Valid strings:

| `link_type` stored | Meaning | Inverse `link_type` when queried from target |
|--------------------|---------|----------------------------------------------|
| `parent`           | source is the parent of target | `child` |
| `child`            | source is a child of target | `parent` |
| `blocked_by`       | source is blocked by target | `blocks` |
| `blocks`           | source blocks target | `blocked_by` |
| `related_to`       | source is related to target | `related_to` |

`link list <task-id>` returns both rows where `source_id = task-id` (with their stored `link_type`) and rows where `target_id = task-id` (with the inverse type computed above). The caller sees a flat list of `{ link_id, related_task_id, relationship }` from the queried task's perspective.

`parent` and `child` are distinct stored types so that a user can assert `task A child task B` directly, but the natural creation path is `link add <parent> parent <child>` which stores `link_type = "parent"`. The query for task B then returns `child` as the inverse.

### 3.3 CLI Commands

Three subcommands nest under a `link` parent command in `src/main.rs`:

**`link add <task-id> <relationship> <target-id>`**

Creates one row in `task_links` with `source_id = task-id`, `link_type = relationship`, `target_id = target-id`. Both task IDs must exist; the command exits with code 1 and an error message if either is not found. `<relationship>` must be one of the five valid strings; clap validates this via a `ValueEnum`.

```
$ task-management link add abc123 blocked_by def456
Link created: 9f1a2b3c (abc123 blocked_by def456)
```

**`link remove <link-id>`**

Deletes the row with `id = link-id`. Exits with code 1 if the link does not exist.

```
$ task-management link remove 9f1a2b3c
Link 9f1a2b3c removed.
```

**`link list <task-id>`**

Queries both directions from `task_links` and prints a table. The `LINK ID` column shows the first 8 characters of the UUID.

```
$ task-management link list abc123
LINK ID    RELATIONSHIP   RELATED TASK
────────   ────────────   ─────────────────────────────────
9f1a2b3c   blocked_by     def456  (Implement auth layer)
a0b1c2d3   child          111222  (Write unit tests)

2 link(s).
```

### 3.4 `show` Command Enhancement

`Database::get_task` returns a `Task`. After printing the existing fields, `show` makes one additional query to `task_links` (both source and target directions) and appends a `Links:` section if any rows exist:

```
ID:          abc123...
Title:       Implement login flow
...
Links:
  blocked_by  def456  (Implement auth layer)
  child       111222  (Write unit tests)
```

No links → the section is omitted entirely (not printed as empty). The `Task` struct in `src/models.rs` does **not** grow a `links` field; the show handler fetches links separately via a new `Database::get_links(task_id)` method.

### 3.5 `list` Command Enhancement

`list` gains three optional filters that query through `task_links`:

| Flag | Behaviour |
|------|-----------|
| `--blocked-by <id>` | Return tasks where a `blocked_by` link exists with `target_id = <id>` |
| `--blocks <id>` | Return tasks where a `blocks` link exists with `target_id = <id>` |
| `--parent <id>` | Return tasks where a `parent` link exists with `target_id = <id>` (replaces the old `parent_task_id`-based `--parent` filter on `list`) |

The existing `--parent` flag (`src/main.rs:68`) currently filters on `tasks.parent_task_id`. After migration it filters on `task_links` instead; the flag name and user-visible behaviour are unchanged.

Filters are additive (AND). Example:

```
$ task-management list --blocked-by def456
```

Returns all tasks blocked by `def456`.

### 3.6 Timeline Tracking

`Database::create_link` and `Database::remove_link` each call `insert_timeline_event` (already in `src/db.rs:310`) with:

| Operation | `event_type` | `old_value` | `new_value` |
|-----------|-------------|-------------|-------------|
| Link created | `link_added` | `NULL` | `"<link_type>:<target_id>"` |
| Link removed | `link_removed` | `"<link_type>:<target_id>"` | `""` |

Both operations wrap in a transaction so the `task_links` row and the `timeline_events` row commit atomically. This reuses the existing transaction pattern in `Database::add_note` (`src/db.rs:336–356`).

The `history` command (`src/main.rs:264–306`) already handles arbitrary `event_type` strings; no changes are needed there — `link_added` and `link_removed` events appear automatically in history output.

### 3.7 Files Affected

| File | Change |
|------|--------|
| `src/db.rs` | Add `task_links` DDL to `execute_batch`; add `create_link`, `remove_link`, `get_links` methods; update `list_tasks` to join through `task_links` for `--parent`/`--blocked-by`/`--blocks` filters; increment schema version to 2 |
| `src/models.rs` | Add `LinkType` enum (`Parent`, `Child`, `BlockedBy`, `Blocks`, `RelatedTo`) with `Display`/`FromStr`/`ValueEnum`; add `TaskLink` struct (`id`, `source_id`, `target_id`, `link_type`, `created_at`) |
| `src/main.rs` | Add `Link` variant to `Commands` enum with `Add`/`Remove`/`List` sub-subcommands; update `Show` handler to call `get_links`; update `List` handler for new filters |

## 4. Migration Strategy

`schema_versions` already exists (`src/db.rs:53–57`) with version 1 inserted at first open. The migration runs inside `Database::open` after the DDL block, gated on the current max version.

**Version 2 migration — `parent_task_id` → `task_links`:**

```sql
-- Run when MAX(version) < 2
INSERT INTO task_links (id, source_id, target_id, link_type, created_at)
SELECT
    lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' ||
    substr(lower(hex(randomblob(2))),2) || '-' ||
    substr('89ab', abs(random()) % 4 + 1, 1) ||
    substr(lower(hex(randomblob(2))),2) || '-' || lower(hex(randomblob(6))),
    id,
    parent_task_id,
    'parent',
    created_at
FROM tasks
WHERE parent_task_id IS NOT NULL;

INSERT OR IGNORE INTO schema_versions (version, applied_at) VALUES (2, <now>);
```

After the migration completes, `tasks.parent_task_id` is retained as a nullable column (dropping columns requires SQLite ≥ 3.35 and may not be available on all targets). It is no longer written by `create_task` or `update_task`. Engineers should remove it in a future cleanup once the rollout is confirmed stable.

The migration is idempotent: `INSERT OR IGNORE` on `schema_versions` and the `WHERE parent_task_id IS NOT NULL` guard mean re-running against a version-2 database is a no-op.

## 5. Acceptance Criteria

### XDG Default Path

- [ ] Running the binary without `--db` creates `~/.local/share/task-management/tasks.db` when `XDG_DATA_HOME` is unset.
- [ ] Running with `XDG_DATA_HOME=/tmp/xdg` creates `/tmp/xdg/task-management/tasks.db`.
- [ ] Setting `XDG_DATA_HOME=` (empty string) falls back to the `~/.local/share/` path.
- [ ] The `task-management/` directory is created automatically if it does not exist.
- [ ] `--db /custom/path/my.db` uses the explicit path and ignores `XDG_DATA_HOME`.
- [ ] All 19 existing tests pass without modification.

### Task Linking

- [ ] `link add <a> blocked_by <b>` inserts one row in `task_links` and one `link_added` event in `timeline_events`.
- [ ] `link list <a>` returns the `blocked_by` relationship; `link list <b>` returns the inverse `blocks` relationship — both referencing the same `task_links` row.
- [ ] `link remove <link-id>` deletes the row and inserts a `link_removed` timeline event.
- [ ] `show <a>` displays a `Links:` section listing all relationships from task A's perspective.
- [ ] `show <a>` omits the `Links:` section when no links exist.
- [ ] `list --blocked-by <b>` returns task A; `list --parent <p>` returns tasks whose parent is `<p>`.
- [ ] `history <a>` shows `link_added` and `link_removed` events with correct timestamps.
- [ ] Opening an existing database with `parent_task_id` rows populated migrates them to `task_links` as `parent` links; `schema_versions` contains version 2 after migration.
- [ ] `link add` with a non-existent task ID exits with code 1 and an error message.
- [ ] `link_type` values outside the five valid strings are rejected by clap before any DB write.

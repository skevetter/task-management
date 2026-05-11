use rusqlite::{Connection, params};
use task_management::db::Database;
use tempfile::NamedTempFile;

fn setup_v4_db_with_closed_data(path: &str) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;

         CREATE TABLE tasks (
             id TEXT PRIMARY KEY,
             title TEXT NOT NULL,
             description TEXT,
             status TEXT NOT NULL DEFAULT 'open',
             priority TEXT NOT NULL DEFAULT 'medium',
             assignee TEXT,
             tags TEXT NOT NULL DEFAULT '[]',
             parent_task_id TEXT,
             created_at TEXT NOT NULL,
             updated_at TEXT NOT NULL,
             namespace TEXT NOT NULL DEFAULT 'default'
         );
         CREATE INDEX idx_tasks_namespace ON tasks (namespace);

         CREATE TABLE task_notes (
             id TEXT PRIMARY KEY,
             task_id TEXT NOT NULL,
             body TEXT NOT NULL,
             author TEXT,
             created_at TEXT NOT NULL,
             FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
         );
         CREATE INDEX idx_task_notes_task_id ON task_notes (task_id);

         CREATE TABLE timeline_events (
             id TEXT PRIMARY KEY,
             task_id TEXT NOT NULL,
             event_type TEXT NOT NULL,
             old_value TEXT,
             new_value TEXT NOT NULL,
             actor TEXT,
             occurred_at TEXT NOT NULL,
             FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
         );
         CREATE INDEX idx_timeline_task_id ON timeline_events (task_id);
         CREATE INDEX idx_timeline_occurred ON timeline_events (occurred_at);

         CREATE TABLE task_links (
             id         TEXT PRIMARY KEY,
             source_id  TEXT NOT NULL,
             target_id  TEXT NOT NULL,
             link_type  TEXT NOT NULL,
             created_at TEXT NOT NULL,
             FOREIGN KEY (source_id) REFERENCES tasks (id) ON DELETE CASCADE,
             FOREIGN KEY (target_id) REFERENCES tasks (id) ON DELETE CASCADE
         );
         CREATE INDEX idx_task_links_source ON task_links (source_id);
         CREATE INDEX idx_task_links_target ON task_links (target_id);

         CREATE TABLE schema_versions (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL
         );

         CREATE TABLE task_templates (
             id TEXT PRIMARY KEY,
             name TEXT UNIQUE NOT NULL,
             title_pattern TEXT NOT NULL,
             default_priority TEXT,
             default_status TEXT,
             default_tags TEXT,
             builtin INTEGER NOT NULL DEFAULT 0,
             created_at TEXT NOT NULL
         );

         INSERT INTO schema_versions (version, applied_at) VALUES (1, datetime('now'));
         INSERT INTO schema_versions (version, applied_at) VALUES (2, datetime('now'));
         INSERT INTO schema_versions (version, applied_at) VALUES (3, datetime('now'));
         INSERT INTO schema_versions (version, applied_at) VALUES (4, datetime('now'));",
    )
    .unwrap();

    conn.execute(
        "INSERT INTO tasks (id, title, status, priority, tags, created_at, updated_at, namespace)
         VALUES (?1, ?2, ?3, ?4, '[]', datetime('now'), datetime('now'), 'default')",
        params!["task-closed-1", "Closed task one", "closed", "medium"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO tasks (id, title, status, priority, tags, created_at, updated_at, namespace)
         VALUES (?1, ?2, ?3, ?4, '[]', datetime('now'), datetime('now'), 'default')",
        params!["task-closed-2", "Closed task two", "closed", "high"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO tasks (id, title, status, priority, tags, created_at, updated_at, namespace)
         VALUES (?1, ?2, ?3, ?4, '[]', datetime('now'), datetime('now'), 'default')",
        params!["task-open-1", "Open task", "open", "low"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO timeline_events (id, task_id, event_type, old_value, new_value, actor, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, datetime('now'))",
        params!["evt-1", "task-closed-1", "status_changed", "open", "closed"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO timeline_events (id, task_id, event_type, old_value, new_value, actor, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, datetime('now'))",
        params!["evt-2", "task-closed-2", "status_changed", "closed", "open"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO timeline_events (id, task_id, event_type, old_value, new_value, actor, occurred_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, datetime('now'))",
        params!["evt-3", "task-open-1", "created", None::<String>, "Open task"],
    )
    .unwrap();
}

#[test]
fn migration_v5_converts_closed_tasks_to_cancelled() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap();
    setup_v4_db_with_closed_data(path);

    let _db = Database::open(path).unwrap();

    let conn = Connection::open(path).unwrap();
    let closed_task_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'closed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        closed_task_count, 0,
        "no tasks should have status 'closed' after migration v5"
    );

    let cancelled_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'cancelled'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        cancelled_count, 2,
        "both previously-closed tasks should now be 'cancelled'"
    );

    let open_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'open'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(open_count, 1, "open tasks should be unaffected");
}

#[test]
fn migration_v5_converts_closed_in_timeline_events() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap();
    setup_v4_db_with_closed_data(path);

    let _db = Database::open(path).unwrap();

    let conn = Connection::open(path).unwrap();

    let closed_new_value: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM timeline_events WHERE event_type = 'status_changed' AND new_value = 'closed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        closed_new_value, 0,
        "no timeline events should have new_value = 'closed'"
    );

    let closed_old_value: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM timeline_events WHERE event_type = 'status_changed' AND old_value = 'closed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        closed_old_value, 0,
        "no timeline events should have old_value = 'closed'"
    );

    let cancelled_new: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM timeline_events WHERE event_type = 'status_changed' AND new_value = 'cancelled'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        cancelled_new, 1,
        "evt-1 new_value should now be 'cancelled'"
    );

    let cancelled_old: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM timeline_events WHERE event_type = 'status_changed' AND old_value = 'cancelled'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        cancelled_old, 1,
        "evt-2 old_value should now be 'cancelled'"
    );

    let created_event_unchanged: String = conn
        .query_row(
            "SELECT new_value FROM timeline_events WHERE id = 'evt-3'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        created_event_unchanged, "Open task",
        "non-status_changed events should be untouched"
    );
}

#[test]
fn migration_v5_no_closed_rows_remain() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap();
    setup_v4_db_with_closed_data(path);

    let _db = Database::open(path).unwrap();

    let conn = Connection::open(path).unwrap();

    let closed_anywhere: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'closed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(closed_anywhere, 0);

    let closed_timeline_new: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM timeline_events WHERE new_value = 'closed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(closed_timeline_new, 0);

    let closed_timeline_old: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM timeline_events WHERE old_value = 'closed'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(closed_timeline_old, 0);

    let v5_applied: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM schema_versions WHERE version = 5",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(v5_applied, 1, "schema version 5 should be recorded");
}

#[test]
fn migration_v5_runs_on_fresh_db() {
    let db = Database::open(":memory:").unwrap();
    drop(db);
}

#[test]
fn closed_status_string_no_longer_parses() {
    use task_management::models::TaskStatus;
    let result = "closed".parse::<TaskStatus>();
    assert!(
        result.is_err(),
        "'closed' should no longer parse as a valid TaskStatus"
    );
}

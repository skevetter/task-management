use task_management::db::Database;
use task_management::models::{LinkType, TaskPriority};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn test_db() -> Database {
    Database::open(":memory:").expect("open in-memory db")
}

fn cli_cmd(db_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("task-management").unwrap();
    cmd.arg("--db").arg(db_path);
    cmd
}

fn create_task_via_cli(db_path: &str, title: &str) -> String {
    let output = cli_cmd(db_path)
        .args(["create", "--title", title])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(id) = line.strip_prefix("ID:") {
            return id.trim().to_string();
        }
    }
    panic!("Could not extract task ID from create output: {stdout}");
}

// --- Unit tests for LinkType ---

#[test]
fn link_type_display() {
    assert_eq!(LinkType::Parent.to_string(), "parent");
    assert_eq!(LinkType::Child.to_string(), "child");
    assert_eq!(LinkType::BlockedBy.to_string(), "blocked_by");
    assert_eq!(LinkType::Blocks.to_string(), "blocks");
    assert_eq!(LinkType::RelatedTo.to_string(), "related_to");
}

#[test]
fn link_type_from_str() {
    assert_eq!("parent".parse::<LinkType>().unwrap(), LinkType::Parent);
    assert_eq!("child".parse::<LinkType>().unwrap(), LinkType::Child);
    assert_eq!(
        "blocked_by".parse::<LinkType>().unwrap(),
        LinkType::BlockedBy
    );
    assert_eq!("blocks".parse::<LinkType>().unwrap(), LinkType::Blocks);
    assert_eq!(
        "related_to".parse::<LinkType>().unwrap(),
        LinkType::RelatedTo
    );
    assert!("invalid".parse::<LinkType>().is_err());
}

#[test]
fn link_type_inverse() {
    assert_eq!(LinkType::Parent.inverse(), LinkType::Child);
    assert_eq!(LinkType::Child.inverse(), LinkType::Parent);
    assert_eq!(LinkType::BlockedBy.inverse(), LinkType::Blocks);
    assert_eq!(LinkType::Blocks.inverse(), LinkType::BlockedBy);
    assert_eq!(LinkType::RelatedTo.inverse(), LinkType::RelatedTo);
}

// --- DB-level link tests ---

#[test]
fn create_and_list_link() {
    let db = test_db();
    let t1 = db
        .create_task(
            "Task A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "Task B",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    let link_id = db
        .create_link(&t1.id, &t2.id, &LinkType::BlockedBy)
        .unwrap();
    assert!(!link_id.is_empty());

    let links = db.get_links(&t1.id).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].0, link_id);
    assert_eq!(links[0].1, LinkType::BlockedBy);
    assert_eq!(links[0].2, t2.id);
    assert_eq!(links[0].3, "Task B");
}

#[test]
fn inverse_relationship_from_target() {
    let db = test_db();
    let t1 = db
        .create_task(
            "Task A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "Task B",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    db.create_link(&t1.id, &t2.id, &LinkType::BlockedBy)
        .unwrap();

    let links_from_t2 = db.get_links(&t2.id).unwrap();
    assert_eq!(links_from_t2.len(), 1);
    assert_eq!(links_from_t2[0].1, LinkType::Blocks);
    assert_eq!(links_from_t2[0].2, t1.id);
}

#[test]
fn remove_link_db() {
    let db = test_db();
    let t1 = db
        .create_task(
            "Task A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "Task B",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    let link_id = db.create_link(&t1.id, &t2.id, &LinkType::Parent).unwrap();

    db.remove_link(&link_id).unwrap();

    let links = db.get_links(&t1.id).unwrap();
    assert!(links.is_empty());
}

#[test]
fn create_link_nonexistent_task() {
    let db = test_db();
    let t1 = db
        .create_task(
            "Task A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    let result = db.create_link(&t1.id, "nonexistent-id", &LinkType::BlockedBy);
    assert!(result.is_err());
}

#[test]
fn remove_link_nonexistent() {
    let db = test_db();
    let result = db.remove_link("nonexistent-link-id");
    assert!(result.is_err());
}

#[test]
fn list_tasks_filter_blocked_by() {
    let db = test_db();
    let blocker = db
        .create_task(
            "Blocker",
            None,
            TaskPriority::High,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let blocked = db
        .create_task(
            "Blocked task",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    db.create_task(
        "Unrelated",
        None,
        TaskPriority::Low,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    db.create_link(&blocked.id, &blocker.id, &LinkType::BlockedBy)
        .unwrap();

    let results = db
        .list_tasks(
            None,
            None,
            None,
            None,
            None,
            Some(&blocker.id),
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, blocked.id);
}

#[test]
fn list_tasks_filter_blocks() {
    let db = test_db();
    let t1 = db
        .create_task(
            "Task A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "Task B",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    db.create_link(&t1.id, &t2.id, &LinkType::Blocks).unwrap();

    let results = db
        .list_tasks(
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&t2.id),
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, t1.id);
}

#[test]
fn list_tasks_filter_parent_via_links() {
    let db = test_db();
    let parent = db
        .create_task(
            "Parent",
            None,
            TaskPriority::High,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let child = db
        .create_task(
            "Child",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    db.create_link(&child.id, &parent.id, &LinkType::Parent)
        .unwrap();

    let results = db
        .list_tasks(
            None,
            None,
            None,
            None,
            Some(&parent.id),
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, child.id);
}

#[test]
fn timeline_link_added() {
    let db = test_db();
    let t1 = db
        .create_task(
            "Task A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "Task B",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    db.create_link(&t1.id, &t2.id, &LinkType::BlockedBy)
        .unwrap();

    let events = db.get_timeline(&t1.id).unwrap();
    let link_event = events.iter().find(|e| e.event_type == "link_added");
    assert!(link_event.is_some());
    let event = link_event.unwrap();
    assert_eq!(event.new_value, format!("blocked_by:{}", t2.id));
    assert!(event.old_value.is_none());
}

#[test]
fn timeline_link_removed() {
    let db = test_db();
    let t1 = db
        .create_task(
            "Task A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "Task B",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    let link_id = db
        .create_link(&t1.id, &t2.id, &LinkType::RelatedTo)
        .unwrap();
    db.remove_link(&link_id).unwrap();

    let events = db.get_timeline(&t1.id).unwrap();
    let remove_event = events.iter().find(|e| e.event_type == "link_removed");
    assert!(remove_event.is_some());
    let event = remove_event.unwrap();
    assert_eq!(
        event.old_value.as_deref(),
        Some(&format!("related_to:{}", t2.id) as &str)
    );
    assert_eq!(event.new_value, "");
}

// --- CLI integration tests ---

#[test]
fn cli_link_add_and_list() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let t1 = create_task_via_cli(db_path, "Task Alpha");
    let t2 = create_task_via_cli(db_path, "Task Beta");

    cli_cmd(db_path)
        .args(["link", "add", &t1, "blocked-by", &t2])
        .assert()
        .success()
        .stdout(predicate::str::contains("Link created:"));

    cli_cmd(db_path)
        .args(["link", "list", &t1])
        .assert()
        .success()
        .stdout(predicate::str::contains("blocked_by"))
        .stdout(predicate::str::contains("Task Beta"))
        .stdout(predicate::str::contains("1 link(s)."));
}

#[test]
fn cli_link_inverse_display() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let t1 = create_task_via_cli(db_path, "Task Alpha");
    let t2 = create_task_via_cli(db_path, "Task Beta");

    cli_cmd(db_path)
        .args(["link", "add", &t1, "blocked-by", &t2])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["link", "list", &t2])
        .assert()
        .success()
        .stdout(predicate::str::contains("blocks"))
        .stdout(predicate::str::contains("Task Alpha"));
}

#[test]
fn cli_link_remove() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let t1 = create_task_via_cli(db_path, "Task Alpha");
    let t2 = create_task_via_cli(db_path, "Task Beta");

    let output = cli_cmd(db_path)
        .args(["link", "add", &t1, "related-to", &t2])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let link_short_id = stdout
        .split("Link created: ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();

    let db = Database::open(db_path).unwrap();
    let links = db.get_links(&t1).unwrap();
    let full_link_id = links[0].0.clone();

    cli_cmd(db_path)
        .args(["link", "remove", &full_link_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(link_short_id))
        .stdout(predicate::str::contains("removed"));

    cli_cmd(db_path)
        .args(["link", "list", &t1])
        .assert()
        .success()
        .stdout(predicate::str::contains("No links found"));
}

#[test]
fn cli_show_displays_links() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let t1 = create_task_via_cli(db_path, "Task Alpha");
    let t2 = create_task_via_cli(db_path, "Task Beta");

    cli_cmd(db_path)
        .args(["link", "add", &t1, "blocked-by", &t2])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["show", &t1])
        .assert()
        .success()
        .stdout(predicate::str::contains("Links:"))
        .stdout(predicate::str::contains("blocked_by"))
        .stdout(predicate::str::contains("Task Beta"));
}

#[test]
fn cli_show_omits_links_when_none() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let t1 = create_task_via_cli(db_path, "Solo task");

    cli_cmd(db_path)
        .args(["show", &t1])
        .assert()
        .success()
        .stdout(predicate::str::contains("Links:").not());
}

#[test]
fn cli_list_blocked_by_filter() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let blocker = create_task_via_cli(db_path, "Blocker");
    let blocked = create_task_via_cli(db_path, "Blocked");
    let _unrelated = create_task_via_cli(db_path, "Unrelated");

    cli_cmd(db_path)
        .args(["link", "add", &blocked, "blocked-by", &blocker])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["list", "--blocked-by", &blocker])
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocked"))
        .stdout(predicate::str::contains("Unrelated").not());
}

#[test]
fn cli_list_blocks_filter() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let t1 = create_task_via_cli(db_path, "Blocking task");
    let t2 = create_task_via_cli(db_path, "Downstream task");

    cli_cmd(db_path)
        .args(["link", "add", &t1, "blocks", &t2])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["list", "--blocks", &t2])
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocking ta"));
}

#[test]
fn cli_list_parent_filter_via_links() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let parent = create_task_via_cli(db_path, "Parent task");
    let child = create_task_via_cli(db_path, "Child task");

    cli_cmd(db_path)
        .args(["link", "add", &child, "parent", &parent])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["list", "--parent", &parent])
        .assert()
        .success()
        .stdout(predicate::str::contains("Child task"));
}

#[test]
fn cli_timeline_link_events() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let t1 = create_task_via_cli(db_path, "Task Alpha");
    let t2 = create_task_via_cli(db_path, "Task Beta");

    cli_cmd(db_path)
        .args(["link", "add", &t1, "blocked-by", &t2])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["history", &t1])
        .assert()
        .success()
        .stdout(predicate::str::contains("[link_added]"))
        .stdout(predicate::str::contains("blocked_by:"));

    let db = Database::open(db_path).unwrap();
    let links = db.get_links(&t1).unwrap();
    let full_link_id = links[0].0.clone();

    cli_cmd(db_path)
        .args(["link", "remove", &full_link_id])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["history", &t1])
        .assert()
        .success()
        .stdout(predicate::str::contains("[link_removed]"))
        .stdout(predicate::str::contains("blocked_by:"));
}

#[test]
fn cli_link_add_nonexistent_task() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let t1 = create_task_via_cli(db_path, "Real task");

    cli_cmd(db_path)
        .args(["link", "add", &t1, "blocked-by", "nonexistent-id"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn cli_link_remove_nonexistent() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let _t1 = create_task_via_cli(db_path, "Dummy");

    cli_cmd(db_path)
        .args(["link", "remove", "nonexistent-link-id"])
        .assert()
        .failure()
        .code(1);
}

// --- Migration test ---

#[test]
fn migration_parent_task_id_to_links() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    {
        let conn = rusqlite::Connection::open(db_path).unwrap();
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

             CREATE TABLE IF NOT EXISTS schema_versions (
                 version INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL
             );

             INSERT INTO schema_versions (version, applied_at) VALUES (1, '2025-01-01T00:00:00Z');

             INSERT INTO tasks (id, title, status, priority, tags, parent_task_id, created_at, updated_at)
             VALUES ('parent-1', 'Parent Task', 'open', 'high', '[]', NULL, '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z');

             INSERT INTO tasks (id, title, status, priority, tags, parent_task_id, created_at, updated_at)
             VALUES ('child-1', 'Child Task', 'open', 'medium', '[]', 'parent-1', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z');

             INSERT INTO tasks (id, title, status, priority, tags, parent_task_id, created_at, updated_at)
             VALUES ('orphan-1', 'Orphan Task', 'open', 'low', '[]', NULL, '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z');",
        )
        .unwrap();
    }

    let db = Database::open(db_path).unwrap();

    let links = db.get_links("child-1").unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].1, LinkType::Parent);
    assert_eq!(links[0].2, "parent-1");
    assert_eq!(links[0].3, "Parent Task");

    let inverse_links = db.get_links("parent-1").unwrap();
    assert_eq!(inverse_links.len(), 1);
    assert_eq!(inverse_links[0].1, LinkType::Child);
    assert_eq!(inverse_links[0].2, "child-1");

    let orphan_links = db.get_links("orphan-1").unwrap();
    assert!(orphan_links.is_empty());

    let children = db
        .list_tasks(
            None,
            None,
            None,
            None,
            Some("parent-1"),
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, "child-1");
}

// --- Short prefix resolution for unlink (INI-121) ---

#[test]
fn remove_link_with_4_char_prefix() {
    let db = test_db();
    let t1 = db
        .create_task(
            "A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "B",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    let link_id = db
        .create_link(&t1.id, &t2.id, &LinkType::RelatedTo)
        .unwrap();
    let prefix = &link_id[..4];

    let resolved = db.resolve_short_link_id(prefix).unwrap();
    assert_eq!(resolved, link_id);

    db.remove_link(&resolved).unwrap();
    assert!(db.get_links(&t1.id).unwrap().is_empty());
}

#[test]
fn remove_link_with_8_char_prefix() {
    let db = test_db();
    let t1 = db
        .create_task(
            "A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "B",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    let link_id = db.create_link(&t1.id, &t2.id, &LinkType::Blocks).unwrap();
    let prefix = &link_id[..8];

    let resolved = db.resolve_short_link_id(prefix).unwrap();
    assert_eq!(resolved, link_id);

    db.remove_link(&resolved).unwrap();
    assert!(db.get_links(&t1.id).unwrap().is_empty());
}

#[test]
fn remove_link_with_full_uuid() {
    let db = test_db();
    let t1 = db
        .create_task(
            "A",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "B",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    let link_id = db.create_link(&t1.id, &t2.id, &LinkType::Parent).unwrap();

    let resolved = db.resolve_short_link_id(&link_id).unwrap();
    assert_eq!(resolved, link_id);

    db.remove_link(&resolved).unwrap();
    assert!(db.get_links(&t1.id).unwrap().is_empty());
}

#[test]
fn resolve_short_link_id_too_short() {
    let db = test_db();
    let result = db.resolve_short_link_id("abc");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("at least 4 characters"));
}

#[test]
fn resolve_short_link_id_no_match() {
    let db = test_db();
    let result = db.resolve_short_link_id("zzzz");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No link found"));
}

#[test]
fn cli_link_remove_short_prefix() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let t1 = create_task_via_cli(db_path, "Task Alpha");
    let t2 = create_task_via_cli(db_path, "Task Beta");

    let db = Database::open(db_path).unwrap();
    let link_id = db.create_link(&t1, &t2, &LinkType::RelatedTo).unwrap();
    drop(db);

    let prefix_8 = &link_id[..8];
    cli_cmd(db_path)
        .args(["link", "remove", prefix_8])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed"));

    cli_cmd(db_path)
        .args(["link", "list", &t1])
        .assert()
        .success()
        .stdout(predicate::str::contains("No links found"));
}

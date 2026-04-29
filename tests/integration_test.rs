use task_management::db::Database;
use task_management::models::{TaskPriority, TaskStatus};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn test_db() -> Database {
    Database::open(":memory:").expect("open in-memory db")
}

#[test]
fn create_and_retrieve_task() {
    let db = test_db();
    let task = db
        .create_task(
            "Buy milk",
            Some("From the store"),
            TaskPriority::Low,
            Some("alice"),
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    assert_eq!(task.title, "Buy milk");
    assert_eq!(task.status, TaskStatus::Open);

    let fetched = db.get_task(&task.id).unwrap().expect("task should exist");
    assert_eq!(fetched.id, task.id);
    assert_eq!(fetched.title, "Buy milk");
    assert_eq!(fetched.description.as_deref(), Some("From the store"));
    assert_eq!(fetched.assignee.as_deref(), Some("alice"));
}

#[test]
fn update_task_status() {
    let db = test_db();
    let task = db
        .create_task(
            "Deploy v2",
            None,
            TaskPriority::High,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    assert_eq!(task.status, TaskStatus::Open);

    let updated = db
        .update_task(
            &task.id,
            None,
            None,
            Some(TaskStatus::InProgress),
            None,
            None,
            None,
            None,
        )
        .unwrap()
        .expect("task should exist");
    assert_eq!(updated.status, TaskStatus::InProgress);

    let fetched = db.get_task(&task.id).unwrap().expect("task should exist");
    assert_eq!(fetched.status, TaskStatus::InProgress);
}

#[test]
fn close_task() {
    let db = test_db();
    let task = db
        .create_task(
            "Fix bug",
            None,
            TaskPriority::Critical,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();
    let closed = db
        .close_task(&task.id, None)
        .unwrap()
        .expect("task should exist");
    assert_eq!(closed.status, TaskStatus::Closed);

    let fetched = db.get_task(&task.id).unwrap().expect("task should exist");
    assert_eq!(fetched.status, TaskStatus::Closed);
}

#[test]
fn list_all_tasks() {
    let db = test_db();
    db.create_task(
        "Task A",
        None,
        TaskPriority::Low,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Task B",
        None,
        TaskPriority::High,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Task C",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let all = db
        .list_tasks(None, None, None, None, None, None, None, None, 50, 0)
        .unwrap()
        .tasks;
    assert_eq!(all.len(), 3);
}

#[test]
fn list_tasks_filter_by_status() {
    let db = test_db();
    let t1 = db
        .create_task(
            "Open task",
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
        "Another open",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();
    db.update_task(
        &t1.id,
        None,
        None,
        Some(TaskStatus::Done),
        None,
        None,
        None,
        None,
    )
    .unwrap();

    let open = db
        .list_tasks(
            Some(TaskStatus::Open),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].title, "Another open");

    let done = db
        .list_tasks(
            Some(TaskStatus::Done),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].title, "Open task");
}

#[test]
fn list_tasks_filter_by_priority() {
    let db = test_db();
    db.create_task(
        "Low prio",
        None,
        TaskPriority::Low,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "High prio",
        None,
        TaskPriority::High,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Another high",
        None,
        TaskPriority::High,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let high = db
        .list_tasks(
            None,
            None,
            Some(TaskPriority::High),
            None,
            None,
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(high.len(), 2);
    for t in &high {
        assert_eq!(t.priority, TaskPriority::High);
    }
}

#[test]
fn list_tasks_filter_by_tag() {
    let db = test_db();
    db.create_task(
        "Tagged",
        None,
        TaskPriority::Medium,
        None,
        &["backend".into(), "rust".into()],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Other tag",
        None,
        TaskPriority::Medium,
        None,
        &["frontend".into()],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "No tags",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let backend = db
        .list_tasks(
            None,
            None,
            None,
            Some("backend"),
            None,
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(backend.len(), 1);
    assert_eq!(backend[0].title, "Tagged");

    let rust = db
        .list_tasks(
            None,
            None,
            None,
            Some("rust"),
            None,
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(rust.len(), 1);

    let frontend = db
        .list_tasks(
            None,
            None,
            None,
            Some("frontend"),
            None,
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(frontend.len(), 1);
    assert_eq!(frontend[0].title, "Other tag");
}

#[test]
fn list_tasks_filter_by_assignee() {
    let db = test_db();
    db.create_task(
        "Alice task",
        None,
        TaskPriority::Medium,
        Some("alice"),
        &[],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Bob task",
        None,
        TaskPriority::Medium,
        Some("bob"),
        &[],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Unassigned",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let alice = db
        .list_tasks(
            None,
            Some("alice"),
            None,
            None,
            None,
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(alice.len(), 1);
    assert_eq!(alice[0].title, "Alice task");
}

#[test]
fn list_tasks_filter_by_parent() {
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
    db.create_task(
        "Child 1",
        None,
        TaskPriority::Medium,
        None,
        &[],
        Some(&parent.id),
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Child 2",
        None,
        TaskPriority::Low,
        None,
        &[],
        Some(&parent.id),
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Orphan",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let children = db
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
    assert_eq!(children.len(), 2);
    for child in &children {
        assert_eq!(child.parent_task_id.as_deref(), Some(parent.id.as_str()));
    }
}

#[test]
fn create_task_with_parent() {
    let db = test_db();
    let parent = db
        .create_task(
            "Epic",
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
            "Sub-task",
            None,
            TaskPriority::Medium,
            None,
            &[],
            Some(&parent.id),
            None,
            "default",
        )
        .unwrap();
    assert_eq!(child.parent_task_id.as_deref(), Some(parent.id.as_str()));

    let fetched = db.get_task(&child.id).unwrap().expect("child should exist");
    assert_eq!(fetched.parent_task_id.as_deref(), Some(parent.id.as_str()));
}

#[test]
fn list_tasks_combined_filters() {
    let db = test_db();
    db.create_task(
        "Match",
        None,
        TaskPriority::High,
        Some("alice"),
        &["backend".into()],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Wrong assignee",
        None,
        TaskPriority::High,
        Some("bob"),
        &["backend".into()],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Wrong prio",
        None,
        TaskPriority::Low,
        Some("alice"),
        &["backend".into()],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db
        .list_tasks(
            None,
            Some("alice"),
            Some(TaskPriority::High),
            Some("backend"),
            None,
            None,
            None,
            None,
            50,
            0,
        )
        .unwrap()
        .tasks;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Match");
}

// --- CLI integration tests for notes/timeline ---

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

#[test]
fn note_creation() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let task_id = create_task_via_cli(db_path, "Test note task");

    cli_cmd(db_path)
        .args(["note", &task_id, "My test note", "--author", "alice"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Note ID:"))
        .stdout(predicate::str::contains(format!("Task:       {task_id}")))
        .stdout(predicate::str::contains("Author:     alice"))
        .stdout(predicate::str::contains("Body:       My test note"))
        .stdout(predicate::str::contains("Created:"));
}

#[test]
fn note_on_nonexistent_task() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    // Initialize the DB by creating and then using a fake id
    let _task_id = create_task_via_cli(db_path, "Dummy");

    let fake_id = "00000000-0000-0000-0000-000000000000";
    cli_cmd(db_path)
        .args(["note", fake_id, "Should fail"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(format!(
            "No task found matching prefix '{fake_id}'"
        )));
}

#[test]
fn history_with_events() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let task_id = create_task_via_cli(db_path, "History task");

    // Update status
    cli_cmd(db_path)
        .args(["update", &task_id, "--status", "in-progress"])
        .assert()
        .success();

    // Add a note
    cli_cmd(db_path)
        .args(["note", &task_id, "Working on it", "--author", "bob"])
        .assert()
        .success();

    // Check history
    cli_cmd(db_path)
        .args(["history", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "History for task {task_id}"
        )))
        .stdout(predicate::str::contains("[created]"))
        .stdout(predicate::str::contains("History task"))
        .stdout(predicate::str::contains("[status_changed]"))
        .stdout(predicate::str::contains("open"))
        .stdout(predicate::str::contains("in-progress"))
        .stdout(predicate::str::contains("[note_added]"))
        .stdout(predicate::str::contains("Working on it (by bob)"))
        .stdout(predicate::str::contains("3 event(s)"));
}

#[test]
fn history_format_correct() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let task_id = create_task_via_cli(db_path, "Format test");

    // A newly created task should have exactly 1 "created" event
    cli_cmd(db_path)
        .args(["history", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "History for task {task_id}"
        )))
        .stdout(predicate::str::contains("[created]"))
        .stdout(predicate::str::contains("Format test"))
        .stdout(predicate::str::contains("1 event(s)"))
        .stdout(predicate::str::contains("\u{2500}"));
}

#[test]
fn history_nonexistent_task() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let _task_id = create_task_via_cli(db_path, "Dummy");

    let fake_id = "00000000-0000-0000-0000-000000000000";
    cli_cmd(db_path)
        .args(["history", fake_id])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(format!(
            "No task found matching prefix '{fake_id}'"
        )));
}

#[test]
fn auto_tracking_status() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let task_id = create_task_via_cli(db_path, "Status tracking");

    cli_cmd(db_path)
        .args(["update", &task_id, "--status", "in-progress"])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["history", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("[status_changed]"))
        .stdout(predicate::str::contains("in-progress"));
}

#[test]
fn auto_tracking_assignee() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let task_id = create_task_via_cli(db_path, "Assignee tracking");

    cli_cmd(db_path)
        .args(["update", &task_id, "--assignee", "charlie"])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["history", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("[assignee_changed]"))
        .stdout(predicate::str::contains("charlie"));
}

#[test]
fn auto_tracking_priority() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let task_id = create_task_via_cli(db_path, "Priority tracking");

    cli_cmd(db_path)
        .args(["update", &task_id, "--priority", "critical"])
        .assert()
        .success();

    cli_cmd(db_path)
        .args(["history", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("[priority_changed]"))
        .stdout(predicate::str::contains("critical"));
}

// --- Namespace scoping tests ---

#[test]
fn test_create_task_with_namespace() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    let output = cli_cmd(db_path)
        .args([
            "--namespace",
            "ns-a",
            "--json",
            "create",
            "--title",
            "Namespaced task",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(val["title"].as_str().unwrap(), "Namespaced task");
    assert_eq!(val["namespace"].as_str().unwrap(), "ns-a");
}

#[test]
fn test_namespace_filtering_on_list() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    cli_cmd(db_path)
        .args(["--namespace", "ns-a", "create", "--title", "A1"])
        .assert()
        .success();
    cli_cmd(db_path)
        .args(["--namespace", "ns-a", "create", "--title", "A2"])
        .assert()
        .success();
    cli_cmd(db_path)
        .args(["--namespace", "ns-b", "create", "--title", "B1"])
        .assert()
        .success();

    let filtered = cli_cmd(db_path)
        .args(["--namespace", "ns-a", "--json", "list"])
        .output()
        .unwrap();
    assert!(filtered.status.success());
    let val: serde_json::Value = serde_json::from_slice(&filtered.stdout).unwrap();
    assert_eq!(val["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(val["total"].as_i64().unwrap(), 2);

    let all = cli_cmd(db_path).args(["--json", "list"]).output().unwrap();
    assert!(all.status.success());
    let val_all: serde_json::Value = serde_json::from_slice(&all.stdout).unwrap();
    assert_eq!(val_all["tasks"].as_array().unwrap().len(), 3);
    assert_eq!(val_all["total"].as_i64().unwrap(), 3);
}

#[test]
fn test_default_namespace() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    let output = cli_cmd(db_path)
        .args(["--json", "create", "--title", "Default ns task"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(val["namespace"].as_str().unwrap(), "default");
}

// --- Actor flag and --version tests ---

#[test]
fn update_with_actor_flag() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let task_id = create_task_via_cli(db_path, "Actor update test");

    cli_cmd(db_path)
        .args([
            "update",
            &task_id,
            "--status",
            "in-progress",
            "--actor",
            "agent-x",
        ])
        .assert()
        .success();

    let history_output = cli_cmd(db_path)
        .args(["--json", "history", &task_id])
        .output()
        .unwrap();
    assert!(history_output.status.success());
    let events: Vec<serde_json::Value> = serde_json::from_slice(&history_output.stdout).unwrap();
    let status_event = events
        .iter()
        .find(|e| e["event_type"].as_str() == Some("status_changed"))
        .expect("should have status_changed event");
    assert_eq!(status_event["actor"].as_str(), Some("agent-x"));
}

#[test]
fn close_with_actor_flag() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();
    let task_id = create_task_via_cli(db_path, "Actor close test");

    cli_cmd(db_path)
        .args(["close", &task_id, "--actor", "agent-x"])
        .assert()
        .success();

    let history_output = cli_cmd(db_path)
        .args(["--json", "history", &task_id])
        .output()
        .unwrap();
    assert!(history_output.status.success());
    let events: Vec<serde_json::Value> = serde_json::from_slice(&history_output.stdout).unwrap();
    let status_event = events
        .iter()
        .find(|e| e["event_type"].as_str() == Some("status_changed"))
        .expect("should have status_changed event");
    assert_eq!(status_event["actor"].as_str(), Some("agent-x"));
}

#[test]
fn version_flag() {
    let mut cmd = Command::cargo_bin("task-management").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.4.1"));
}

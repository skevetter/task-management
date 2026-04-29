use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn cli_cmd(db_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("task-management").unwrap();
    cmd.arg("--db").arg(db_path);
    cmd
}

fn create_task_cli(db_path: &str, title: &str) -> String {
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

// ─── (a) Status consistency — hyphenated format in JSON output ───

#[test]
fn status_in_progress_is_hyphenated_in_json() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Hyphenation test");

    cli_cmd(db)
        .args(["update", &task_id, "--status", "in-progress"])
        .assert()
        .success();

    let output = cli_cmd(db)
        .args(["--json", "show", &task_id])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(val["status"].as_str().unwrap(), "in-progress");
}

#[test]
fn list_filter_in_progress_returns_hyphenated_json() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "List hyphenation");

    cli_cmd(db)
        .args(["update", &task_id, "--status", "in-progress"])
        .assert()
        .success();

    let output = cli_cmd(db)
        .args(["--json", "list", "--status", "in-progress"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["status"].as_str().unwrap(), "in-progress");
}

// ─── (b) WAL mode verification ───

#[test]
fn database_uses_wal_journal_mode() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    create_task_cli(db_path, "WAL check task");

    let conn = rusqlite::Connection::open(db_path).unwrap();
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode, "wal");
}

// ─── (c) --actor flag on update and close ───

#[test]
fn update_actor_recorded_in_timeline() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Actor update test");

    cli_cmd(db)
        .args([
            "update",
            &task_id,
            "--status",
            "in-progress",
            "--actor",
            "agent-007",
        ])
        .assert()
        .success();

    let output = cli_cmd(db)
        .args(["--json", "history", &task_id])
        .output()
        .unwrap();
    assert!(output.status.success());
    let events: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();

    let status_event = events
        .iter()
        .find(|e| e["event_type"].as_str().unwrap() == "status_changed")
        .expect("should have a status_changed event");
    assert_eq!(status_event["actor"].as_str().unwrap(), "agent-007");
}

#[test]
fn close_actor_recorded_in_timeline() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Actor close test");

    cli_cmd(db)
        .args(["close", &task_id, "--actor", "closer-bot"])
        .assert()
        .success();

    let output = cli_cmd(db)
        .args(["--json", "history", &task_id])
        .output()
        .unwrap();
    assert!(output.status.success());
    let events: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();

    let status_event = events
        .iter()
        .find(|e| e["event_type"].as_str().unwrap() == "status_changed")
        .expect("should have a status_changed event from close");
    assert_eq!(status_event["actor"].as_str().unwrap(), "closer-bot");
}

// ─── (d) --version output ───

#[test]
fn version_flag_outputs_name_and_semver() {
    let mut cmd = Command::cargo_bin("task-management").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("task-management"))
        .stdout(predicate::str::is_match(r"\d+\.\d+\.\d+").unwrap());
}

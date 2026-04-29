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

fn create_task_json(db_path: &str, title: &str) -> serde_json::Value {
    let output = cli_cmd(db_path)
        .args(["--json", "create", "--title", title])
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("valid JSON from create --json")
}

// ─── JSON Output Tests ───

#[test]
fn json_create_has_expected_fields() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    let val = create_task_json(db, "JSON create test");
    assert!(val["id"].is_string());
    assert_eq!(val["title"].as_str().unwrap(), "JSON create test");
    assert_eq!(val["status"].as_str().unwrap(), "open");
    assert_eq!(val["priority"].as_str().unwrap(), "medium");
    assert!(val["created_at"].is_string());
    assert!(val["updated_at"].is_string());
}

#[test]
fn json_show_includes_detail() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Show detail task");

    cli_cmd(db)
        .args(["note", &task_id, "A note", "--author", "alice"])
        .assert()
        .success();

    let output = cli_cmd(db)
        .args(["--json", "show", &task_id])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(val["id"].as_str().unwrap(), task_id);
    assert_eq!(val["title"].as_str().unwrap(), "Show detail task");
    assert!(val["notes"].is_array());
    assert_eq!(val["notes"].as_array().unwrap().len(), 1);
    assert_eq!(val["notes"][0]["body"].as_str().unwrap(), "A note");
    assert!(val["timeline"].is_array());
    assert!(val["links"].is_array());
}

#[test]
fn json_list_is_array() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    create_task_cli(db, "List item A");
    create_task_cli(db, "List item B");

    let output = cli_cmd(db).args(["--json", "list"]).output().unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(val.is_object());
    let arr = val["tasks"].as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert!(arr[0]["id"].is_string());
    assert!(arr[0]["title"].is_string());
    assert_eq!(val["total"].as_i64().unwrap(), 2);
    assert_eq!(val["limit"].as_i64().unwrap(), 50);
    assert_eq!(val["offset"].as_i64().unwrap(), 0);
}

#[test]
fn json_update_returns_updated_task() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Update me");

    let output = cli_cmd(db)
        .args([
            "--json",
            "update",
            &task_id,
            "--status",
            "in-progress",
            "--title",
            "Updated title",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(val["id"].as_str().unwrap(), task_id);
    assert_eq!(val["title"].as_str().unwrap(), "Updated title");
    assert_eq!(val["status"].as_str().unwrap(), "in-progress");
}

#[test]
fn json_close_returns_closed_task() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Close me");

    let output = cli_cmd(db)
        .args(["--json", "close", &task_id])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(val["id"].as_str().unwrap(), task_id);
    assert_eq!(val["status"].as_str().unwrap(), "closed");
}

#[test]
fn json_note_returns_note_object() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Note task");

    let output = cli_cmd(db)
        .args(["--json", "note", &task_id, "Hello note", "--author", "bob"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(val["id"].is_string());
    assert_eq!(val["task_id"].as_str().unwrap(), task_id);
    assert_eq!(val["body"].as_str().unwrap(), "Hello note");
    assert_eq!(val["author"].as_str().unwrap(), "bob");
    assert!(val["created_at"].is_string());
}

#[test]
fn json_history_returns_event_array() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "History task");

    cli_cmd(db)
        .args(["update", &task_id, "--status", "in-progress"])
        .assert()
        .success();

    let output = cli_cmd(db)
        .args(["--json", "history", &task_id])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(val.is_array());
    let arr = val.as_array().unwrap();
    assert!(arr.len() >= 2);
    let event_types: Vec<&str> = arr
        .iter()
        .map(|e| e["event_type"].as_str().unwrap())
        .collect();
    assert!(event_types.contains(&"created"));
    assert!(event_types.contains(&"status_changed"));
    assert!(arr[0]["task_id"].is_string());
    assert!(arr[0]["occurred_at"].is_string());
}

#[test]
fn json_link_add_returns_link_object() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let t1 = create_task_cli(db, "Link source");
    let t2 = create_task_cli(db, "Link target");

    let output = cli_cmd(db)
        .args(["--json", "link", "add", &t1, "blocked-by", &t2])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(val["link_id"].is_string());
    assert_eq!(val["relationship"].as_str().unwrap(), "blocked_by");
    assert_eq!(val["related_task_id"].as_str().unwrap(), t2);
    assert_eq!(val["related_task_title"].as_str().unwrap(), "Link target");
}

#[test]
fn json_link_list_returns_array() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let t1 = create_task_cli(db, "Source");
    let t2 = create_task_cli(db, "Target");

    cli_cmd(db)
        .args(["link", "add", &t1, "related-to", &t2])
        .assert()
        .success();

    let output = cli_cmd(db)
        .args(["--json", "link", "list", &t1])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert!(val.is_array());
    let arr = val.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert!(arr[0]["link_id"].is_string());
    assert_eq!(arr[0]["relationship"].as_str().unwrap(), "related_to");
}

// ─── Short ID Prefix Tests ───

#[test]
fn short_id_exact_full_uuid_works() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Full UUID task");

    cli_cmd(db)
        .args(["show", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(&task_id));
}

#[test]
fn short_id_unique_4char_prefix_resolves() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Short prefix task");

    let prefix = &task_id[..8];

    cli_cmd(db)
        .args(["show", prefix])
        .assert()
        .success()
        .stdout(predicate::str::contains(&task_id))
        .stdout(predicate::str::contains("Short prefix task"));
}

#[test]
fn short_id_ambiguous_prefix_errors() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    let mut ids = Vec::new();
    for i in 0..20 {
        ids.push(create_task_cli(db, &format!("Ambiguity task {i}")));
    }

    let mut found_ambiguous = false;
    for id in &ids {
        let prefix = &id[..4];
        let matches: Vec<_> = ids
            .iter()
            .filter(|other| other.starts_with(prefix))
            .collect();
        if matches.len() >= 2 {
            cli_cmd(db)
                .args(["show", prefix])
                .assert()
                .failure()
                .code(1)
                .stderr(predicate::str::contains("Ambiguous prefix"));
            found_ambiguous = true;
            break;
        }
    }

    if !found_ambiguous {
        let first_prefix = &ids[0][..4];
        cli_cmd(db).args(["show", first_prefix]).assert().success();
    }
}

#[test]
fn short_id_too_short_prefix_errors() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let _task_id = create_task_cli(db, "Dummy for init");

    cli_cmd(db)
        .args(["show", "abc"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "Prefix must be at least 4 characters",
        ));
}

#[test]
fn short_id_prefix_works_with_update() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Update prefix task");

    let prefix = &task_id[..8];

    cli_cmd(db)
        .args(["update", prefix, "--title", "Updated via prefix"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated via prefix"));
}

#[test]
fn short_id_prefix_works_with_close() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Close prefix task");

    let prefix = &task_id[..8];

    cli_cmd(db)
        .args(["close", prefix])
        .assert()
        .success()
        .stdout(predicate::str::contains("closed"));
}

#[test]
fn short_id_prefix_works_with_note() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "Note prefix task");

    let prefix = &task_id[..8];

    cli_cmd(db)
        .args(["note", prefix, "Note via prefix"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Note ID:"));
}

#[test]
fn short_id_prefix_works_with_history() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let task_id = create_task_cli(db, "History prefix task");

    let prefix = &task_id[..8];

    cli_cmd(db)
        .args(["history", prefix])
        .assert()
        .success()
        .stdout(predicate::str::contains("History for task"))
        .stdout(predicate::str::contains("[created]"));
}

#[test]
fn short_id_nonexistent_prefix_errors() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let _task_id = create_task_cli(db, "Dummy");

    cli_cmd(db)
        .args(["show", "zzzz-nonexistent"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("No task found matching prefix"));
}

#[test]
fn json_list_empty_returns_empty_array() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    cli_cmd(db)
        .args(["--json", "create", "--title", "init"])
        .assert()
        .success();

    let output = cli_cmd(db)
        .args(["--json", "list", "--status", "blocked"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(val.is_object());
    assert_eq!(val["tasks"].as_array().unwrap().len(), 0);
    assert_eq!(val["total"].as_i64().unwrap(), 0);
}

#[test]
fn json_link_remove_returns_removed_id() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();
    let t1 = create_task_cli(db, "Rm source");
    let t2 = create_task_cli(db, "Rm target");

    let add_output = cli_cmd(db)
        .args(["--json", "link", "add", &t1, "related-to", &t2])
        .output()
        .unwrap();
    let add_val: serde_json::Value = serde_json::from_slice(&add_output.stdout).unwrap();
    let link_id = add_val["link_id"].as_str().unwrap();

    let rm_output = cli_cmd(db)
        .args(["--json", "link", "remove", link_id])
        .output()
        .unwrap();
    assert!(rm_output.status.success());
    let rm_val: serde_json::Value = serde_json::from_slice(&rm_output.stdout).unwrap();
    assert_eq!(rm_val["removed"].as_str().unwrap(), link_id);
}

// --- Pagination envelope and namespace filter tests ---

#[test]
fn test_json_list_pagination_envelope() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    for i in 0..5 {
        create_task_cli(db, &format!("Paginate {i}"));
    }

    let output = cli_cmd(db)
        .args(["--json", "list", "--limit", "2", "--offset", "0"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(val["total"].as_i64().unwrap(), 5);
    assert_eq!(val["limit"].as_i64().unwrap(), 2);
    assert_eq!(val["offset"].as_i64().unwrap(), 0);
    assert_eq!(val["tasks"].as_array().unwrap().len(), 2);
}

#[test]
fn test_json_list_pagination_offset() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    for i in 0..5 {
        create_task_cli(db, &format!("Offset {i}"));
    }

    let output = cli_cmd(db)
        .args(["--json", "list", "--limit", "2", "--offset", "2"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(val["total"].as_i64().unwrap(), 5);
    assert_eq!(val["offset"].as_i64().unwrap(), 2);
    assert_eq!(val["tasks"].as_array().unwrap().len(), 2);
}

#[test]
fn test_json_list_with_namespace_filter() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    for i in 0..3 {
        cli_cmd(db)
            .args([
                "--namespace",
                "ns-a",
                "create",
                "--title",
                &format!("NsA {i}"),
            ])
            .assert()
            .success();
    }
    cli_cmd(db)
        .args(["--namespace", "ns-b", "create", "--title", "NsB 0"])
        .assert()
        .success();

    let output = cli_cmd(db)
        .args(["--namespace", "ns-a", "--json", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(val["total"].as_i64().unwrap(), 3);
    assert_eq!(val["tasks"].as_array().unwrap().len(), 3);
    for task in val["tasks"].as_array().unwrap() {
        assert_eq!(task["namespace"].as_str().unwrap(), "ns-a");
    }
}

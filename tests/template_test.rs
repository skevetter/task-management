use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

use task_management::db::Database;
use task_management::models::TaskPriority;

fn test_db() -> Database {
    Database::open(":memory:").expect("open in-memory db")
}

fn cli_cmd(db_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("task-management").unwrap();
    cmd.arg("--db").arg(db_path);
    cmd
}

#[test]
fn list_templates_returns_builtins() {
    let db = test_db();
    let templates = db.list_templates().unwrap();
    assert_eq!(templates.len(), 3);
    let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"bug-report"));
    assert!(names.contains(&"feature-request"));
    assert!(names.contains(&"investigation"));
    assert!(templates.iter().all(|t| t.builtin));
}

#[test]
fn get_template_by_name() {
    let db = test_db();
    let tmpl = db.get_template("bug-report").unwrap().unwrap();
    assert_eq!(tmpl.title_pattern, "[Bug] {title}");
    assert_eq!(tmpl.default_priority.as_deref(), Some("high"));
    assert_eq!(tmpl.default_status.as_deref(), Some("open"));
    assert!(tmpl.builtin);
}

#[test]
fn create_and_delete_user_template() {
    let db = test_db();
    let tmpl = db
        .create_template(
            "chore",
            "[Chore] {title}",
            Some("low"),
            Some("open"),
            Some(&["maintenance".to_string()]),
        )
        .unwrap();
    assert_eq!(tmpl.name, "chore");
    assert!(!tmpl.builtin);
    assert_eq!(tmpl.default_tags, Some(vec!["maintenance".to_string()]));

    db.delete_template("chore").unwrap();
    assert!(db.get_template("chore").unwrap().is_none());
}

#[test]
fn delete_builtin_template_errors() {
    let db = test_db();
    let result = db.delete_template("bug-report");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot delete builtin"));
}

#[test]
fn create_task_from_template_applies_pattern() {
    let db = test_db();
    let task = db
        .create_task_from_template("bug-report", "Login fails", "default", Some("alice"))
        .unwrap();
    assert_eq!(task.title, "[Bug] Login fails");
    assert_eq!(task.priority, TaskPriority::High);
    assert_eq!(task.namespace, "default");
}

#[test]
fn cli_create_with_template_flag() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    let output = cli_cmd(db_path)
        .args(["--json", "create", "--title", "Login fails", "--template", "bug-report"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(val["title"].as_str().unwrap(), "[Bug] Login fails");
    assert_eq!(val["priority"].as_str().unwrap(), "high");
}

#[test]
fn cli_template_list() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    cli_cmd(db_path)
        .args(["template", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bug-report"))
        .stdout(predicate::str::contains("feature-request"))
        .stdout(predicate::str::contains("investigation"));
}

#[test]
fn cli_template_show() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    cli_cmd(db_path)
        .args(["template", "show", "bug-report"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Name:     bug-report"))
        .stdout(predicate::str::contains("Pattern:  [Bug] {title}"))
        .stdout(predicate::str::contains("Builtin:  true"));
}

#[test]
fn cli_template_create_and_delete() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    cli_cmd(db_path)
        .args(["template", "create", "--name", "hotfix", "--title-pattern", "[Hotfix] {title}", "--priority", "critical"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Template 'hotfix' created."));

    cli_cmd(db_path)
        .args(["template", "delete", "hotfix"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Template 'hotfix' deleted."));
}

#[test]
fn cli_template_delete_builtin_fails() {
    let tmp = NamedTempFile::new().unwrap();
    let db_path = tmp.path().to_str().unwrap();

    cli_cmd(db_path)
        .args(["template", "delete", "bug-report"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot delete builtin"));
}

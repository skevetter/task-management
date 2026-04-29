use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn cli_cmd(db_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("task-management").unwrap();
    cmd.arg("--db").arg(db_path);
    cmd
}

#[test]
fn template_list_shows_builtins() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    cli_cmd(db)
        .args(["template", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bug-report"))
        .stdout(predicate::str::contains("feature-request"))
        .stdout(predicate::str::contains("investigation"));
}

#[test]
fn template_show_displays_details() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    cli_cmd(db)
        .args(["template", "show", "bug-report"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bug-report"))
        .stdout(predicate::str::contains("[Bug] {title}"))
        .stdout(predicate::str::contains("high"));
}

#[test]
fn template_create_and_delete() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    cli_cmd(db)
        .args([
            "template",
            "create",
            "--name",
            "chore",
            "--title-pattern",
            "[Chore] {title}",
            "--priority",
            "low",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Template 'chore' created."));

    cli_cmd(db)
        .args(["--json", "template", "show", "chore"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\":\"chore\""));

    cli_cmd(db)
        .args(["template", "delete", "chore"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Template 'chore' deleted."));

    cli_cmd(db)
        .args(["template", "show", "chore"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Template not found"));
}

#[test]
fn template_delete_builtin_fails() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    cli_cmd(db)
        .args(["template", "delete", "bug-report"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot delete builtin"));
}

#[test]
fn create_task_with_template_flag() {
    let tmp = NamedTempFile::new().unwrap();
    let db = tmp.path().to_str().unwrap();

    let output = cli_cmd(db)
        .args([
            "--json",
            "create",
            "--title",
            "Login fails",
            "--template",
            "bug-report",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(val["title"].as_str().unwrap(), "[Bug] Login fails");
    assert_eq!(val["priority"].as_str().unwrap(), "high");
}

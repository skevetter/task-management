use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn task_cmd() -> Command {
    Command::cargo_bin("task-management").unwrap()
}

#[test]
fn xdg_data_home_sets_default_path() {
    let tmp = TempDir::new().unwrap();
    let xdg_data = tmp.path().join("xdg-data");

    task_cmd()
        .env("XDG_DATA_HOME", &xdg_data)
        .env_remove("HOME")
        .args(["create", "--title", "XDG test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Title:"));

    let db_path = xdg_data.join("task-management").join("tasks.db");
    assert!(db_path.exists(), "DB should be at {}", db_path.display());
}

#[test]
fn fallback_to_home_local_share() {
    let tmp = TempDir::new().unwrap();
    let fake_home = tmp.path().join("fakehome");

    task_cmd()
        .env_remove("XDG_DATA_HOME")
        .env("HOME", &fake_home)
        .args(["create", "--title", "Home fallback test"])
        .assert()
        .success();

    let db_path = fake_home
        .join(".local")
        .join("share")
        .join("task-management")
        .join("tasks.db");
    assert!(db_path.exists(), "DB should be at {}", db_path.display());
}

#[test]
fn db_flag_overrides_xdg() {
    let tmp = TempDir::new().unwrap();
    let xdg_data = tmp.path().join("xdg-data");
    let explicit_db = tmp.path().join("custom").join("override.db");

    task_cmd()
        .env("XDG_DATA_HOME", &xdg_data)
        .args([
            "--db",
            explicit_db.to_str().unwrap(),
            "create",
            "--title",
            "Override test",
        ])
        .assert()
        .success();

    assert!(explicit_db.exists(), "DB should be at explicit path");
    let xdg_db = xdg_data.join("task-management").join("tasks.db");
    assert!(
        !xdg_db.exists(),
        "XDG path should NOT be used when --db is set"
    );
}

#[test]
fn directory_auto_created() {
    let tmp = TempDir::new().unwrap();
    let nested = tmp.path().join("a").join("b").join("c").join("tasks.db");

    assert!(!nested.parent().unwrap().exists());

    task_cmd()
        .args([
            "--db",
            nested.to_str().unwrap(),
            "create",
            "--title",
            "Auto-dir test",
        ])
        .assert()
        .success();

    assert!(nested.exists(), "DB and parent dirs should be created");
}

#[test]
fn empty_xdg_data_home_treated_as_unset() {
    let tmp = TempDir::new().unwrap();
    let fake_home = tmp.path().join("fakehome");

    task_cmd()
        .env("XDG_DATA_HOME", "")
        .env("HOME", &fake_home)
        .args(["create", "--title", "Empty XDG test"])
        .assert()
        .success();

    let db_path = fake_home
        .join(".local")
        .join("share")
        .join("task-management")
        .join("tasks.db");
    assert!(
        db_path.exists(),
        "Empty XDG_DATA_HOME should fall back to HOME: {}",
        db_path.display()
    );
}

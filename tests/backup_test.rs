use rusqlite::Connection;
use std::path::Path;
use task_management::db::Database;
use tempfile::TempDir;

#[test]
fn backup_created_for_existing_db() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("tasks.db");
    let db_path_str = db_path.to_str().unwrap();

    let _db1 = Database::open(db_path_str).unwrap();
    drop(_db1);

    let bak_path = format!("{}.bak", db_path_str);
    assert!(
        !Path::new(&bak_path).exists(),
        "no backup on first open (new DB)"
    );

    let _db2 = Database::open(db_path_str).unwrap();
    drop(_db2);

    assert!(
        Path::new(&bak_path).exists(),
        "backup should exist after opening existing DB"
    );
}

#[test]
fn no_backup_for_new_db() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("brand_new.db");
    let db_path_str = db_path.to_str().unwrap();

    assert!(!db_path.exists(), "DB file should not exist yet");

    let _db = Database::open(db_path_str).unwrap();
    drop(_db);

    let bak_path = format!("{}.bak", db_path_str);
    assert!(
        !Path::new(&bak_path).exists(),
        "no backup for a brand new DB"
    );
}

#[test]
fn backup_is_valid_sqlite() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("tasks.db");
    let db_path_str = db_path.to_str().unwrap();

    let db1 = Database::open(db_path_str).unwrap();
    db1.create_task(
        "Test task",
        None,
        task_management::models::TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();
    drop(db1);

    let _db2 = Database::open(db_path_str).unwrap();
    drop(_db2);

    let bak_path = format!("{}.bak", db_path_str);
    let conn = Connection::open(&bak_path).expect("backup should be a valid SQLite DB");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
        .expect("should be able to query tasks table in backup");
    assert!(count >= 1, "backup should contain the task we created");
}

#[test]
fn no_backup_for_memory_db() {
    let _db = Database::open(":memory:").unwrap();
    assert!(
        !Path::new(":memory:.bak").exists(),
        "no backup for in-memory DB"
    );
}

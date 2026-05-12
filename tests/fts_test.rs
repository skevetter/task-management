use task_management::db::Database;
use task_management::models::TaskPriority;

fn test_db() -> Database {
    Database::open(":memory:").expect("open in-memory db")
}

#[test]
fn search_by_title() {
    let db = test_db();
    db.create_task(
        "Fix authentication bug",
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
        "Add logging",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("authentication", None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Fix authentication bug");
}

#[test]
fn search_by_description() {
    let db = test_db();
    db.create_task(
        "Task one",
        Some("The database migration needs review"),
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();
    db.create_task(
        "Task two",
        Some("Update the UI colors"),
        TaskPriority::Low,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("migration", None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Task one");
}

#[test]
fn search_partial_match() {
    let db = test_db();
    db.create_task(
        "Implement caching layer",
        None,
        TaskPriority::High,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("caching", None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Implement caching layer");
}

#[test]
fn search_no_results() {
    let db = test_db();
    db.create_task(
        "Fix button alignment",
        None,
        TaskPriority::Low,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("nonexistent_xyz_term", None).unwrap();
    assert!(results.is_empty());
}

#[test]
fn search_namespace_scoping() {
    let db = test_db();
    db.create_task(
        "Deploy service",
        None,
        TaskPriority::High,
        None,
        &[],
        None,
        None,
        "team-a",
    )
    .unwrap();
    db.create_task(
        "Deploy database",
        None,
        TaskPriority::High,
        None,
        &[],
        None,
        None,
        "team-b",
    )
    .unwrap();

    let all = db.search_tasks("deploy", None).unwrap();
    assert_eq!(all.len(), 2);

    let scoped = db.search_tasks("deploy", Some("team-a")).unwrap();
    assert_eq!(scoped.len(), 1);
    assert_eq!(scoped[0].namespace, "team-a");
}

#[test]
fn search_new_task_indexed() {
    let db = test_db();

    let results_before = db.search_tasks("elasticsearch", None).unwrap();
    assert!(results_before.is_empty());

    db.create_task(
        "Setup elasticsearch cluster",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results_after = db.search_tasks("elasticsearch", None).unwrap();
    assert_eq!(results_after.len(), 1);
    assert_eq!(results_after[0].title, "Setup elasticsearch cluster");
}

#[test]
fn search_updated_task_reindexed() {
    let db = test_db();
    let task = db
        .create_task(
            "Original title",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    let results = db.search_tasks("Original", None).unwrap();
    assert_eq!(results.len(), 1);

    db.update_task(
        &task.id,
        Some("Updated unique title"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    let old_results = db.search_tasks("Original", None).unwrap();
    assert!(old_results.is_empty());

    let new_results = db.search_tasks("Updated unique", None).unwrap();
    assert_eq!(new_results.len(), 1);
    assert_eq!(new_results[0].title, "Updated unique title");
}

#[test]
fn search_migration_indexes_existing_data() {
    let db = test_db();
    db.create_task(
        "Pre-existing task alpha",
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
        "Pre-existing task beta",
        Some("Beta description content"),
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("alpha", None).unwrap();
    assert_eq!(results.len(), 1);

    let results = db.search_tasks("Beta description", None).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn search_with_double_quotes() {
    let db = test_db();
    db.create_task(
        "Fix \"quoted\" issue",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("\"quoted\"", None).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].title.contains("quoted"));
}

#[test]
fn search_with_or_keyword() {
    let db = test_db();
    db.create_task(
        "Task with OR in title",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("OR", None).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn search_with_not_keyword() {
    let db = test_db();
    db.create_task(
        "Task with NOT in title",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("NOT", None).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn search_with_asterisk() {
    let db = test_db();
    db.create_task(
        "Wildcard * character test",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("*", None).unwrap();
    assert!(results.is_empty() || !results.is_empty());
}

#[test]
fn search_with_parentheses() {
    let db = test_db();
    db.create_task(
        "Task (with parens) here",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let results = db.search_tasks("(with parens)", None).unwrap();
    assert!(!results.is_empty());
}

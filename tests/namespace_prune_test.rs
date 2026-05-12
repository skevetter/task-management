use task_management::db::Database;
use task_management::models::TaskPriority;

fn test_db() -> Database {
    Database::open(":memory:").expect("open in-memory db")
}

#[test]
fn list_namespaces_correct_counts() {
    let db = test_db();
    for _ in 0..3 {
        db.create_task(
            "t",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "alpha",
        )
        .unwrap();
    }
    db.create_task(
        "t",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "beta",
    )
    .unwrap();

    let ns = db.list_namespaces().unwrap();
    assert_eq!(ns.len(), 2);
    assert_eq!(ns[0].namespace, "alpha");
    assert_eq!(ns[0].task_count, 3);
    assert_eq!(ns[1].namespace, "beta");
    assert_eq!(ns[1].task_count, 1);
}

#[test]
fn list_namespaces_last_activity() {
    let db = test_db();
    let task = db
        .create_task(
            "first",
            None,
            TaskPriority::Medium,
            None,
            &[],
            None,
            None,
            "ns1",
        )
        .unwrap();
    let initial_updated = task.updated_at.clone();

    std::thread::sleep(std::time::Duration::from_millis(10));
    db.update_task(
        &task.id,
        Some("updated"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    let ns = db.list_namespaces().unwrap();
    assert_eq!(ns.len(), 1);
    assert!(ns[0].last_activity > initial_updated);
}

#[test]
fn prune_closes_stale_tasks() {
    let db = test_db();
    let task = db
        .create_task(
            "old task",
            None,
            TaskPriority::Low,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    // Manually backdate the task to make it stale
    db.conn_for_test()
        .execute(
            "UPDATE tasks SET updated_at = datetime('now', '-60 days') WHERE id = ?1",
            rusqlite::params![task.id],
        )
        .unwrap();

    let pruned = db.prune_stale_tasks(30, None, None).unwrap();
    assert_eq!(pruned.len(), 1);
    assert_eq!(pruned[0], task.id);

    let updated = db.get_task(&task.id).unwrap().unwrap();
    assert_eq!(updated.status.to_string(), "cancelled");
}

#[test]
fn prune_does_not_close_recent_tasks() {
    let db = test_db();
    db.create_task(
        "recent task",
        None,
        TaskPriority::Medium,
        None,
        &[],
        None,
        None,
        "default",
    )
    .unwrap();

    let pruned = db.prune_stale_tasks(30, None, None).unwrap();
    assert!(pruned.is_empty());
}

#[test]
fn prune_respects_namespace_filter() {
    let db = test_db();
    let t1 = db
        .create_task(
            "old in alpha",
            None,
            TaskPriority::Low,
            None,
            &[],
            None,
            None,
            "alpha",
        )
        .unwrap();
    let t2 = db
        .create_task(
            "old in beta",
            None,
            TaskPriority::Low,
            None,
            &[],
            None,
            None,
            "beta",
        )
        .unwrap();

    // Backdate both
    db.conn_for_test()
        .execute(
            "UPDATE tasks SET updated_at = datetime('now', '-60 days') WHERE id IN (?1, ?2)",
            rusqlite::params![t1.id, t2.id],
        )
        .unwrap();

    let pruned = db.prune_stale_tasks(30, Some("alpha"), None).unwrap();
    assert_eq!(pruned.len(), 1);
    assert_eq!(pruned[0], t1.id);

    // beta task should still be open
    let beta_task = db.get_task(&t2.id).unwrap().unwrap();
    assert_eq!(beta_task.status.to_string(), "open");
}

#[test]
fn prune_records_timeline_events() {
    let db = test_db();
    let task = db
        .create_task(
            "stale task",
            None,
            TaskPriority::Low,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    db.conn_for_test()
        .execute(
            "UPDATE tasks SET updated_at = datetime('now', '-60 days') WHERE id = ?1",
            rusqlite::params![task.id],
        )
        .unwrap();

    db.prune_stale_tasks(30, None, None).unwrap();

    let events = db.get_timeline(&task.id).unwrap();
    let status_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "status_changed")
        .collect();
    assert_eq!(status_events.len(), 1);
    assert_eq!(status_events[0].old_value.as_deref(), Some("open"));
    assert_eq!(status_events[0].new_value, "cancelled");
}

#[test]
fn prune_with_actor_records_actor_on_timeline() {
    let db = test_db();
    let task = db
        .create_task(
            "stale task",
            None,
            TaskPriority::Low,
            None,
            &[],
            None,
            None,
            "default",
        )
        .unwrap();

    db.conn_for_test()
        .execute(
            "UPDATE tasks SET updated_at = datetime('now', '-60 days') WHERE id = ?1",
            rusqlite::params![task.id],
        )
        .unwrap();

    db.prune_stale_tasks(30, None, Some("cleanup-bot")).unwrap();

    let events = db.get_timeline(&task.id).unwrap();
    let status_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "status_changed")
        .collect();
    assert_eq!(status_events.len(), 1);
    assert_eq!(status_events[0].actor.as_deref(), Some("cleanup-bot"));
}

#[test]
fn prune_rejects_zero_stale_days() {
    let db = test_db();
    let result = db.prune_stale_tasks(0, None, None);
    assert!(result.is_err());
}

#[test]
fn prune_rejects_negative_stale_days() {
    let db = test_db();
    let result = db.prune_stale_tasks(-1, None, None);
    assert!(result.is_err());
}

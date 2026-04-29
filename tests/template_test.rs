use task_management::db::Database;
use task_management::models::TaskPriority;

fn test_db() -> Database {
    Database::open(":memory:").expect("open in-memory db")
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

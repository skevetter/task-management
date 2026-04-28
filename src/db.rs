use chrono::Utc;
use rusqlite::{Connection, Result, params};
use uuid::Uuid;

use crate::models::{Task, TaskPriority, TaskStatus};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL DEFAULT 'open',
                priority TEXT NOT NULL DEFAULT 'medium',
                assignee TEXT,
                tags TEXT NOT NULL DEFAULT '[]',
                parent_task_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    pub fn create_task(
        &self,
        title: &str,
        description: Option<&str>,
        priority: TaskPriority,
        assignee: Option<&str>,
        tags: &[String],
        parent_task_id: Option<&str>,
    ) -> Result<Task> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());

        self.conn.execute(
            "INSERT INTO tasks (id, title, description, status, priority, assignee, tags, parent_task_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                title,
                description,
                TaskStatus::Open.to_string(),
                priority.to_string(),
                assignee,
                tags_json,
                parent_task_id,
                now,
                now,
            ],
        )?;

        Ok(Task {
            id,
            title: title.to_string(),
            description: description.map(String::from),
            status: TaskStatus::Open,
            priority,
            assignee: assignee.map(String::from),
            tags: tags.to_vec(),
            parent_task_id: parent_task_id.map(String::from),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, description, status, priority, assignee, tags, parent_task_id, created_at, updated_at
             FROM tasks WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            let status_str: String = row.get(3)?;
            let priority_str: String = row.get(4)?;
            let tags_str: String = row.get(6)?;

            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: status_str.parse::<TaskStatus>().unwrap_or(TaskStatus::Open),
                priority: priority_str
                    .parse::<TaskPriority>()
                    .unwrap_or(TaskPriority::Medium),
                assignee: row.get(5)?,
                tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                parent_task_id: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        match rows.next() {
            Some(Ok(task)) => Ok(Some(task)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_task(
        &self,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<TaskStatus>,
        priority: Option<TaskPriority>,
        assignee: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<Option<Task>> {
        let existing = self.get_task(id)?;
        let Some(task) = existing else {
            return Ok(None);
        };

        let now = Utc::now().to_rfc3339();
        let new_title = title.unwrap_or(&task.title);
        let new_description = description.or(task.description.as_deref());
        let new_status = status.unwrap_or(task.status);
        let new_priority = priority.unwrap_or(task.priority);
        let new_assignee = assignee.or(task.assignee.as_deref());
        let new_tags = tags.map(|t| t.to_vec()).unwrap_or(task.tags.clone());
        let tags_json = serde_json::to_string(&new_tags).unwrap_or_else(|_| "[]".to_string());

        self.conn.execute(
            "UPDATE tasks SET title = ?1, description = ?2, status = ?3, priority = ?4, assignee = ?5, tags = ?6, updated_at = ?7
             WHERE id = ?8",
            params![
                new_title,
                new_description,
                new_status.to_string(),
                new_priority.to_string(),
                new_assignee,
                tags_json,
                now,
                id,
            ],
        )?;

        Ok(Some(Task {
            id: id.to_string(),
            title: new_title.to_string(),
            description: new_description.map(String::from),
            status: new_status,
            priority: new_priority,
            assignee: new_assignee.map(String::from),
            tags: new_tags,
            parent_task_id: task.parent_task_id,
            created_at: task.created_at,
            updated_at: now,
        }))
    }

    pub fn close_task(&self, id: &str) -> Result<Option<Task>> {
        self.update_task(id, None, None, Some(TaskStatus::Closed), None, None, None)
    }
}

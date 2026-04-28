use chrono::Utc;
use rusqlite::{Connection, Result, params};
use uuid::Uuid;

use crate::models::{Task, TaskNote, TaskPriority, TaskStatus, TimelineEvent};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;

             CREATE TABLE IF NOT EXISTS tasks (
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
             );

             CREATE TABLE IF NOT EXISTS task_notes (
                 id TEXT PRIMARY KEY,
                 task_id TEXT NOT NULL,
                 body TEXT NOT NULL,
                 author TEXT,
                 created_at TEXT NOT NULL,
                 FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_task_notes_task_id ON task_notes (task_id);

             CREATE TABLE IF NOT EXISTS timeline_events (
                 id TEXT PRIMARY KEY,
                 task_id TEXT NOT NULL,
                 event_type TEXT NOT NULL,
                 old_value TEXT,
                 new_value TEXT,
                 actor TEXT,
                 occurred_at TEXT NOT NULL,
                 FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_timeline_task_id ON timeline_events (task_id);
             CREATE INDEX IF NOT EXISTS idx_timeline_occurred ON timeline_events (occurred_at);

             CREATE TABLE IF NOT EXISTS schema_versions (
                 version INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL
             );",
        )?;

        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO schema_versions (version, applied_at) VALUES (1, ?1)",
            params![now],
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

        self.insert_timeline_event(&id, "created", None, Some(title), None, &now)?;

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

        if task.status != new_status {
            self.insert_timeline_event(
                id,
                "status_changed",
                Some(&task.status.to_string()),
                Some(&new_status.to_string()),
                None,
                &now,
            )?;
        }
        if task.assignee.as_deref() != new_assignee {
            self.insert_timeline_event(
                id,
                "assignee_changed",
                task.assignee.as_deref(),
                new_assignee,
                None,
                &now,
            )?;
        }
        if task.priority != new_priority {
            self.insert_timeline_event(
                id,
                "priority_changed",
                Some(&task.priority.to_string()),
                Some(&new_priority.to_string()),
                None,
                &now,
            )?;
        }

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

    pub fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        assignee: Option<&str>,
        priority: Option<TaskPriority>,
        tag: Option<&str>,
        parent: Option<&str>,
    ) -> Result<Vec<Task>> {
        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(s) = status {
            conditions.push("status = ?".to_string());
            param_values.push(Box::new(s.to_string()));
        }
        if let Some(a) = assignee {
            conditions.push("assignee = ?".to_string());
            param_values.push(Box::new(a.to_string()));
        }
        if let Some(p) = priority {
            conditions.push("priority = ?".to_string());
            param_values.push(Box::new(p.to_string()));
        }
        if let Some(t) = tag {
            let pattern = format!("%\"{t}\"%");
            conditions.push("tags LIKE ?".to_string());
            param_values.push(Box::new(pattern));
        }
        if let Some(pid) = parent {
            conditions.push("parent_task_id = ?".to_string());
            param_values.push(Box::new(pid.to_string()));
        }

        let mut sql =
            "SELECT id, title, description, status, priority, assignee, tags, parent_task_id, created_at, updated_at FROM tasks"
                .to_string();
        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }
        sql.push_str(" ORDER BY created_at DESC");

        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), |row| {
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

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    fn insert_timeline_event(
        &self,
        task_id: &str,
        event_type: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
        actor: Option<&str>,
        occurred_at: &str,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO timeline_events (id, task_id, event_type, old_value, new_value, actor, occurred_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, task_id, event_type, old_value, new_value, actor, occurred_at],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn add_note(&self, task_id: &str, body: &str, author: Option<&str>) -> Result<TaskNote> {
        self.get_task(task_id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        self.conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> Result<()> {
            self.conn.execute(
                "INSERT INTO task_notes (id, task_id, body, author, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, task_id, body, author, now],
            )?;
            self.insert_timeline_event(task_id, "note_added", None, Some(body), author, &now)?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.conn.execute("COMMIT", [])?;
            }
            Err(e) => {
                let _ = self.conn.execute("ROLLBACK", []);
                return Err(e);
            }
        }

        Ok(TaskNote {
            id,
            task_id: task_id.to_string(),
            body: body.to_string(),
            author: author.map(String::from),
            created_at: now,
        })
    }

    #[allow(dead_code)]
    pub fn get_notes(&self, task_id: &str) -> Result<Vec<TaskNote>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, body, author, created_at
             FROM task_notes
             WHERE task_id = ?1
             ORDER BY created_at ASC",
        )?;

        let rows = stmt.query_map(params![task_id], |row| {
            Ok(TaskNote {
                id: row.get(0)?,
                task_id: row.get(1)?,
                body: row.get(2)?,
                author: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut notes = Vec::new();
        for row in rows {
            notes.push(row?);
        }
        Ok(notes)
    }

    #[allow(dead_code)]
    pub fn get_timeline(&self, task_id: &str) -> Result<Vec<TimelineEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, event_type, old_value, new_value, actor, occurred_at
             FROM timeline_events
             WHERE task_id = ?1
             ORDER BY occurred_at ASC",
        )?;

        let rows = stmt.query_map(params![task_id], |row| {
            Ok(TimelineEvent {
                id: row.get(0)?,
                task_id: row.get(1)?,
                event_type: row.get(2)?,
                old_value: row.get(3)?,
                new_value: row.get(4)?,
                actor: row.get(5)?,
                occurred_at: row.get(6)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }
}

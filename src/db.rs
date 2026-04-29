use chrono::Utc;
use rusqlite::{Connection, Result, params};
use uuid::Uuid;

use crate::models::{LinkType, Task, TaskNote, TaskPriority, TaskStatus, TimelineEvent};

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
                 new_value TEXT NOT NULL,
                 actor TEXT,
                 occurred_at TEXT NOT NULL,
                 FOREIGN KEY (task_id) REFERENCES tasks (id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_timeline_task_id ON timeline_events (task_id);
             CREATE INDEX IF NOT EXISTS idx_timeline_occurred ON timeline_events (occurred_at);

             CREATE TABLE IF NOT EXISTS task_links (
                 id         TEXT PRIMARY KEY,
                 source_id  TEXT NOT NULL,
                 target_id  TEXT NOT NULL,
                 link_type  TEXT NOT NULL,
                 created_at TEXT NOT NULL,
                 FOREIGN KEY (source_id) REFERENCES tasks (id) ON DELETE CASCADE,
                 FOREIGN KEY (target_id) REFERENCES tasks (id) ON DELETE CASCADE
             );
             CREATE INDEX IF NOT EXISTS idx_task_links_source ON task_links (source_id);
             CREATE INDEX IF NOT EXISTS idx_task_links_target ON task_links (target_id);

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

        let max_version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_versions",
            [],
            |row| row.get(0),
        )?;

        if max_version < 2 {
            conn.execute_batch(
                "BEGIN IMMEDIATE;
                 INSERT INTO task_links (id, source_id, target_id, link_type, created_at)
                 SELECT
                     lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' ||
                     substr(lower(hex(randomblob(2))),2) || '-' ||
                     substr('89ab', abs(random()) % 4 + 1, 1) ||
                     substr(lower(hex(randomblob(2))),2) || '-' || lower(hex(randomblob(6))),
                     id,
                     parent_task_id,
                     'parent',
                     created_at
                 FROM tasks
                 WHERE parent_task_id IS NOT NULL;
                 INSERT OR IGNORE INTO schema_versions (version, applied_at)
                 VALUES (2, datetime('now'));
                 COMMIT;",
            )?;
        }

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

        self.conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> Result<()> {
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

            self.insert_timeline_event(&id, "created", None, title, None, &now)?;

            if let Some(pid) = parent_task_id {
                let link_id = Uuid::new_v4().to_string();
                self.conn.execute(
                    "INSERT INTO task_links (id, source_id, target_id, link_type, created_at)
                     VALUES (?1, ?2, ?3, 'parent', ?4)",
                    params![link_id, &id, pid, &now],
                )?;
                let new_value = format!("parent:{pid}");
                self.insert_timeline_event(&id, "link_added", None, &new_value, None, &now)?;
            }

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

    pub fn resolve_short_id(&self, prefix: &str) -> std::result::Result<String, String> {
        if prefix.len() < 4 {
            return Err("Prefix must be at least 4 characters".to_string());
        }
        let pattern = format!("{prefix}%");
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM tasks WHERE id LIKE ?1")
            .map_err(|e| e.to_string())?;
        let ids: Vec<String> = stmt
            .query_map(params![pattern], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        match ids.len() {
            0 => Err(format!("No task found matching prefix '{prefix}'")),
            1 => Ok(ids.into_iter().next().unwrap()),
            _ => Err(format!(
                "Ambiguous prefix '{}' matches: {}",
                prefix,
                ids.join(", ")
            )),
        }
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
                &new_status.to_string(),
                None,
                &now,
            )?;
        }
        if task.assignee.as_deref() != new_assignee {
            self.insert_timeline_event(
                id,
                "assignee_changed",
                task.assignee.as_deref(),
                new_assignee.unwrap_or(""),
                None,
                &now,
            )?;
        }
        if task.priority != new_priority {
            self.insert_timeline_event(
                id,
                "priority_changed",
                Some(&task.priority.to_string()),
                &new_priority.to_string(),
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

    #[allow(clippy::too_many_arguments)]
    pub fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        assignee: Option<&str>,
        priority: Option<TaskPriority>,
        tag: Option<&str>,
        parent: Option<&str>,
        blocked_by: Option<&str>,
        blocks: Option<&str>,
    ) -> Result<Vec<Task>> {
        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(s) = status {
            conditions.push("t.status = ?".to_string());
            param_values.push(Box::new(s.to_string()));
        }
        if let Some(a) = assignee {
            conditions.push("t.assignee = ?".to_string());
            param_values.push(Box::new(a.to_string()));
        }
        if let Some(p) = priority {
            conditions.push("t.priority = ?".to_string());
            param_values.push(Box::new(p.to_string()));
        }
        if let Some(t) = tag {
            let pattern = format!("%\"{t}\"%");
            conditions.push("t.tags LIKE ?".to_string());
            param_values.push(Box::new(pattern));
        }
        if let Some(pid) = parent {
            conditions.push(
                "t.id IN (SELECT source_id FROM task_links WHERE link_type = 'parent' AND target_id = ?)"
                    .to_string(),
            );
            param_values.push(Box::new(pid.to_string()));
        }
        if let Some(bid) = blocked_by {
            conditions.push(
                "t.id IN (SELECT source_id FROM task_links WHERE link_type = 'blocked_by' AND target_id = ?)"
                    .to_string(),
            );
            param_values.push(Box::new(bid.to_string()));
        }
        if let Some(bid) = blocks {
            conditions.push(
                "t.id IN (SELECT source_id FROM task_links WHERE link_type = 'blocks' AND target_id = ?)"
                    .to_string(),
            );
            param_values.push(Box::new(bid.to_string()));
        }

        let mut sql =
            "SELECT t.id, t.title, t.description, t.status, t.priority, t.assignee, t.tags, t.parent_task_id, t.created_at, t.updated_at FROM tasks t"
                .to_string();
        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }
        sql.push_str(" ORDER BY t.created_at DESC");

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
        new_value: &str,
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
            self.insert_timeline_event(task_id, "note_added", None, body, author, &now)?;
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

    pub fn create_link(
        &self,
        source_id: &str,
        target_id: &str,
        link_type: &LinkType,
    ) -> Result<String> {
        self.get_task(source_id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        self.get_task(target_id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        self.conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> Result<()> {
            self.conn.execute(
                "INSERT INTO task_links (id, source_id, target_id, link_type, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, source_id, target_id, link_type.to_string(), now],
            )?;
            let new_value = format!("{link_type}:{target_id}");
            self.insert_timeline_event(source_id, "link_added", None, &new_value, None, &now)?;
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

        Ok(id)
    }

    pub fn remove_link(&self, link_id: &str) -> Result<()> {
        let link: (String, String, String) = self.conn.query_row(
            "SELECT source_id, target_id, link_type FROM task_links WHERE id = ?1",
            params![link_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        let now = Utc::now().to_rfc3339();
        let old_value = format!("{}:{}", link.2, link.1);

        self.conn.execute("BEGIN IMMEDIATE", [])?;

        let result = (|| -> Result<()> {
            self.conn
                .execute("DELETE FROM task_links WHERE id = ?1", params![link_id])?;
            self.insert_timeline_event(&link.0, "link_removed", Some(&old_value), "", None, &now)?;
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

        Ok(())
    }

    pub fn get_links(&self, task_id: &str) -> Result<Vec<(String, LinkType, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT tl.id, tl.link_type, tl.target_id, t.title, 'source' AS direction
             FROM task_links tl
             JOIN tasks t ON t.id = tl.target_id
             WHERE tl.source_id = ?1
             UNION ALL
             SELECT tl.id, tl.link_type, tl.source_id, t.title, 'target' AS direction
             FROM task_links tl
             JOIN tasks t ON t.id = tl.source_id
             WHERE tl.target_id = ?1",
        )?;

        let rows = stmt.query_map(params![task_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        let mut links = Vec::new();
        for row in rows {
            let (link_id, link_type_str, related_id, title, direction) = row?;
            let link_type: LinkType = link_type_str.parse().map_err(|e: String| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::from(e),
                )
            })?;
            let effective_type = if direction == "source" {
                link_type
            } else {
                link_type.inverse()
            };
            links.push((link_id, effective_type, related_id, title));
        }
        Ok(links)
    }
}

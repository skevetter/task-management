use std::sync::Mutex;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ErrorData, tool, tool_router};

use crate::db::Database;
use crate::models::{LinkType, TaskDetail, TaskLink, TaskPriority, TaskStatus};

use super::tools::*;

pub struct TaskMcpServer {
    db: Mutex<Database>,
    #[allow(dead_code)]
    default_namespace: Option<String>,
    default_actor: Option<String>,
}

impl TaskMcpServer {
    pub fn new(
        db: Database,
        default_namespace: Option<String>,
        default_actor: Option<String>,
    ) -> Self {
        Self {
            db: Mutex::new(db),
            default_namespace,
            default_actor,
        }
    }

    fn resolve_actor(&self, params_actor: Option<String>) -> Option<String> {
        params_actor.or_else(|| self.default_actor.clone())
    }

    fn resolve_namespace<'a>(&'a self, params_ns: &'a Option<String>) -> Option<&'a str> {
        Some(
            params_ns
                .as_deref()
                .or(self.default_namespace.as_deref())
                .unwrap_or("default"),
        )
    }

    fn resolve_id(&self, prefix: &str, namespace: Option<&str>) -> Result<String, ErrorData> {
        self.db
            .lock()
            .unwrap()
            .resolve_short_id(prefix, namespace)
            .map_err(|e| ErrorData::invalid_params(e, None))
    }
}

#[tool_router(server_handler)]
impl TaskMcpServer {
    #[allow(dead_code)]
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder().enable_tools().build();
        ServerInfo::new(capabilities).with_server_info(Implementation::new(
            "task-management",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    #[tool(description = "Create a new task")]
    fn create_task(
        &self,
        Parameters(params): Parameters<CreateTaskParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let actor = self.resolve_actor(params.actor);
        let priority = params
            .priority
            .as_deref()
            .map(|p| p.parse::<TaskPriority>())
            .transpose()
            .map_err(|e| ErrorData::invalid_params(e, None))?
            .unwrap_or(TaskPriority::Medium);

        let db = self.db.lock().unwrap();
        let task = db
            .create_task(
                &params.title,
                params.description.as_deref(),
                priority,
                params.assignee.as_deref(),
                &params.tags.unwrap_or_default(),
                params.parent.as_deref(),
                actor.as_deref(),
                ns.unwrap_or("default"),
            )
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&task).unwrap(),
        )]))
    }

    #[tool(description = "Update an existing task")]
    fn update_task(
        &self,
        Parameters(params): Parameters<UpdateTaskParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let id = self.resolve_id(&params.id, ns)?;
        let actor = self.resolve_actor(params.actor);

        let status = params
            .status
            .as_deref()
            .map(|s| s.parse::<TaskStatus>())
            .transpose()
            .map_err(|e| ErrorData::invalid_params(e, None))?;

        let priority = params
            .priority
            .as_deref()
            .map(|p| p.parse::<TaskPriority>())
            .transpose()
            .map_err(|e| ErrorData::invalid_params(e, None))?;

        let tags = params.tags;
        let tags_slice = tags.as_deref();

        let db = self.db.lock().unwrap();
        let task = db
            .update_task(
                &id,
                params.title.as_deref(),
                params.description.as_deref(),
                status,
                priority,
                params.assignee.as_deref(),
                tags_slice,
                actor.as_deref(),
            )
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| ErrorData::invalid_params(format!("Task not found: {id}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&task).unwrap(),
        )]))
    }

    #[tool(description = "Close a task")]
    fn close_task(
        &self,
        Parameters(params): Parameters<CloseTaskParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let id = self.resolve_id(&params.id, ns)?;
        let actor = self.resolve_actor(params.actor);

        let db = self.db.lock().unwrap();
        let task = db
            .close_task(&id, actor.as_deref())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| ErrorData::invalid_params(format!("Task not found: {id}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&task).unwrap(),
        )]))
    }

    #[tool(description = "List tasks with optional filters")]
    fn list_tasks(
        &self,
        Parameters(params): Parameters<ListTasksParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let status = params
            .status
            .as_deref()
            .map(|s| s.parse::<TaskStatus>())
            .transpose()
            .map_err(|e| ErrorData::invalid_params(e, None))?;

        let priority = params
            .priority
            .as_deref()
            .map(|p| p.parse::<TaskPriority>())
            .transpose()
            .map_err(|e| ErrorData::invalid_params(e, None))?;

        let limit = params.limit.unwrap_or(50);
        let offset = params.offset.unwrap_or(0);

        let db = self.db.lock().unwrap();
        let result = db
            .list_tasks(
                status,
                params.assignee.as_deref(),
                priority,
                params.tag.as_deref(),
                params.parent.as_deref(),
                params.blocked_by.as_deref(),
                params.blocks.as_deref(),
                ns,
                limit,
                offset,
            )
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&result).unwrap(),
        )]))
    }

    #[tool(description = "Show detailed information about a task")]
    fn show_task(
        &self,
        Parameters(params): Parameters<ShowTaskParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let id = self.resolve_id(&params.id, ns)?;

        let db = self.db.lock().unwrap();
        let task = db
            .get_task(&id)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| ErrorData::invalid_params(format!("Task not found: {id}"), None))?;

        let notes = db
            .get_notes(&id)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let timeline = db
            .get_timeline(&id)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let links_raw = db
            .get_links(&id)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let links: Vec<TaskLink> = links_raw
            .into_iter()
            .map(|(lid, lt, rid, title)| TaskLink {
                link_id: lid,
                relationship: lt.to_string(),
                related_task_id: rid,
                related_task_title: title,
            })
            .collect();

        let detail = TaskDetail {
            task,
            notes,
            timeline,
            links,
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&detail).unwrap(),
        )]))
    }

    #[tool(description = "Add a note to a task")]
    fn add_note(
        &self,
        Parameters(params): Parameters<AddNoteParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let id = self.resolve_id(&params.id, ns)?;

        let db = self.db.lock().unwrap();
        let note = db
            .add_note(&id, &params.message, params.author.as_deref())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&note).unwrap(),
        )]))
    }

    #[tool(description = "Get the timeline history of a task")]
    fn task_history(
        &self,
        Parameters(params): Parameters<TaskHistoryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let id = self.resolve_id(&params.id, ns)?;

        let db = self.db.lock().unwrap();
        db.get_task(&id)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| ErrorData::invalid_params(format!("Task not found: {id}"), None))?;

        let events = db
            .get_timeline(&id)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&events).unwrap(),
        )]))
    }

    #[tool(description = "Create a link between two tasks")]
    fn link_tasks(
        &self,
        Parameters(params): Parameters<LinkTasksParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let source_id = self.resolve_id(&params.source_id, ns)?;
        let target_id = self.resolve_id(&params.target_id, ns)?;
        let link_type: LinkType = params
            .relationship
            .parse()
            .map_err(|e: String| ErrorData::invalid_params(e, None))?;

        let db = self.db.lock().unwrap();
        let link_id = db
            .create_link(&source_id, &target_id, &link_type)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let target_title = db
            .get_task(&target_id)
            .ok()
            .flatten()
            .map(|t| t.title)
            .unwrap_or_default();

        let link = TaskLink {
            link_id,
            relationship: link_type.to_string(),
            related_task_id: target_id,
            related_task_title: target_title,
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&link).unwrap(),
        )]))
    }

    #[tool(description = "List all links for a task")]
    fn list_links(
        &self,
        Parameters(params): Parameters<ListLinksParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let id = self.resolve_id(&params.id, ns)?;

        let db = self.db.lock().unwrap();
        let links_raw = db
            .get_links(&id)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let links: Vec<TaskLink> = links_raw
            .into_iter()
            .map(|(lid, lt, rid, title)| TaskLink {
                link_id: lid,
                relationship: lt.to_string(),
                related_task_id: rid,
                related_task_title: title,
            })
            .collect();

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&links).unwrap(),
        )]))
    }

    #[tool(description = "Close multiple tasks at once by IDs or status filter")]
    fn bulk_close_tasks(
        &self,
        Parameters(params): Parameters<BulkCloseTasksParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let actor = self.resolve_actor(params.actor);

        let task_ids: Vec<String> = if let Some(ids) = params.ids {
            ids.iter()
                .map(|id| self.resolve_id(id, ns))
                .collect::<Result<Vec<_>, _>>()?
        } else if let Some(filter) = &params.status_filter {
            let status: TaskStatus = filter
                .parse()
                .map_err(|e: String| ErrorData::invalid_params(e, None))?;
            let db = self.db.lock().unwrap();
            let result = db
                .list_tasks(
                    Some(status),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    ns,
                    10000,
                    0,
                )
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            drop(db);
            result.tasks.into_iter().map(|t| t.id).collect()
        } else {
            return Err(ErrorData::invalid_params(
                "Must provide ids or status_filter",
                None,
            ));
        };

        let db = self.db.lock().unwrap();
        let closed = db
            .bulk_close_tasks(&task_ids, actor.as_deref(), params.reason.as_deref())
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let closed_ids: Vec<&str> = closed.iter().map(|t| t.id.as_str()).collect();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({"closed": closed_ids}).to_string(),
        )]))
    }

    #[tool(description = "Remove a link between two tasks")]
    fn unlink_tasks(
        &self,
        Parameters(params): Parameters<UnlinkTasksParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = self.db.lock().unwrap();
        let link_id = db
            .resolve_short_link_id(&params.link_id)
            .map_err(|e| ErrorData::invalid_params(e, None))?;
        db.remove_link(&link_id)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({"status": "ok", "link_id": link_id}).to_string(),
        )]))
    }

    #[tool(description = "Create a task from a named template")]
    fn create_from_template(
        &self,
        Parameters(params): Parameters<CreateFromTemplateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let ns = self.resolve_namespace(&params.namespace);
        let actor = self.resolve_actor(params.actor);

        let db = self.db.lock().unwrap();
        let task = db
            .create_task_from_template(
                &params.template,
                &params.title,
                ns.unwrap_or("default"),
                actor.as_deref(),
            )
            .map_err(|e| ErrorData::invalid_params(e, None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&task).unwrap(),
        )]))
    }

    #[tool(description = "List all available task templates")]
    fn list_templates(
        &self,
        #[allow(unused_variables)] Parameters(params): Parameters<ListTemplatesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = self.db.lock().unwrap();
        let templates = db
            .list_templates()
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&templates).unwrap(),
        )]))
    }

    #[tool(description = "Show details of a specific task template")]
    fn show_template(
        &self,
        Parameters(params): Parameters<ShowTemplateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let db = self.db.lock().unwrap();
        let template = db
            .get_template(&params.name)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            .ok_or_else(|| {
                ErrorData::invalid_params(format!("Template not found: {}", params.name), None)
            })?;

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string(&template).unwrap(),
        )]))
    }
}

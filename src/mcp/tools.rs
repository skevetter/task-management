#![allow(dead_code)]

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTaskParams {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub tags: Option<Vec<String>>,
    pub parent: Option<String>,
    pub namespace: Option<String>,
    pub actor: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateTaskParams {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assignee: Option<String>,
    pub tags: Option<Vec<String>>,
    pub namespace: Option<String>,
    pub actor: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CloseTaskParams {
    pub id: String,
    pub namespace: Option<String>,
    pub actor: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListTasksParams {
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub tag: Option<String>,
    pub parent: Option<String>,
    pub blocked_by: Option<String>,
    pub blocks: Option<String>,
    pub namespace: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShowTaskParams {
    pub id: String,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddNoteParams {
    pub id: String,
    pub message: String,
    pub author: Option<String>,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskHistoryParams {
    pub id: String,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LinkTasksParams {
    pub source_id: String,
    pub relationship: String,
    pub target_id: String,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UnlinkTasksParams {
    pub link_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BulkCloseTasksParams {
    pub ids: Option<Vec<String>>,
    pub status_filter: Option<String>,
    pub namespace: Option<String>,
    pub actor: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListLinksParams {
    pub id: String,
    pub namespace: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateFromTemplateParams {
    pub template: String,
    pub title: String,
    pub namespace: Option<String>,
    pub actor: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListTemplatesParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShowTemplateParams {
    pub name: String,
}

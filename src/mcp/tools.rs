// T3 will populate fields on each struct.

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTaskParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateTaskParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CloseTaskParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListTasksParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShowTaskParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddNoteParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TaskHistoryParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LinkTasksParams {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListLinksParams {}

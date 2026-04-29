use std::fmt;
use std::str::FromStr;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    Parent,
    Child,
    BlockedBy,
    Blocks,
    RelatedTo,
}

impl fmt::Display for LinkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parent => write!(f, "parent"),
            Self::Child => write!(f, "child"),
            Self::BlockedBy => write!(f, "blocked_by"),
            Self::Blocks => write!(f, "blocks"),
            Self::RelatedTo => write!(f, "related_to"),
        }
    }
}

impl FromStr for LinkType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "parent" => Ok(Self::Parent),
            "child" => Ok(Self::Child),
            "blocked_by" | "blockedby" => Ok(Self::BlockedBy),
            "blocks" => Ok(Self::Blocks),
            "related_to" | "relatedto" => Ok(Self::RelatedTo),
            _ => Err(format!("unknown link type: {s}")),
        }
    }
}

impl LinkType {
    pub fn inverse(&self) -> LinkType {
        match self {
            Self::Parent => Self::Child,
            Self::Child => Self::Parent,
            Self::BlockedBy => Self::Blocks,
            Self::Blocks => Self::BlockedBy,
            Self::RelatedTo => Self::RelatedTo,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Open,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Blocked => write!(f, "blocked"),
            Self::Done => write!(f, "done"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "in-progress" | "inprogress" | "in_progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "done" => Ok(Self::Done),
            "cancelled" | "closed" => Ok(Self::Cancelled),
            _ => Err(format!("unknown status: {s}")),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl FromStr for TaskPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("unknown priority: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assignee: Option<String>,
    pub tags: Vec<String>,
    pub parent_task_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub namespace: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    pub tasks: Vec<Task>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNote {
    pub id: String,
    pub task_id: String,
    pub body: String,
    pub author: Option<String>,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    pub task_id: String,
    pub event_type: String,
    pub old_value: Option<String>,
    pub new_value: String,
    pub actor: Option<String>,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetail {
    #[serde(flatten)]
    pub task: Task,
    pub notes: Vec<TaskNote>,
    pub timeline: Vec<TimelineEvent>,
    pub links: Vec<TaskLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLink {
    pub link_id: String,
    pub relationship: String,
    pub related_task_id: String,
    pub related_task_title: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub id: String,
    pub name: String,
    pub title_pattern: String,
    pub default_priority: Option<String>,
    pub default_status: Option<String>,
    pub default_tags: Option<Vec<String>>,
    pub builtin: bool,
    pub created_at: String,
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ID:          {}", self.id)?;
        writeln!(f, "Title:       {}", self.title)?;
        if let Some(desc) = &self.description {
            writeln!(f, "Description: {desc}")?;
        }
        writeln!(f, "Status:      {}", self.status)?;
        writeln!(f, "Priority:    {}", self.priority)?;
        if let Some(assignee) = &self.assignee {
            writeln!(f, "Assignee:    {assignee}")?;
        }
        if !self.tags.is_empty() {
            writeln!(f, "Tags:        {}", self.tags.join(", "))?;
        }
        if let Some(parent) = &self.parent_task_id {
            writeln!(f, "Parent:      {parent}")?;
        }
        writeln!(f, "Namespace:   {}", self.namespace)?;
        writeln!(f, "Created:     {}", self.created_at)?;
        write!(f, "Updated:     {}", self.updated_at)
    }
}

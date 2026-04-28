use std::fmt;
use std::str::FromStr;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Open,
    InProgress,
    Blocked,
    Done,
    Closed,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Blocked => write!(f, "blocked"),
            Self::Done => write!(f, "done"),
            Self::Closed => write!(f, "closed"),
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
            "closed" => Ok(Self::Closed),
            _ => Err(format!("unknown status: {s}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
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
        writeln!(f, "Created:     {}", self.created_at)?;
        write!(f, "Updated:     {}", self.updated_at)
    }
}

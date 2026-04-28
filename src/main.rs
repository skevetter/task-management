mod db;
mod models;

use clap::{Parser, Subcommand};

use db::Database;
use models::{TaskPriority, TaskStatus};

const DEFAULT_DB_PATH: &str = "tasks.db";

#[derive(Parser)]
#[command(name = "task-management", about = "A task management CLI tool")]
struct Cli {
    #[arg(long, default_value = DEFAULT_DB_PATH, global = true)]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, value_enum, default_value_t = TaskPriority::Medium)]
        priority: TaskPriority,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long = "parent")]
        parent: Option<String>,
    },
    Show {
        id: String,
    },
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, value_enum)]
        status: Option<TaskStatus>,
        #[arg(long, value_enum)]
        priority: Option<TaskPriority>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
    },
    Close {
        id: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let db = Database::open(&cli.db).unwrap_or_else(|e| {
        eprintln!("Failed to open database: {e}");
        std::process::exit(1);
    });

    match cli.command {
        Commands::Create {
            title,
            description,
            priority,
            assignee,
            tags,
            parent,
        } => {
            let task = db
                .create_task(
                    &title,
                    description.as_deref(),
                    priority,
                    assignee.as_deref(),
                    &tags,
                    parent.as_deref(),
                )
                .unwrap_or_else(|e| {
                    eprintln!("Failed to create task: {e}");
                    std::process::exit(1);
                });
            println!("{task}");
        }
        Commands::Show { id } => {
            let task = db.get_task(&id).unwrap_or_else(|e| {
                eprintln!("Failed to get task: {e}");
                std::process::exit(1);
            });
            match task {
                Some(t) => println!("{t}"),
                None => {
                    eprintln!("Task not found: {id}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Update {
            id,
            title,
            description,
            status,
            priority,
            assignee,
            tags,
        } => {
            let tags_opt = if tags.is_empty() {
                None
            } else {
                Some(tags.as_slice())
            };
            let task = db
                .update_task(
                    &id,
                    title.as_deref(),
                    description.as_deref(),
                    status,
                    priority,
                    assignee.as_deref(),
                    tags_opt,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Failed to update task: {e}");
                    std::process::exit(1);
                });
            match task {
                Some(t) => println!("{t}"),
                None => {
                    eprintln!("Task not found: {id}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Close { id } => {
            let task = db.close_task(&id).unwrap_or_else(|e| {
                eprintln!("Failed to close task: {e}");
                std::process::exit(1);
            });
            match task {
                Some(t) => println!("{t}"),
                None => {
                    eprintln!("Task not found: {id}");
                    std::process::exit(1);
                }
            }
        }
    }
}

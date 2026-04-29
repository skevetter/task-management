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
    List {
        #[arg(long, value_enum)]
        status: Option<TaskStatus>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long, value_enum)]
        priority: Option<TaskPriority>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        parent: Option<String>,
    },
    Note {
        id: String,
        message: String,
        #[arg(long)]
        author: Option<String>,
    },
    History {
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
        Commands::List {
            status,
            assignee,
            priority,
            tag,
            parent,
        } => {
            let tasks = db
                .list_tasks(
                    status,
                    assignee.as_deref(),
                    priority,
                    tag.as_deref(),
                    parent.as_deref(),
                )
                .unwrap_or_else(|e| {
                    eprintln!("Failed to list tasks: {e}");
                    std::process::exit(1);
                });
            if tasks.is_empty() {
                println!("No tasks found.");
            } else {
                let header = format!(
                    "{:<10} {:<30} {:<14} {:<10} {}",
                    "ID", "TITLE", "STATUS", "PRIORITY", "ASSIGNEE"
                );
                println!("{header}");
                println!("{}", "-".repeat(76));
                for task in &tasks {
                    let short_id = if task.id.len() > 8 {
                        &task.id[..8]
                    } else {
                        &task.id
                    };
                    let title = if task.title.len() > 28 {
                        format!("{}...", &task.title[..25])
                    } else {
                        task.title.clone()
                    };
                    let assignee_str = task.assignee.as_deref().unwrap_or("-");
                    println!(
                        "{:<10} {:<30} {:<14} {:<10} {}",
                        short_id, title, task.status, task.priority, assignee_str
                    );
                }
                println!("\n{} task(s) found.", tasks.len());
            }
        }
        Commands::Note {
            id,
            message,
            author,
        } => {
            let note = db.add_note(&id, &message, author.as_deref());
            match note {
                Ok(n) => {
                    println!("Note ID:    {}", n.id);
                    println!("Task:       {}", n.task_id);
                    println!("Author:     {}", n.author.as_deref().unwrap_or("(none)"));
                    println!("Body:       {}", n.body);
                    println!("Created:    {}", n.created_at);
                }
                Err(_) => {
                    eprintln!("Task not found: {id}");
                    std::process::exit(1);
                }
            }
        }
        Commands::History { id } => {
            let task = db.get_task(&id).unwrap_or_else(|e| {
                eprintln!("Failed to get task: {e}");
                std::process::exit(1);
            });
            if task.is_none() {
                eprintln!("Task not found: {id}");
                std::process::exit(1);
            }

            let events = db.get_timeline(&id).unwrap_or_else(|e| {
                eprintln!("Failed to get timeline: {e}");
                std::process::exit(1);
            });

            let separator = "\u{2500}".repeat(54);
            println!("History for task {id}");
            println!("{separator}");
            if events.is_empty() {
                println!("(no events)");
            } else {
                for event in &events {
                    let description = match event.event_type.as_str() {
                        "created" => event.new_value.clone(),
                        "status_changed" | "priority_changed" => {
                            format!(
                                "{} \u{2192} {}",
                                event.old_value.as_deref().unwrap_or(""),
                                &event.new_value
                            )
                        }
                        "assignee_changed" => {
                            let old = event
                                .old_value
                                .as_deref()
                                .filter(|s| !s.is_empty())
                                .unwrap_or("(none)");
                            let new = if event.new_value.is_empty() {
                                "(none)"
                            } else {
                                &event.new_value
                            };
                            format!("{old} \u{2192} {new}")
                        }
                        "note_added" => match &event.actor {
                            Some(actor) if !actor.is_empty() => {
                                format!("{} (by {actor})", event.new_value)
                            }
                            _ => event.new_value.clone(),
                        },
                        _ => event.new_value.clone(),
                    };
                    println!(
                        "{:<20}  {:<18}  {}",
                        event.occurred_at,
                        format!("[{}]", event.event_type),
                        description
                    );
                }
            }
            println!("{separator}");
            if events.is_empty() {
                // footer already printed separator
            } else {
                println!("{} event(s)", events.len());
            }
        }
    }
}

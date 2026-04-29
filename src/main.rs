mod db;
mod mcp;
mod models;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use db::Database;
use models::{LinkType, TaskDetail, TaskLink, TaskPriority, TaskStatus};

fn default_db_path() -> PathBuf {
    let base = match std::env::var("XDG_DATA_HOME") {
        Ok(val) if !val.is_empty() => {
            let path = PathBuf::from(&val);
            if path.is_relative() {
                eprintln!("XDG_DATA_HOME is a relative path; resolving against cwd");
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(path)
            } else {
                path
            }
        }
        _ => match std::env::var("HOME") {
            Ok(home) if !home.is_empty() => PathBuf::from(home).join(".local").join("share"),
            _ => {
                eprintln!(
                    "Neither XDG_DATA_HOME nor HOME is set; using ./task-management/tasks.db"
                );
                PathBuf::from(".")
            }
        },
    };
    base.join("task-management").join("tasks.db")
}

#[derive(Parser)]
#[command(name = "task-management", about = "A task management CLI tool")]
struct Cli {
    #[arg(long, global = true)]
    db: Option<String>,

    #[arg(long, global = true)]
    json: bool,

    #[arg(long, short = 'n', global = true)]
    namespace: Option<String>,

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
        #[arg(long)]
        blocked_by: Option<String>,
        #[arg(long)]
        blocks: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long, default_value_t = 0)]
        offset: i64,
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
    Link {
        #[command(subcommand)]
        command: LinkCommands,
    },
    Serve {
        #[arg(long, default_value = "stdio")]
        transport: String,
        #[arg(long)]
        namespace: Option<String>,
    },
}

#[derive(Subcommand)]
enum LinkCommands {
    Add {
        task_id: String,
        #[arg(value_enum)]
        relationship: LinkType,
        target_id: String,
    },
    Remove {
        link_id: String,
    },
    List {
        task_id: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let db_path = match cli.db {
        Some(p) => PathBuf::from(p),
        None => default_db_path(),
    };
    if let Some(parent) = db_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("Failed to create database directory: {e}");
            std::process::exit(1);
        });
    }
    let db_str = db_path.to_string_lossy();
    let db = Database::open(&db_str).unwrap_or_else(|e| {
        eprintln!("Failed to open database: {e}");
        std::process::exit(1);
    });

    let json = cli.json;
    let namespace = cli.namespace.as_deref();

    let resolve = |prefix: &str| -> String {
        db.resolve_short_id(prefix, namespace).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        })
    };

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
                    None,
                    namespace.unwrap_or("default"),
                )
                .unwrap_or_else(|e| {
                    eprintln!("Failed to create task: {e}");
                    std::process::exit(1);
                });
            if json {
                println!("{}", serde_json::to_string(&task).unwrap());
            } else {
                println!("{task}");
            }
        }
        Commands::Show { id } => {
            let id = resolve(&id);
            let task = db.get_task(&id).unwrap_or_else(|e| {
                eprintln!("Failed to get task: {e}");
                std::process::exit(1);
            });
            match task {
                Some(t) => {
                    let raw_links = db.get_links(&t.id).unwrap_or_else(|e| {
                        eprintln!("Failed to get links: {e}");
                        std::process::exit(1);
                    });
                    if json {
                        let notes = db.get_notes(&t.id).unwrap_or_else(|e| {
                            eprintln!("Failed to get notes: {e}");
                            std::process::exit(1);
                        });
                        let timeline = db.get_timeline(&t.id).unwrap_or_else(|e| {
                            eprintln!("Failed to get timeline: {e}");
                            std::process::exit(1);
                        });
                        let links: Vec<TaskLink> = raw_links
                            .into_iter()
                            .map(|(lid, lt, rid, title)| TaskLink {
                                link_id: lid,
                                relationship: lt.to_string(),
                                related_task_id: rid,
                                related_task_title: title,
                            })
                            .collect();
                        let detail = TaskDetail {
                            task: t,
                            notes,
                            timeline,
                            links,
                        };
                        println!("{}", serde_json::to_string(&detail).unwrap());
                    } else {
                        println!("{t}");
                        if !raw_links.is_empty() {
                            println!("Links:");
                            for (_, link_type, related_id, title) in &raw_links {
                                let short_id = if related_id.len() > 8 {
                                    &related_id[..8]
                                } else {
                                    related_id
                                };
                                println!("  {link_type}  {short_id}  ({title})");
                            }
                        }
                    }
                }
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
            let id = resolve(&id);
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
                Some(t) => {
                    if json {
                        println!("{}", serde_json::to_string(&t).unwrap());
                    } else {
                        println!("{t}");
                    }
                }
                None => {
                    eprintln!("Task not found: {id}");
                    std::process::exit(1);
                }
            }
        }
        Commands::Close { id } => {
            let id = resolve(&id);
            let task = db.close_task(&id).unwrap_or_else(|e| {
                eprintln!("Failed to close task: {e}");
                std::process::exit(1);
            });
            match task {
                Some(t) => {
                    if json {
                        println!("{}", serde_json::to_string(&t).unwrap());
                    } else {
                        println!("{t}");
                    }
                }
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
            blocked_by,
            blocks,
            limit,
            offset,
        } => {
            let result = db
                .list_tasks(
                    status,
                    assignee.as_deref(),
                    priority,
                    tag.as_deref(),
                    parent.as_deref(),
                    blocked_by.as_deref(),
                    blocks.as_deref(),
                    namespace,
                    limit,
                    offset,
                )
                .unwrap_or_else(|e| {
                    eprintln!("Failed to list tasks: {e}");
                    std::process::exit(1);
                });
            if json {
                println!("{}", serde_json::to_string(&result).unwrap());
            } else if result.tasks.is_empty() {
                println!("No tasks found.");
            } else {
                let header = format!(
                    "{:<10} {:<30} {:<14} {:<10} {}",
                    "ID", "TITLE", "STATUS", "PRIORITY", "ASSIGNEE"
                );
                println!("{header}");
                println!("{}", "-".repeat(76));
                for task in &result.tasks {
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
                let start = offset + 1;
                let end = offset + result.tasks.len() as i64;
                println!("\nShowing {start}-{end} of {} task(s)", result.total);
            }
        }
        Commands::Note {
            id,
            message,
            author,
        } => {
            let id = resolve(&id);
            let note = db.add_note(&id, &message, author.as_deref());
            match note {
                Ok(n) => {
                    if json {
                        println!("{}", serde_json::to_string(&n).unwrap());
                    } else {
                        println!("Note ID:    {}", n.id);
                        println!("Task:       {}", n.task_id);
                        println!("Author:     {}", n.author.as_deref().unwrap_or("(none)"));
                        println!("Body:       {}", n.body);
                        println!("Created:    {}", n.created_at);
                    }
                }
                Err(_) => {
                    eprintln!("Task not found: {id}");
                    std::process::exit(1);
                }
            }
        }
        Commands::History { id } => {
            let id = resolve(&id);
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

            if json {
                println!("{}", serde_json::to_string(&events).unwrap());
            } else {
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
                if !events.is_empty() {
                    println!("{} event(s)", events.len());
                }
            }
        }
        Commands::Serve {
            transport,
            namespace,
        } => {
            if transport != "stdio" {
                eprintln!("Only stdio transport is supported");
                std::process::exit(1);
            }
            let server = mcp::server::TaskMcpServer::new(db, namespace, None);
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                use rmcp::ServiceExt;
                let transport = rmcp::transport::io::stdio();
                let service = server.serve(transport).await.unwrap();
                service.waiting().await.unwrap();
            });
        }
        Commands::Link { command } => match command {
            LinkCommands::Add {
                task_id,
                relationship,
                target_id,
            } => {
                let task_id = resolve(&task_id);
                let target_id = resolve(&target_id);
                let link_id = db
                    .create_link(&task_id, &target_id, &relationship)
                    .unwrap_or_else(|e| {
                        eprintln!("Failed to create link: {e}");
                        std::process::exit(1);
                    });
                if json {
                    let target_title = db
                        .get_task(&target_id)
                        .ok()
                        .flatten()
                        .map(|t| t.title)
                        .unwrap_or_default();
                    let link = TaskLink {
                        link_id: link_id.clone(),
                        relationship: relationship.to_string(),
                        related_task_id: target_id,
                        related_task_title: target_title,
                    };
                    println!("{}", serde_json::to_string(&link).unwrap());
                } else {
                    let short_id = if link_id.len() > 8 {
                        &link_id[..8]
                    } else {
                        &link_id
                    };
                    println!("Link created: {short_id} ({task_id} {relationship} {target_id})");
                }
            }
            LinkCommands::Remove { link_id } => {
                db.remove_link(&link_id).unwrap_or_else(|e| {
                    eprintln!("Failed to remove link: {e}");
                    std::process::exit(1);
                });
                if json {
                    println!(
                        "{}",
                        serde_json::to_string(&serde_json::json!({"removed": link_id})).unwrap()
                    );
                } else {
                    let short_id = if link_id.len() > 8 {
                        &link_id[..8]
                    } else {
                        &link_id
                    };
                    println!("Link {short_id} removed.");
                }
            }
            LinkCommands::List { task_id } => {
                let task_id = resolve(&task_id);
                let raw_links = db.get_links(&task_id).unwrap_or_else(|e| {
                    eprintln!("Failed to get links: {e}");
                    std::process::exit(1);
                });
                if json {
                    let links: Vec<TaskLink> = raw_links
                        .into_iter()
                        .map(|(lid, lt, rid, title)| TaskLink {
                            link_id: lid,
                            relationship: lt.to_string(),
                            related_task_id: rid,
                            related_task_title: title,
                        })
                        .collect();
                    println!("{}", serde_json::to_string(&links).unwrap());
                } else if raw_links.is_empty() {
                    println!("No links found for task {task_id}.");
                } else {
                    println!("{:<10} {:<14} RELATED TASK", "LINK ID", "RELATIONSHIP");
                    let sep = format!(
                        "{:<10} {:<14} {}",
                        "\u{2500}".repeat(8),
                        "\u{2500}".repeat(12),
                        "\u{2500}".repeat(33)
                    );
                    println!("{sep}");
                    for (link_id, link_type, related_id, title) in &raw_links {
                        let short_link = if link_id.len() > 8 {
                            &link_id[..8]
                        } else {
                            link_id
                        };
                        let short_task = if related_id.len() > 8 {
                            &related_id[..8]
                        } else {
                            related_id
                        };
                        println!(
                            "{:<10} {:<14} {}  ({})",
                            short_link, link_type, short_task, title
                        );
                    }
                    println!("\n{} link(s).", raw_links.len());
                }
            }
        },
    }
}

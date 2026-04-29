use crate::db::Database;

#[expect(dead_code)] // fields used in T3 tool implementations
pub struct TaskMcpServer {
    db: Database,
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
            db,
            default_namespace,
            default_actor,
        }
    }
}

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::Value;
use tempfile::NamedTempFile;

struct McpTestClient {
    child: std::process::Child,
    stdin: std::io::BufWriter<std::process::ChildStdin>,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: i64,
}

impl McpTestClient {
    fn new(db_path: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_task-management"))
            .args(["--db", db_path, "serve"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start MCP server");

        let stdin = std::io::BufWriter::new(child.stdin.take().unwrap());
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut client = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        };
        client.initialize();
        client
    }

    fn initialize(&mut self) {
        let resp = self.send_request(
            "initialize",
            serde_json::json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0"}
            }),
        );
        assert!(
            resp.get("result").is_some(),
            "initialize should succeed: {resp}"
        );

        let msg = serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        writeln!(self.stdin, "{}", msg).unwrap();
        self.stdin.flush().unwrap();
    }

    fn call_tool(&mut self, name: &str, args: Value) -> Value {
        self.send_request(
            "tools/call",
            serde_json::json!({
                "name": name,
                "arguments": args
            }),
        )
    }

    fn send_request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        writeln!(self.stdin, "{}", msg).unwrap();
        self.stdin.flush().unwrap();
        self.read_response()
    }

    fn read_response(&mut self) -> Value {
        loop {
            let mut line = String::new();
            let n = self
                .stdout
                .read_line(&mut line)
                .expect("Failed to read from MCP server stdout");
            assert!(n > 0, "MCP server closed stdout unexpectedly");
            let msg: Value = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("Failed to parse JSON-RPC response: {e}\nRaw: {line}"));
            if msg.get("id").is_some() {
                return msg;
            }
        }
    }
}

impl Drop for McpTestClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn extract_content(response: &Value) -> Value {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("Expected text content in response: {response}"));
    serde_json::from_str(text)
        .unwrap_or_else(|e| panic!("Expected valid JSON in content text: {e}\nRaw: {text}"))
}

#[test]
fn test_create_task() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let resp = client.call_tool("create_task", serde_json::json!({"title": "Test task"}));
    let task = extract_content(&resp);

    assert!(task["id"].is_string());
    assert_eq!(task["title"].as_str().unwrap(), "Test task");
    assert_eq!(task["status"].as_str().unwrap(), "open");
    assert_eq!(task["priority"].as_str().unwrap(), "medium");
}

#[test]
fn test_list_tasks() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    client.call_tool("create_task", serde_json::json!({"title": "Task A"}));
    client.call_tool("create_task", serde_json::json!({"title": "Task B"}));

    let resp = client.call_tool("list_tasks", serde_json::json!({}));
    let tasks = extract_content(&resp);

    assert!(tasks.is_object());
    let tasks_arr = tasks["tasks"].as_array().unwrap();
    assert_eq!(tasks_arr.len(), 2);
    assert_eq!(tasks["total"].as_i64().unwrap(), 2);
    assert_eq!(tasks["limit"].as_i64().unwrap(), 50);
    assert_eq!(tasks["offset"].as_i64().unwrap(), 0);
}

#[test]
fn test_show_task() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let create_resp =
        client.call_tool("create_task", serde_json::json!({"title": "Detailed task"}));
    let task = extract_content(&create_resp);
    let task_id = task["id"].as_str().unwrap();

    let resp = client.call_tool("show_task", serde_json::json!({"id": task_id}));
    let detail = extract_content(&resp);

    assert_eq!(detail["title"].as_str().unwrap(), "Detailed task");
    assert!(detail["notes"].is_array());
    assert!(detail["timeline"].is_array());
    assert!(detail["links"].is_array());
}

#[test]
fn test_update_task() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let create_resp = client.call_tool("create_task", serde_json::json!({"title": "Update me"}));
    let task = extract_content(&create_resp);
    let task_id = task["id"].as_str().unwrap();

    let resp = client.call_tool(
        "update_task",
        serde_json::json!({"id": task_id, "status": "in-progress"}),
    );
    let updated = extract_content(&resp);

    assert_eq!(updated["status"].as_str().unwrap(), "in-progress");
    assert_eq!(updated["id"].as_str().unwrap(), task_id);
}

#[test]
fn test_close_task() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let create_resp = client.call_tool("create_task", serde_json::json!({"title": "Close me"}));
    let task = extract_content(&create_resp);
    let task_id = task["id"].as_str().unwrap();

    let resp = client.call_tool("close_task", serde_json::json!({"id": task_id}));
    let closed = extract_content(&resp);

    assert_eq!(closed["status"].as_str().unwrap(), "cancelled");
    assert_eq!(closed["id"].as_str().unwrap(), task_id);
}

#[test]
fn test_add_note() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let create_resp = client.call_tool("create_task", serde_json::json!({"title": "Note task"}));
    let task = extract_content(&create_resp);
    let task_id = task["id"].as_str().unwrap().to_string();

    let resp = client.call_tool(
        "add_note",
        serde_json::json!({"id": task_id, "message": "Test note content", "author": "tester"}),
    );
    let note = extract_content(&resp);

    assert!(note["id"].is_string());
    assert_eq!(note["task_id"].as_str().unwrap(), task_id);
    assert_eq!(note["body"].as_str().unwrap(), "Test note content");
    assert_eq!(note["author"].as_str().unwrap(), "tester");
}

#[test]
fn test_task_history() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let create_resp = client.call_tool("create_task", serde_json::json!({"title": "History task"}));
    let task = extract_content(&create_resp);
    let task_id = task["id"].as_str().unwrap();

    let resp = client.call_tool("task_history", serde_json::json!({"id": task_id}));
    let events = extract_content(&resp);

    assert!(events.is_array());
    let events_arr = events.as_array().unwrap();
    assert!(!events_arr.is_empty());
    assert_eq!(events_arr[0]["event_type"].as_str().unwrap(), "created");
}

#[test]
fn test_link_tasks() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let resp_a = client.call_tool("create_task", serde_json::json!({"title": "Source task"}));
    let task_a = extract_content(&resp_a);
    let id_a = task_a["id"].as_str().unwrap().to_string();

    let resp_b = client.call_tool("create_task", serde_json::json!({"title": "Target task"}));
    let task_b = extract_content(&resp_b);
    let id_b = task_b["id"].as_str().unwrap().to_string();

    let resp = client.call_tool(
        "link_tasks",
        serde_json::json!({"source_id": id_a, "relationship": "blocks", "target_id": id_b}),
    );
    let link = extract_content(&resp);

    assert!(link["link_id"].is_string());
    assert_eq!(link["relationship"].as_str().unwrap(), "blocks");
    assert_eq!(link["related_task_id"].as_str().unwrap(), id_b);
    assert_eq!(link["related_task_title"].as_str().unwrap(), "Target task");
}

#[test]
fn test_list_links() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let resp_a = client.call_tool("create_task", serde_json::json!({"title": "Link source"}));
    let task_a = extract_content(&resp_a);
    let id_a = task_a["id"].as_str().unwrap().to_string();

    let resp_b = client.call_tool("create_task", serde_json::json!({"title": "Link target"}));
    let task_b = extract_content(&resp_b);
    let id_b = task_b["id"].as_str().unwrap().to_string();

    client.call_tool(
        "link_tasks",
        serde_json::json!({"source_id": id_a, "relationship": "related_to", "target_id": id_b}),
    );

    let resp = client.call_tool("list_links", serde_json::json!({"id": id_a}));
    let links = extract_content(&resp);

    assert!(links.is_array());
    let links_arr = links.as_array().unwrap();
    assert_eq!(links_arr.len(), 1);
    assert_eq!(links_arr[0]["relationship"].as_str().unwrap(), "related_to");
}

#[test]
fn test_short_id_resolution() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let create_resp =
        client.call_tool("create_task", serde_json::json!({"title": "Short ID test"}));
    let task = extract_content(&create_resp);
    let full_id = task["id"].as_str().unwrap().to_string();
    let short_id = &full_id[..4];

    let resp = client.call_tool("show_task", serde_json::json!({"id": short_id}));
    let detail = extract_content(&resp);

    assert_eq!(detail["id"].as_str().unwrap(), full_id);
    assert_eq!(detail["title"].as_str().unwrap(), "Short ID test");
}

#[test]
fn test_task_not_found() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    client.call_tool("create_task", serde_json::json!({"title": "Dummy"}));

    let resp = client.call_tool(
        "show_task",
        serde_json::json!({"id": "00000000-0000-0000-0000-000000000000"}),
    );

    assert!(
        resp.get("error").is_some(),
        "Expected error for non-existent task: {resp}"
    );
}

#[test]
fn test_list_tasks_with_filter() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let resp_a = client.call_tool("create_task", serde_json::json!({"title": "Open task"}));
    let task_a = extract_content(&resp_a);
    let id_a = task_a["id"].as_str().unwrap().to_string();

    client.call_tool("create_task", serde_json::json!({"title": "Another open"}));

    client.call_tool(
        "update_task",
        serde_json::json!({"id": id_a, "status": "in-progress"}),
    );

    let resp = client.call_tool("list_tasks", serde_json::json!({"status": "open"}));
    let result = extract_content(&resp);
    let tasks_arr = result["tasks"].as_array().unwrap();
    assert_eq!(tasks_arr.len(), 1);
    assert_eq!(tasks_arr[0]["title"].as_str().unwrap(), "Another open");

    let resp2 = client.call_tool("list_tasks", serde_json::json!({"status": "in-progress"}));
    let result2 = extract_content(&resp2);
    let tasks2_arr = result2["tasks"].as_array().unwrap();
    assert_eq!(tasks2_arr.len(), 1);
    assert_eq!(tasks2_arr[0]["title"].as_str().unwrap(), "Open task");
}

// --- Namespace and pagination MCP tests ---

#[test]
fn test_mcp_create_task_with_namespace() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let resp = client.call_tool(
        "create_task",
        serde_json::json!({"title": "NS task", "namespace": "ns-a"}),
    );
    let task = extract_content(&resp);

    assert_eq!(task["title"].as_str().unwrap(), "NS task");
    assert_eq!(task["namespace"].as_str().unwrap(), "ns-a");
}

#[test]
fn test_mcp_list_tasks_with_namespace() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    client.call_tool(
        "create_task",
        serde_json::json!({"title": "A1", "namespace": "ns-a"}),
    );
    client.call_tool(
        "create_task",
        serde_json::json!({"title": "A2", "namespace": "ns-a"}),
    );
    client.call_tool(
        "create_task",
        serde_json::json!({"title": "B1", "namespace": "ns-b"}),
    );

    let resp = client.call_tool("list_tasks", serde_json::json!({"namespace": "ns-a"}));
    let result = extract_content(&resp);

    assert_eq!(result["total"].as_i64().unwrap(), 2);
    let tasks = result["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 2);
    for t in tasks {
        assert_eq!(t["namespace"].as_str().unwrap(), "ns-a");
    }
}

#[test]
fn test_mcp_list_tasks_pagination() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    for i in 0..5 {
        client.call_tool(
            "create_task",
            serde_json::json!({"title": format!("Page {i}")}),
        );
    }

    let resp = client.call_tool("list_tasks", serde_json::json!({"limit": 2, "offset": 0}));
    let result = extract_content(&resp);

    assert_eq!(result["total"].as_i64().unwrap(), 5);
    assert_eq!(result["limit"].as_i64().unwrap(), 2);
    assert_eq!(result["offset"].as_i64().unwrap(), 0);
    assert_eq!(result["tasks"].as_array().unwrap().len(), 2);
}

#[test]
fn test_mcp_list_tasks_no_namespace_returns_only_default() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    client.call_tool(
        "create_task",
        serde_json::json!({"title": "Default task"}),
    );
    client.call_tool(
        "create_task",
        serde_json::json!({"title": "Other task", "namespace": "other"}),
    );

    let resp = client.call_tool("list_tasks", serde_json::json!({}));
    let result = extract_content(&resp);

    assert_eq!(result["total"].as_i64().unwrap(), 1);
    let tasks = result["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["title"].as_str().unwrap(), "Default task");
    assert_eq!(tasks[0]["namespace"].as_str().unwrap(), "default");
}

#[test]
fn test_unlink_tasks() {
    let tmp = NamedTempFile::new().unwrap();
    let mut client = McpTestClient::new(tmp.path().to_str().unwrap());

    let resp_a = client.call_tool("create_task", serde_json::json!({"title": "Unlink A"}));
    let id_a = extract_content(&resp_a)["id"].as_str().unwrap().to_string();

    let resp_b = client.call_tool("create_task", serde_json::json!({"title": "Unlink B"}));
    let id_b = extract_content(&resp_b)["id"].as_str().unwrap().to_string();

    let link_resp = client.call_tool(
        "link_tasks",
        serde_json::json!({"source_id": id_a, "relationship": "blocks", "target_id": id_b}),
    );
    let link_id = extract_content(&link_resp)["link_id"].as_str().unwrap().to_string();

    let unlink_resp = client.call_tool("unlink_tasks", serde_json::json!({"link_id": link_id}));
    let result = extract_content(&unlink_resp);
    assert_eq!(result["status"].as_str().unwrap(), "ok");

    let links_resp = client.call_tool("list_links", serde_json::json!({"id": id_a}));
    let links = extract_content(&links_resp);
    assert_eq!(links.as_array().unwrap().len(), 0);

    let bad_resp = client.call_tool("unlink_tasks", serde_json::json!({"link_id": "nonexistent"}));
    assert!(bad_resp.get("error").is_some());
}

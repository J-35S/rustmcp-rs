//! RustMCP Server Example
//!
//! This example demonstrates how to create a complete RustMCP server with tools, resources, and prompts.
//! The server supports both HTTP and WebSocket connections for full MCP protocol compatibility.

use rustmcp::{RustMCP, FunctionTool, FunctionResource, FunctionPrompt, create_app, ToolAnnotations};
use rustmcp::{ToolDuplicateBehavior, ResourceDuplicateBehavior, PromptDuplicateBehavior};
use serde_json::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // 初始化日志
    env_logger::init();
    
    // 创建RustMCP实例，设置重复行为
    let mut rustmcp = RustMCP::with_behavior(
        ToolDuplicateBehavior::Warn,
        ResourceDuplicateBehavior::Warn,
        PromptDuplicateBehavior::Warn,
    );

    // 添加示例工具 - Echo工具
    // 该工具接收一个消息参数并返回相同的消息
    let echo_tool = FunctionTool::from_function(
        |_args: Option<HashMap<String, Value>>| -> Result<Value, String> {
            let message = _args
                .as_ref()
                .and_then(|m| m.get("message"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("Hello, World!");
            Ok(Value::String(message.to_string()))
        },
        Some("echo".to_string()),
        Some("Echo Tool".to_string()), // title
        Some("Echoes back the provided message".to_string()),
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo"
                }
            },
            "required": ["message"]
        })),
        None, // output_schema
        None, // annotations
        Some(vec!["utility".to_string()]),
        None,
    );
    rustmcp.add_tool(echo_tool);
    
    // 添加执行shell命令的工具
    let shell_tool = FunctionTool::from_function(
        |_args: Option<HashMap<String, Value>>| -> Result<Value, String> {
            let command = _args
                .as_ref()
                .and_then(|m| m.get("command"))
                .and_then(|v| v.as_str())
                .ok_or("Missing 'command' argument")?;
            
            // 安全检查：限制只能执行特定的安全命令
            let allowed_commands = ["ls", "pwd", "date", "echo", "cat", "which"];
            let cmd_parts: Vec<&str> = command.split_whitespace().collect();
            if cmd_parts.is_empty() {
                return Err("Empty command not allowed".to_string());
            }
            
            if !allowed_commands.contains(&cmd_parts[0]) {
                return Err(format!("Command '{}' not allowed", cmd_parts[0]));
            }
            
            // 防止命令注入攻击
            if cmd_parts.iter().any(|&arg| arg.contains(|c| "!\"#$&'()*+,;<=>?@[\\]^`{|}~".contains(c))) {
                return Err("Invalid characters in command arguments".to_string());
            }
            
            // 执行命令
            match std::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output() 
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let result = format!("stdout:\n{}\nstderr:\n{}\nexit_code: {}", 
                                         stdout, stderr, output.status.code().unwrap_or(-1));
                    Ok(Value::String(result))
                }
                Err(e) => Err(format!("Failed to execute command: {}", e)),
            }
        },
        Some("shell".to_string()),
        Some("Shell Tool".to_string()), // title
        Some("Execute safe shell commands".to_string()),
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute (limited to safe commands)"
                }
            },
            "required": ["command"]
        })),
        None, // output_schema
        Some(ToolAnnotations {
            title: Some("Shell Tool".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(false),
            open_world_hint: Some(true),
        }), // annotations
        Some(vec!["system".to_string()]),
        None,
    );
    
    rustmcp.add_tool(shell_tool);
    
    // 添加示例资源 - Hello资源
    // 该资源返回一个简单的问候消息
    let hello_resource = FunctionResource::from_function(
        || -> Result<Value, String> {
            Ok(Value::String("Hello from resource!".to_string()))
        },
        "resource://hello".to_string(),
        Some("hello".to_string()),
        Some("A simple hello resource".to_string()),
        Some("text/plain".to_string()),
        Some(vec!["example".to_string()]),
        None,
        None,
    );
    rustmcp.add_resource(hello_resource);
    
    // 添加一个示例提示
    let greeting_prompt = FunctionPrompt::from_function(
        |_args: Option<HashMap<String, Value>>| -> Result<Vec<rustmcp::PromptMessage>, String> {
            let name = _args
                .as_ref()
                .and_then(|m| m.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("World");
            
            Ok(vec![rustmcp::PromptMessage {
                role: "user".to_string(),
                content: format!("Hello, {}!", name),
                name: None,
            }])
        },
        "greeting".to_string(),
        Some("A simple greeting prompt".to_string()),
        Some(vec!["example".to_string()]),
        None,
        None,
    );
    rustmcp.add_prompt(greeting_prompt);
    
    // 创建并启动服务器
    let app = create_app(rustmcp);
    
    println!("Starting RustMCP server on port 3001...");
    println!("HTTP endpoints available at http://localhost:3001");
    println!("WebSocket endpoint available at ws://localhost:3001/mcp/ws");
    println!("MCP JSON-RPC endpoint available at http://localhost:3001/mcp");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
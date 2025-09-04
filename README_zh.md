# RustMCP

[![Crates.io](https://img.shields.io/crates/v/rustmcp.svg)](https://crates.io/crates/rustmcp)
[![Documentation](https://docs.rs/rustmcp/badge.svg)](https://docs.rs/rustmcp)
[![License](https://img.shields.io/crates/l/rustmcp.svg)](https://github.com/zhangyi/rustmcp-rs/blob/main/LICENSE)

Rust 实现的模型上下文协议 (MCP)，用于构建 AI 代理工具。

## 概述

RustMCP 是一个用 Rust 语言实现的模型上下文协议 (MCP) 库，该协议允许 AI 模型以标准化的方式与工具、资源和提示进行交互。该实现提供了以下功能：

- 工具管理和执行
- 资源处理
- 提示管理
- 完整的 MCP 协议实现（HTTP 和 WebSocket）
- 与 Axum Web 框架轻松集成

## 功能特性

- **工具系统**：注册和执行带参数的工具
- **资源管理**：处理各种类型的资源
- **提示管理**：管理和提供 AI 模型使用的提示
- **MCP 协议**：完整的 MCP 规范实现
- **Web 集成**：使用 Axum 构建的内置 HTTP 和 WebSocket API
- **可扩展性**：易于使用自定义工具、资源和提示进行扩展

## 安装

将以下内容添加到您的 `Cargo.toml` 文件中：

```toml
[dependencies]
rustmcp = "0.1"
```

## 基本用法

```rust
use rustmcp::{RustMCP, FunctionTool};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // 创建 RustMCP 实例
    let mut rustmcp = RustMCP::new();
    
    // 定义一个简单的工具函数
    fn greet_tool(args: Option<HashMap<String, serde_json::Value>>) -> Result<serde_json::Value, String> {
        let name = args
            .and_then(|map| map.get("name").cloned())
            .unwrap_or(serde_json::Value::String("World".to_string()));
        
        let greeting = format!("Hello, {}!", name.as_str().unwrap_or("World"));
        Ok(serde_json::Value::String(greeting))
    }
    
    // 创建并注册工具
    let greet_function_tool = FunctionTool::from_function(
        greet_tool,
        Some("greet".to_string()),
        Some("Greets a person by name".to_string()),
        Some(vec!["greeting".to_string()]),
        None,
        None,
    );
    
    rustmcp.add_tool(greet_function_tool);
    
    // 测试调用工具
    let mut args = HashMap::new();
    args.insert("name".to_string(), serde_json::Value::String("RustMCP".to_string()));
    
    match rustmcp.mcp_call_tool("greet".to_string(), Some(args)).await {
        Ok(result) => println!("Tool result: {}", result.as_str().unwrap_or("Unknown")),
        Err(e) => println!("Error calling tool: {}", e),
    }
}
```

## 服务器示例

RustMCP 包含一个内置的 Web 服务器实现，支持 HTTP 和 WebSocket 连接。以下是一个完整示例：

```rust
use rustmcp::{RustMCP, FunctionTool, FunctionResource, FunctionPrompt, create_app};
use serde_json::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // 创建 RustMCP 实例
    let mut rustmcp = RustMCP::new();
    
    // 添加示例工具
    let echo_tool = FunctionTool::from_function(
        |_args: Option<HashMap<String, Value>>| -> Result<Value, String> {
            let message = _args
                .as_ref()
                .and_then(|m| m.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("Hello, World!");
            Ok(Value::String(message.to_string()))
        },
        Some("echo".to_string()),
        Some("Echoes back the provided message".to_string()),
        Some(vec!["utility".to_string()]),
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
        None,
        None,
    );
    rustmcp.add_tool(echo_tool);
    
    // 添加示例资源
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
    
    // 添加示例提示
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
            }])
        },
        Some("greeting".to_string()),
        Some("A simple greeting prompt".to_string()),
        Some(vec!["example".to_string()]),
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
```

## API 端点

服务器提供以下端点：

- `GET /` - 健康检查端点
- `GET /mcp/tools` - 列出所有工具
- `GET /mcp/resources` - 列出所有资源
- `GET /mcp/prompts` - 列出所有提示
- `POST /mcp/call-tool` - 调用特定工具
- `POST /mcp` - MCP JSON-RPC 端点（用于完整 MCP 协议）
- `GET /mcp/ws` - WebSocket 端点（用于完整 MCP 协议）

## 文档

- [API 文档](https://docs.rs/rustmcp)
- [示例](https://github.com/yourusername/rustmcp-rs/tree/main/examples)

## 许可证

可根据以下任一许可证获得：

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT)

您可以自行选择。

## 贡献

除非您明确声明，否则您有意提交的任何贡献（如 Apache-2.0 许可证所定义）都将按照上述双重许可进行授权，不附带任何其他条款或条件。
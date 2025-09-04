# RustMCP

[![Crates.io](https://img.shields.io/crates/v/rustmcp.svg)](https://crates.io/crates/rustmcp)
[![Documentation](https://docs.rs/rustmcp/badge.svg)](https://docs.rs/rustmcp)
[![License](https://img.shields.io/crates/l/rustmcp.svg)](https://github.com/zhangyi/rustmcp-rs/blob/main/LICENSE)

A Rust implementation of the Model Context Protocol (MCP) for building AI agent tools.

## Overview

RustMCP is a Rust library that implements the Model Context Protocol (MCP), which allows AI models to interact with tools, resources, and prompts in a standardized way. This implementation provides:

- Tool management and execution
- Resource handling
- Prompt management
- Full MCP protocol implementation (HTTP and WebSocket)
- Easy integration with Axum web framework

## Features

- **Tool System**: Register and execute tools with parameter support
- **Resource Management**: Handle various types of resources
- **Prompt Management**: Manage and serve prompts for AI models
- **MCP Protocol**: Full implementation of the MCP specification
- **Web Integration**: Built-in HTTP and WebSocket API using Axum
- **Extensible**: Easy to extend with custom tools, resources, and prompts

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rustmcp = "0.1"
```

## Basic Usage

```rust
use rustmcp::{RustMCP, FunctionTool};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // Create a RustMCP instance
    let mut rustmcp = RustMCP::new();
    
    // Define a simple tool function
    fn greet_tool(args: Option<HashMap<String, serde_json::Value>>) -> Result<serde_json::Value, String> {
        let name = args
            .and_then(|map| map.get("name").cloned())
            .unwrap_or(serde_json::Value::String("World".to_string()));
        
        let greeting = format!("Hello, {}!", name.as_str().unwrap_or("World"));
        Ok(serde_json::Value::String(greeting))
    }
    
    // Create and register the tool
    let greet_function_tool = FunctionTool::from_function(
        greet_tool,
        Some("greet".to_string()),
        Some("Greets a person by name".to_string()),
        Some(vec!["greeting".to_string()]),
        None,
        None,
    );
    
    rustmcp.add_tool(greet_function_tool);
    
    // Test calling the tool
    let mut args = HashMap::new();
    args.insert("name".to_string(), serde_json::Value::String("RustMCP".to_string()));
    
    match rustmcp.mcp_call_tool("greet".to_string(), Some(args)).await {
        Ok(result) => println!("Tool result: {}", result.as_str().unwrap_or("Unknown")),
        Err(e) => println!("Error calling tool: {}", e),
    }
}
```

## Server Example

RustMCP includes a built-in web server implementation that supports both HTTP and WebSocket connections. Here's a complete example:

```rust
use rustmcp::{RustMCP, FunctionTool, FunctionResource, FunctionPrompt, create_app};
use serde_json::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    // Create RustMCP instance
    let mut rustmcp = RustMCP::new();
    
    // Add a sample tool
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
    
    // Add a sample resource
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
    
    // Add a sample prompt
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
    
    // Create and start the server
    let app = create_app(rustmcp);
    
    println!("Starting RustMCP server on port 3001...");
    println!("HTTP endpoints available at http://localhost:3001");
    println!("WebSocket endpoint available at ws://localhost:3001/mcp/ws");
    println!("MCP JSON-RPC endpoint available at http://localhost:3001/mcp");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## API Endpoints

The server provides the following endpoints:

- `GET /` - Health check endpoint
- `GET /mcp/tools` - List all tools
- `GET /mcp/resources` - List all resources
- `GET /mcp/prompts` - List all prompts
- `POST /mcp/call-tool` - Call a specific tool
- `POST /mcp` - MCP JSON-RPC endpoint (for full MCP protocol)
- `GET /mcp/ws` - WebSocket endpoint (for full MCP protocol)

## Documentation

- [API Documentation](https://docs.rs/rustmcp)
- [Examples](https://github.com/yourusername/rustmcp-rs/tree/main/examples)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
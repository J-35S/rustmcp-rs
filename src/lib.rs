//! RustMCP - 一个Rust实现的Model Context Protocol库
//!
//! 这个库提供了构建MCP兼容服务器所需的所有组件，
//! 包括工具、资源和提示的管理功能。
//!
//! ## 功能特性
//!
//! - 工具管理: 注册和执行自定义工具函数
//! - 资源管理: 管理和提供各种类型的资源
//! - 提示管理: 创建和管理AI模型使用的提示
//! - 完整MCP协议: 实现完整的MCP协议规范，支持HTTP和WebSocket
//! - Web集成: 基于Axum框架的内置HTTP服务器
//!
//! ## 快速开始
//!
//! ```rust
//! use rustmcp::{RustMCP, FunctionTool};
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() {
//!     // 创建RustMCP实例
//!     let mut rustmcp = RustMCP::new();
//!     
//!     // 定义一个简单的工具函数
//!     fn greet_tool(args: Option<HashMap<String, serde_json::Value>>) -> Result<serde_json::Value, String> {
//!         let name = args
//!             .and_then(|map| map.get("name").cloned())
//!             .unwrap_or(serde_json::Value::String("World".to_string()));
//!         
//!         let greeting = format!("Hello, {}!", name.as_str().unwrap_or("World"));
//!         Ok(serde_json::Value::String(greeting))
//!     }
//!     
//!     // 创建并注册工具
//!     let greet_function_tool = FunctionTool::from_function(
//!         greet_tool,
//!         Some("greet".to_string()),
//!         Some("Greets a person by name".to_string()),
//!         Some(vec!["greeting".to_string()]),
//!         None,
//!         None,
//!     );
//!     
//!     rustmcp.add_tool(greet_function_tool);
//!     
//!     // 测试调用工具
//!     let mut args = HashMap::new();
//!     args.insert("name".to_string(), serde_json::Value::String("RustMCP".to_string()));
//!     
//!     match rustmcp.mcp_call_tool("greet".to_string(), Some(args)).await {
//!         Ok(result) => println!("Tool result: {}", result.as_str().unwrap_or("Unknown")),
//!         Err(e) => println!("Error calling tool: {}", e),
//!     }
//! }
//! ```

//! ## 创建完整服务器
//!
//! ```rust
//! use rustmcp::{RustMCP, FunctionTool, create_app};
//! use serde_json::Value;
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut rustmcp = RustMCP::new();
//!     
//!     // 添加工具
//!     let echo_tool = FunctionTool::from_function(
//!         |_args: Option<HashMap<String, Value>>| -> Result<Value, String> {
//!             let message = _args
//!                 .as_ref()
//!                 .and_then(|m| m.get("message"))
//!                 .and_then(|v| v.as_str())
//!                 .unwrap_or("Hello, World!");
//!             Ok(Value::String(message.to_string()))
//!         },
//!         Some("echo".to_string()),
//!         Some("Echoes back the provided message".to_string()),
//!         Some(vec!["utility".to_string()]),
//!         Some(serde_json::json!({
//!             "type": "object",
//!             "properties": {
//!                 "message": {
//!                     "type": "string",
//!                     "description": "The message to echo"
//!                 }
//!             },
//!             "required": ["message"]
//!         })),
//!         None,
//!         None,
//!     );
//!     rustmcp.add_tool(echo_tool);
//!     
//!     // 创建服务器应用
//!     let app = create_app(rustmcp);
//!     
//!     // 启动服务器
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```

/// 主版本号
pub const MAJOR_VERSION: u32 = 0;
/// 次版本号
pub const MINOR_VERSION: u32 = 1;
/// 修订版本号
pub const PATCH_VERSION: u32 = 0;

pub mod server;
mod settings;

pub use server::{RustMCP, Context};
pub use server::tools::{FunctionTool, ToolAnnotations, DuplicateBehavior as ToolDuplicateBehavior};
pub use server::resources::{FunctionResource, Resource, DuplicateBehavior as ResourceDuplicateBehavior};
pub use server::prompts::{FunctionPrompt, Prompt, PromptMessage, DuplicateBehavior as PromptDuplicateBehavior};
pub use server::{create_app};

/// 获取库版本
pub fn version() -> String {
    format!("{}.{}.{}", MAJOR_VERSION, MINOR_VERSION, PATCH_VERSION)
}
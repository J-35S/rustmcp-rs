//! RustMCP服务器模块
//!
//! 这个模块包含了RustMCP的所有核心功能实现，包括：
//! - [RustMCP][]: 核心管理器，协调所有MCP功能
//! - [Context][]: 上下文管理器，处理MCP会话上下文
//! - 工具管理器: 工具注册、查找和执行
//! - 资源管理器: 资源注册、查找和读取
//! - 提示管理器: 提示注册、查找和获取
//!
//! ## 使用示例
//!
//! ```rust
//! use rustmcp::{RustMCP, FunctionTool, create_app};
//!
//! // 创建RustMCP实例
//! let mut rustmcp = RustMCP::new();
//!
//! // 添加工具
//! let echo_tool = FunctionTool::from_function(
//!     |_args: Option<std::collections::HashMap<String, serde_json::Value>>| -> Result<serde_json::Value, String> {
//!         let message = _args
//!             .as_ref()
//!             .and_then(|m| m.get("message"))
//!             .and_then(|v| v.as_str())
//!             .unwrap_or("Hello, World!");
//!         Ok(serde_json::Value::String(message.to_string()))
//!     },
//!     Some("echo".to_string()),
//!     Some("Echoes back the provided message".to_string()),
//!     Some(vec!["utility".to_string()]),
//!     Some(serde_json::json!({
//!         "type": "object",
//!         "properties": {
//!             "message": {
//!                 "type": "string",
//!                 "description": "The message to echo"
//!             }
//!         },
//!         "required": ["message"]
//!     })),
//!     None,
//!     None,
//! );
//! rustmcp.add_tool(echo_tool);
//!
//! // 创建Axum应用
//! let app = create_app(rustmcp);
//! ```
//!
//! ## 模块结构
//!
//! - [tools](tools/index.html): 工具管理实现
//! - [resources](resources/index.html): 资源管理实现
//! - [prompts](prompts/index.html): 提示管理实现
//! - [ws](ws/index.html): WebSocket支持实现

pub mod tools;
pub mod resources;
pub mod prompts;
pub mod ws;

use axum::{
    extract::{State},
    response::{IntoResponse, Json},
    http::StatusCode,
    http::HeaderMap,
    body::Bytes,
    routing::{get, post},
    Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// 重新导出主要类型
pub use tools::{ToolManager, FunctionTool, DuplicateBehavior as ToolDuplicateBehavior};
pub use resources::{ResourceManager, Resource, FunctionResource, DuplicateBehavior as ResourceDuplicateBehavior};
pub use prompts::{PromptManager, Prompt, FunctionPrompt, PromptMessage, DuplicateBehavior as PromptDuplicateBehavior};

/// RustMCP上下文
#[derive(Debug, Clone)]
pub struct Context {
    // 可以添加上下文相关字段
}

/// RustMCP核心类
#[derive(Debug, Clone)]
pub struct RustMCP {
    tool_manager: ToolManager,
    resource_manager: ResourceManager,
    prompt_manager: PromptManager,
}

impl RustMCP {
    /// 创建新的RustMCP实例
    pub fn new() -> Self {
        Self {
            tool_manager: ToolManager::new(),
            resource_manager: ResourceManager::new(),
            prompt_manager: PromptManager::new(),
        }
    }
    
    /// 使用指定的重复行为创建新的RustMCP实例
    pub fn with_behavior(
        tool_behavior: ToolDuplicateBehavior,
        resource_behavior: ResourceDuplicateBehavior,
        prompt_behavior: PromptDuplicateBehavior,
    ) -> Self {
        Self {
            tool_manager: ToolManager::with_behavior(tool_behavior),
            resource_manager: ResourceManager::with_behavior(resource_behavior),
            prompt_manager: PromptManager::with_behavior(prompt_behavior),
        }
    }
    
    /// 添加工具
    pub fn add_tool(&mut self, tool: FunctionTool) {
        self.tool_manager.add_tool(tool);
    }
    
    /// 添加资源
    pub fn add_resource(&mut self, resource: FunctionResource) {
        self.resource_manager.add_resource(resource);
    }
    
    /// 添加提示
    pub fn add_prompt(&mut self, prompt: FunctionPrompt) {
        self.prompt_manager.add_prompt(prompt);
    }
    
    /// 列出所有工具
    pub fn mcp_list_tools(&self) -> Vec<&tools::FunctionTool> {
        self.tool_manager.list_tools()
    }
    
    /// 列出所有资源
    pub fn mcp_list_resources(&self) -> Vec<Resource> {
        self.resource_manager.list_resources()
    }
    
    /// 列出所有提示
    pub fn mcp_list_prompts(&self) -> Vec<Prompt> {
        self.prompt_manager.list_prompts()
    }
    
    /// 调用工具
    pub async fn mcp_call_tool(&self, name: &str, arguments: Option<HashMap<String, Value>>) -> Result<Value, String> {
        self.tool_manager.call_tool(name, arguments)
    }
    
    /// 读取资源
    pub fn mcp_read_resource(&self, uri: &str) -> Result<Value, String> {
        self.resource_manager.read_resource(uri)
    }
    
    /// 获取提示
    pub fn mcp_get_prompt(&self, name: &str, arguments: Option<HashMap<String, Value>>) -> Result<Vec<PromptMessage>, String> {
        self.prompt_manager.get_prompt(name, arguments)
    }
}

impl Default for RustMCP {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建Axum应用
pub fn create_app(rustmcp: RustMCP) -> Router {
    let shared_state = Arc::new(rustmcp);
    
    Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/mcp/tools", get(mcp_list_tools_handler))
        .route("/mcp/resources", get(mcp_list_resources_handler))
        .route("/mcp/prompts", get(mcp_list_prompts_handler))
        .route("/mcp/call-tool", post(mcp_call_tool_handler))
        .route("/mcp", post(mcp_jsonrpc_handler))
        .route("/mcp/ws", get(ws::ws_handler))
        .with_state(shared_state)
}

// HTTP处理函数
async fn root() -> &'static str {
    "Welcome to RustMCP-rs server!"
}

async fn health_check() -> &'static str {
    "OK"
}

async fn mcp_list_tools_handler(State(rustmcp): State<Arc<RustMCP>>) -> String {
    let tools = rustmcp.mcp_list_tools();
    serde_json::to_string(&tools).unwrap_or_else(|_| "[]".to_string())
}

async fn mcp_list_resources_handler(State(rustmcp): State<Arc<RustMCP>>) -> String {
    let resources = rustmcp.mcp_list_resources();
    serde_json::to_string(&resources).unwrap_or_else(|_| "[]".to_string())
}

async fn mcp_list_prompts_handler(State(rustmcp): State<Arc<RustMCP>>) -> String {
    let prompts = rustmcp.mcp_list_prompts();
    serde_json::to_string(&prompts).unwrap_or_else(|_| "[]".to_string())
}

async fn mcp_call_tool_handler(
    State(rustmcp): State<Arc<RustMCP>>,
    body: String,
) -> Result<String, (StatusCode, String)> {
    #[derive(Deserialize)]
    struct CallToolRequest {
        name: String,
        arguments: Option<std::collections::HashMap<String, Value>>,
    }

    let request: CallToolRequest = serde_json::from_str(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    match rustmcp.mcp_call_tool(&request.name, request.arguments).await {
        Ok(result) => Ok(serde_json::to_string(&result)
            .unwrap_or_else(|_| r#"{"error": "Failed to serialize result"}"#.to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

// JSON-RPC处理函数
async fn mcp_jsonrpc_handler(
    State(rustmcp): State<Arc<RustMCP>>,
    headers: HeaderMap,
    request: Bytes,
) -> impl IntoResponse {
    // 记录请求头和内容
    println!("Received request headers: {:?}", headers);
    println!("Received request body: {}", String::from_utf8_lossy(&request));
    
    // 解析JSON-RPC请求
    let request: JsonRpcRequest = match serde_json::from_slice(&request) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Failed to parse JSON-RPC request: {}", e);
            return (StatusCode::UNPROCESSABLE_ENTITY, format!("Failed to parse JSON: {}", e)).into_response();
        }
    };
    
    // 记录请求日志
    println!("Received JSON-RPC request: method={}, id={:?}", request.method, request.id);
    
    // 处理通知消息（没有id的消息）
    if request.id.is_none() {
        match request.method.as_str() {
            "notifications/initialized" => {
                // initialized通知不需要响应
                println!("Received initialized notification, sending success response");
                // 对于通知消息，发送一个特殊的成功响应
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(serde_json::Value::Number(serde_json::Number::from(0))),
                    result: Some(serde_json::json!({})),
                    error: None,
                };
                return (StatusCode::OK, [("content-type", "application/json")], Json(response)).into_response();
            }
            _ => {
                println!("Unknown notification: {}, sending success response", request.method);
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(serde_json::Value::Number(serde_json::Number::from(0))),
                    result: Some(serde_json::json!({})),
                    error: None,
                };
                return (StatusCode::OK, [("content-type", "application/json")], Json(response)).into_response();
            }
        }
    }
    
    // 为日志输出创建id的克隆
    let request_id_for_log = request.id.clone();
    
    // 处理请求消息（有id的消息）
    let response = match request.method.as_str() {
        "initialize" => {
            // 构造响应
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {
                        "listChanged": true
                    },
                    "resources": {
                        "subscribe": true,
                        "listChanged": true
                    },
                    "prompts": {
                        "listChanged": true
                    }
                },
                "serverInfo": {
                    "name": "RustMCP-rs",
                    "version": "0.1.0"
                }
            });

            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id, // 保持原始ID
                result: Some(result),
                error: None,
            }
        },
        "tools/list" => {
            let tools = rustmcp.mcp_list_tools();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::json!({
                    "tools": tools
                })),
                error: None,
            }
        },
        "resources/list" => {
            let resources = rustmcp.mcp_list_resources();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::json!({
                    "resources": resources
                })),
                error: None,
            }
        },
        "prompts/list" => {
            let prompts = rustmcp.mcp_list_prompts();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(serde_json::json!({
                    "prompts": prompts
                })),
                error: None,
            }
        },
        "tools/call" => {
            if let Some(params) = request.params {
                if let Ok(call_params) = serde_json::from_value::<serde_json::Map<String, Value>>(params) {
                    let name = call_params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let arguments = call_params.get("arguments").cloned();
                    
                    // 转换arguments为HashMap
                    let arguments_map = if let Some(args) = arguments {
                        serde_json::from_value::<std::collections::HashMap<String, Value>>(args).ok()
                    } else {
                        None
                    };
                    
                    match rustmcp.mcp_call_tool(name, arguments_map).await {
                        Ok(result) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: Some(serde_json::json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("{}", result)
                                }],
                                "isError": false
                            })),
                            error: None,
                        },
                        Err(e) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: Some(serde_json::json!({
                                "content": [{
                                    "type": "text",
                                    "text": e
                                }],
                                "isError": true
                            })),
                            error: None,
                        },
                    }
                } else {
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Invalid params".to_string(),
                            data: None,
                        }),
                    }
                }
            } else {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing params".to_string(),
                        data: None,
                    }),
                }
            }
        },
        "resources/read" => {
            if let Some(params) = request.params {
                if let Ok(read_params) = serde_json::from_value::<serde_json::Map<String, Value>>(params) {
                    let uri = read_params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                    
                    match rustmcp.mcp_read_resource(uri) {
                        Ok(result) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: Some(serde_json::json!({
                                "contents": [{
                                    "uri": uri,
                                    "text": result
                                }]
                            })),
                            error: None,
                        },
                        Err(e) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32000,
                                message: e,
                                data: None,
                            }),
                        },
                    }
                } else {
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Invalid params".to_string(),
                            data: None,
                        }),
                    }
                }
            } else {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing params".to_string(),
                        data: None,
                    }),
                }
            }
        },
        "prompts/get" => {
            if let Some(params) = request.params {
                if let Ok(get_params) = serde_json::from_value::<serde_json::Map<String, Value>>(params) {
                    let name = get_params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let arguments = get_params.get("arguments").cloned();
                    
                    // 转换arguments为HashMap
                    let arguments_map = if let Some(args) = arguments {
                        serde_json::from_value::<std::collections::HashMap<String, Value>>(args).ok()
                    } else {
                        None
                    };
                    
                    match rustmcp.mcp_get_prompt(name, arguments_map) {
                        Ok(messages) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: Some(serde_json::json!({
                                "messages": messages
                            })),
                            error: None,
                        },
                        Err(e) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32000,
                                message: e,
                                data: None,
                            }),
                        },
                    }
                } else {
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Invalid params".to_string(),
                            data: None,
                        }),
                    }
                }
            } else {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing params".to_string(),
                        data: None,
                    }),
                }
            }
        },
        _ => {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
            }
        }
    };
    
    // 记录响应日志
    println!("Sending JSON-RPC response: id={:?}", request_id_for_log);
    if let Some(ref result) = response.result {
        println!("Response body: {}", serde_json::to_string(result).unwrap_or_else(|_| "无法序列化响应".to_string()));
    }
    
    // 返回响应
    (StatusCode::OK, [("content-type", "application/json")], Json(response)).into_response()
}

// JSON-RPC数据结构定义
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

// JSON-RPC响应结构
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

// JSON-RPC错误结构
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}
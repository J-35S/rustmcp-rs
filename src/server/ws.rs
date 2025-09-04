//! WebSocket和JSON-RPC支持模块
//! 实现MCP协议的WebSocket传输层

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::server::RustMCP;

/// JSON-RPC请求结构
#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC响应结构
#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC通知结构
#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC错误结构
#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// WebSocket连接处理函数
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RustMCP>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// 客户端状态
#[derive(Debug)]
pub struct ClientState {
    // 可以添加客户端特定的状态信息
}

impl ClientState {
    /// 创建新的客户端状态
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ClientState {
    fn default() -> Self {
        Self::new()
    }
}

/// 处理WebSocket连接
async fn handle_socket(socket: WebSocket, state: Arc<RustMCP>) {
    println!("WebSocket connection established");
    
    // 创建客户端状态
    let client_state = Arc::new(Mutex::new(ClientState::new()));
    
    // 分离读写
    let (mut sender, mut receiver) = socket.split();
    
    // 处理接收消息的任务
    let state_clone = state.clone();
    let client_state_clone = client_state.clone();
    let receiver_handle = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                println!("Received message: {}", text);
                if let Err(e) = handle_message(text, &state_clone, &mut sender, &client_state_clone).await {
                    eprintln!("Error handling message: {}", e);
                    break;
                }
            }
        }
    });
    
    // 等待任务完成
    let _ = receiver_handle.await;
    println!("WebSocket connection closed");
}

/// 处理接收到的消息
async fn handle_message(
    text: String,
    state: &Arc<RustMCP>,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    _client_state: &Arc<Mutex<ClientState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 解析JSON-RPC请求
    if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&text) {
        let response = match request.method.as_str() {
            "initialize" => {
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
                if let Ok(response_text) = serde_json::to_string(&response) {
                    let _ = sender.send(Message::Text(response_text)).await;
                }
                return Ok(());
            },
            _ => {
                // 转发到HTTP处理器处理其他方法
                handle_jsonrpc_method(request, state).await
            }
        };

        // 发送响应
        if let Ok(response_text) = serde_json::to_string(&response) {
            sender.send(Message::Text(response_text)).await?;
        }
    }
    
    Ok(())
}

/// 处理JSON-RPC方法调用
async fn handle_jsonrpc_method(request: JsonRpcRequest, state: &Arc<RustMCP>) -> JsonRpcResponse {
    match request.method.as_str() {
        "tools/list" => {
            let tools = state.mcp_list_tools();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: Some(serde_json::json!({
                    "tools": tools
                })),
                error: None,
            }
        },
        "resources/list" => {
            let resources = state.mcp_list_resources();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: Some(serde_json::json!({
                    "resources": resources
                })),
                error: None,
            }
        },
        "prompts/list" => {
            let prompts = state.mcp_list_prompts();
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
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
                    
                    match state.mcp_call_tool(name, arguments_map).await {
                        Ok(result) => JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id.clone(),
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
                            id: request.id.clone(),
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
                    
                    match state.mcp_read_resource(uri) {
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
        _ => {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
            }
        }
    }
}

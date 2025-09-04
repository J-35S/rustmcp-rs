use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use log::warn;

/// 工具函数类型定义
pub type ToolFunction = Box<dyn Fn(Option<HashMap<String, Value>>) -> Result<Value, String> + Send + Sync>;

/// 重复工具处理行为
#[derive(Debug, Clone)]
pub enum DuplicateBehavior {
    Warn,
    Error,
    Replace,
    Ignore,
}

/// 工具注解结构体
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolAnnotations {
    /// 工具的人类可读标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 工具是否为只读
    #[serde(rename = "readOnlyHint", skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    /// 工具是否具有破坏性
    #[serde(rename = "destructiveHint", skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    /// 工具是否幂等
    #[serde(rename = "idempotentHint", skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    /// 工具是否与开放世界交互
    #[serde(rename = "openWorldHint", skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

/// 函数式工具结构体
#[derive(Serialize, Deserialize)]
pub struct FunctionTool {
    /// 工具名称
    pub name: String,
    /// 工具的人类可读标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 工具描述
    pub description: String,
    /// 工具参数的JSON Schema
    #[serde(rename = "inputSchema", skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
    /// 工具输出的JSON Schema（可选）
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// 工具注解
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    /// 工具标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// 工具元数据
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
    
    /// 工具函数（不参与序列化）
    #[serde(skip)]
    function: Option<Arc<ToolFunction>>,
}

// 手动实现Clone trait
impl Clone for FunctionTool {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
            output_schema: self.output_schema.clone(),
            annotations: self.annotations.clone(),
            tags: self.tags.clone(),
            meta: self.meta.clone(),
            function: None, // 函数对象不参与克隆
        }
    }
}

// 手动实现Debug trait
impl std::fmt::Debug for FunctionTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionTool")
            .field("name", &self.name)
            .field("title", &self.title)
            .field("description", &self.description)
            .field("input_schema", &self.input_schema)
            .field("output_schema", &self.output_schema)
            .field("annotations", &self.annotations)
            .field("tags", &self.tags)
            .field("meta", &self.meta)
            .finish()
    }
}

impl FunctionTool {
    /// 从函数创建工具
    /// 
    /// # Arguments
    /// * `function` - 要包装的函数
    /// * `name` - 工具名称
    /// * `title` - 工具标题
    /// * `description` - 工具描述
    /// * `input_schema` - 输入模式
    /// * `output_schema` - 输出模式
    /// * `annotations` - 注解
    /// * `tags` - 标签
    /// * `meta` - 元数据
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub fn from_function<F>(
        function: F,
        name: Option<String>,
        title: Option<String>,
        description: Option<String>,
        input_schema: Option<Value>,
        output_schema: Option<Value>,
        annotations: Option<ToolAnnotations>,
        tags: Option<Vec<String>>,
        meta: Option<Value>,
    ) -> Self
    where
        F: Fn(Option<HashMap<String, Value>>) -> Result<Value, String> + Send + Sync + 'static,
    {
        Self {
            function: Some(Arc::new(Box::new(function))),
            name: name.unwrap_or_else(|| "unnamed_tool".to_string()),
            title,
            description: description.unwrap_or_default(),
            input_schema,
            output_schema,
            annotations,
            tags,
            meta,
        }
    }

    /// 调用工具函数
    #[allow(dead_code)]
    pub fn call(&self, args: Option<HashMap<String, Value>>) -> Result<Value, String> {
        if let Some(ref function) = self.function {
            function(args)
        } else {
            Err("Tool function not available".to_string())
        }
    }
}

/// 工具管理器
#[derive(Debug, Clone)]
pub struct ToolManager {
    tools: HashMap<String, FunctionTool>,
    duplicate_behavior: DuplicateBehavior,
}

impl ToolManager {
    /// 创建新的工具管理器
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            duplicate_behavior: DuplicateBehavior::Warn,
        }
    }
    
    /// 创建具有指定重复行为的新工具管理器
    pub fn with_behavior(duplicate_behavior: DuplicateBehavior) -> Self {
        Self {
            tools: HashMap::new(),
            duplicate_behavior,
        }
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolManager {
    /// 添加工具
    #[allow(dead_code)]
    pub fn add_tool(&mut self, tool: FunctionTool) {
        if self.tools.contains_key(&tool.name) {
            match self.duplicate_behavior {
                DuplicateBehavior::Warn => {
                    warn!("Tool '{}' already exists, replacing", tool.name);
                    self.tools.insert(tool.name.clone(), tool);
                }
                DuplicateBehavior::Error => {
                    panic!("Tool '{}' already exists", tool.name);
                }
                DuplicateBehavior::Replace => {
                    self.tools.insert(tool.name.clone(), tool);
                }
                DuplicateBehavior::Ignore => {
                    // 不添加新工具
                }
            }
        } else {
            self.tools.insert(tool.name.clone(), tool);
        }
    }

    /// 获取工具
    #[allow(dead_code)]
    pub fn get_tool(&self, name: &str) -> Option<&FunctionTool> {
        self.tools.get(name)
    }

    /// 列出所有工具
    #[allow(dead_code)]
    pub fn list_tools(&self) -> Vec<&FunctionTool> {
        self.tools.values().collect()
    }

    /// 调用工具
    #[allow(dead_code)]
    pub fn call_tool(&self, name: &str, args: Option<HashMap<String, Value>>) -> Result<Value, String> {
        if let Some(tool) = self.get_tool(name) {
            tool.call(args)
        } else {
            Err(format!("Tool '{}' not found", name))
        }
    }
}
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use log::warn;

/// 提示消息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PromptMessage {
    /// 角色
    pub role: String,
    
    /// 内容
    pub content: String,
    
    /// 名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// 提示定义
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Prompt {
    /// 提示名称
    pub name: String,
    
    /// 提示描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// 标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    
    /// 注解
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, Value>>,
    
    /// 元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, Value>>,
}

type PromptFunction = Arc<dyn Fn(Option<HashMap<String, Value>>) -> Result<Vec<PromptMessage>, String> + Send + Sync>;

/// 函数式提示
#[derive(Clone)]
pub struct FunctionPrompt {
    /// 提示函数
    pub function: Option<PromptFunction>,
    
    /// 提示名称
    pub name: String,
    
    /// 提示描述
    pub description: String,
    
    /// 标签
    pub tags: Vec<String>,
    
    /// 参数
    pub arguments: Option<HashMap<String, String>>,
    
    /// 元数据
    pub meta: Option<Value>,
}

impl FunctionPrompt {
    /// 从函数创建提示
    /// 
    /// # Arguments
    /// * `function` - 要包装的函数
    /// * `name` - 提示名称
    /// * `description` - 提示描述
    /// * `tags` - 标签
    /// * `arguments` - 参数
    /// * `meta` - 元数据
    #[allow(clippy::too_many_arguments)]
    pub fn from_function<F>(
        function: F,
        name: String,
        description: Option<String>,
        tags: Option<Vec<String>>,
        arguments: Option<HashMap<String, String>>,
        meta: Option<Value>,
    ) -> Self
    where
        F: Fn(Option<HashMap<String, Value>>) -> Result<Vec<PromptMessage>, String> + Send + Sync + 'static,
    {
        Self {
            function: Some(Arc::new(function)),
            name,
            description: description.unwrap_or_default(),
            tags: tags.unwrap_or_default(),
            arguments,
            meta,
        }
    }
    
    /// 获取提示
    pub fn get(&self, arguments: Option<HashMap<String, Value>>) -> Result<Vec<PromptMessage>, String> {
        if let Some(func) = &self.function {
            func(arguments)
        } else {
            Err("Prompt function not available".to_string())
        }
    }
}

impl Serialize for FunctionPrompt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // 只序列化Prompt部分，不包括函数
        let prompt = Prompt {
            name: self.name.clone(),
            description: if self.description.is_empty() { None } else { Some(self.description.clone()) },
            tags: if self.tags.is_empty() { None } else { Some(self.tags.clone()) },
            annotations: None, // 注解字段已移除
            meta: match &self.meta {
                Some(Value::Object(obj)) if !obj.is_empty() => {
                    let map: HashMap<String, Value> = obj.clone().into_iter().collect();
                    Some(map)
                },
                _ => None,
            },
        };
        prompt.serialize(serializer)
    }
}

impl std::fmt::Debug for FunctionPrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionPrompt")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("tags", &self.tags)
            .field("arguments", &self.arguments)
            .field("meta", &self.meta)
            .finish()
    }
}

/// 重复提示处理行为
#[derive(Debug, Clone)]
pub enum DuplicateBehavior {
    Warn,
    Error,
    Replace,
    Ignore,
}

/// 提示管理器
#[derive(Debug, Clone)]
pub struct PromptManager {
    /// 提示集合
    prompts: HashMap<String, FunctionPrompt>,
    duplicate_behavior: DuplicateBehavior,
}

impl PromptManager {
    /// 创建新的提示管理器
    pub fn new() -> Self {
        Self {
            prompts: HashMap::new(),
            duplicate_behavior: DuplicateBehavior::Warn,
        }
    }
    
    /// 创建具有指定重复行为的新提示管理器
    pub fn with_behavior(duplicate_behavior: DuplicateBehavior) -> Self {
        Self {
            prompts: HashMap::new(),
            duplicate_behavior,
        }
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptManager {
    /// 添加提示
    pub fn add_prompt(&mut self, prompt: FunctionPrompt) {
        if self.prompts.contains_key(&prompt.name) {
            match self.duplicate_behavior {
                DuplicateBehavior::Warn => {
                    warn!("Prompt '{}' already exists, replacing", prompt.name);
                    self.prompts.insert(prompt.name.clone(), prompt);
                }
                DuplicateBehavior::Error => {
                    panic!("Prompt '{}' already exists", prompt.name);
                }
                DuplicateBehavior::Replace => {
                    self.prompts.insert(prompt.name.clone(), prompt);
                }
                DuplicateBehavior::Ignore => {
                    // 不添加新提示
                }
            }
        } else {
            self.prompts.insert(prompt.name.clone(), prompt);
        }
    }
    
    /// 列出所有提示
    pub fn list_prompts(&self) -> Vec<Prompt> {
        self.prompts.values().map(|p| {
            Prompt {
                name: p.name.clone(),
                description: if p.description.is_empty() { None } else { Some(p.description.clone()) },
                tags: if p.tags.is_empty() { None } else { Some(p.tags.clone()) },
                annotations: None, // 注解字段已移除
                meta: match &p.meta {
                    Some(Value::Object(obj)) if !obj.is_empty() => {
                        let map: HashMap<String, Value> = obj.clone().into_iter().collect();
                        Some(map)
                    },
                    _ => None,
                },

            }
        }).collect()
    }
    
    /// 获取提示函数
    #[allow(clippy::type_complexity)]
    pub fn get_prompt_function(&self, name: &str) -> Option<PromptFunction> {
        self.prompts.get(name).and_then(|prompt| prompt.function.clone())
    }
    
    /// 获取提示
    pub fn get_prompt(&self, name: &str, arguments: Option<HashMap<String, Value>>) -> Result<Vec<PromptMessage>, String> {
        if let Some(prompt) = self.prompts.get(name) {
            prompt.get(arguments)
        } else {
            Err(format!("Prompt not found: {}", name))
        }
    }
}
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use log::warn;

/// 资源定义
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Resource {
    /// 资源URI
    pub uri: String,
    
    /// 资源名称
    pub name: String,
    
    /// 资源描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// MIME类型
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    
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

/// 函数式资源
#[derive(Clone)]
pub struct FunctionResource {
    /// 资源函数
    pub function: Arc<dyn Fn() -> Result<Value, String> + Send + Sync>,
    
    /// 资源URI
    pub uri: String,
    
    /// 资源名称
    pub name: String,
    
    /// 资源描述
    pub description: String,
    
    /// MIME类型
    pub mime_type: String,
    
    /// 标签
    pub tags: Vec<String>,
    
    /// 注解
    pub annotations: HashMap<String, Value>,
    
    /// 元数据
    pub meta: Option<HashMap<String, Value>>,
}

impl FunctionResource {
    /// 从函数创建资源
    /// 
    /// # Arguments
    /// * `function` - 要包装的函数
    /// * `uri` - 资源URI
    /// * `name` - 资源名称
    /// * `description` - 资源描述
    /// * `mime_type` - MIME类型
    /// * `tags` - 标签
    /// * `annotations` - 注解
    /// * `meta` - 元数据
    #[allow(clippy::too_many_arguments)]
    pub fn from_function<F>(
        function: F,
        uri: String,
        name: Option<String>,
        description: Option<String>,
        mime_type: Option<String>,
        tags: Option<Vec<String>>,
        annotations: Option<HashMap<String, Value>>,
        meta: Option<HashMap<String, Value>>,
    ) -> Self
    where
        F: Fn() -> Result<Value, String> + Send + Sync + 'static,
    {
        Self {
            function: Arc::new(function),
            uri: uri.clone(),
            name: name.unwrap_or_else(|| "unnamed_resource".to_string()),
            description: description.unwrap_or_default(),
            mime_type: mime_type.unwrap_or_else(|| "text/plain".to_string()),
            tags: tags.unwrap_or_default(),
            annotations: annotations.unwrap_or_default(),
            meta,
        }
    }
    
    /// 读取资源
    pub fn read(&self) -> Result<Value, String> {
        (self.function)()
    }
}

impl Serialize for FunctionResource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 只序列化Resource部分，不包括函数
        let resource = Resource {
            uri: self.uri.clone(),
            name: self.name.clone(),
            description: if self.description.is_empty() { None } else { Some(self.description.clone()) },
            mime_type: if self.mime_type.is_empty() { None } else { Some(self.mime_type.clone()) },
            tags: if self.tags.is_empty() { None } else { Some(self.tags.clone()) },
            annotations: if self.annotations.is_empty() { None } else { Some(self.annotations.clone()) },
            meta: self.meta.clone(),
        };
        resource.serialize(serializer)
    }
}

impl std::fmt::Debug for FunctionResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionResource")
            .field("uri", &self.uri)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("mime_type", &self.mime_type)
            .field("tags", &self.tags)
            .field("annotations", &self.annotations)
            .field("meta", &self.meta)
            .finish()
    }
}

/// 重复资源处理行为
#[derive(Debug, Clone)]
pub enum DuplicateBehavior {
    Warn,
    Error,
    Replace,
    Ignore,
}

/// 资源管理器
#[derive(Debug, Clone)]
pub struct ResourceManager {
    /// 资源集合
    resources: HashMap<String, FunctionResource>,
    duplicate_behavior: DuplicateBehavior,
}

impl ResourceManager {
    /// 创建新的资源管理器
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            duplicate_behavior: DuplicateBehavior::Warn,
        }
    }
    
    /// 创建具有指定重复行为的新资源管理器
    pub fn with_behavior(duplicate_behavior: DuplicateBehavior) -> Self {
        Self {
            resources: HashMap::new(),
            duplicate_behavior,
        }
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceManager {
    /// 添加资源
    pub fn add_resource(&mut self, resource: FunctionResource) {
        if self.resources.contains_key(&resource.uri) {
            match self.duplicate_behavior {
                DuplicateBehavior::Warn => {
                    warn!("Resource '{}' already exists, replacing", resource.uri);
                    self.resources.insert(resource.uri.clone(), resource);
                }
                DuplicateBehavior::Error => {
                    panic!("Resource '{}' already exists", resource.uri);
                }
                DuplicateBehavior::Replace => {
                    self.resources.insert(resource.uri.clone(), resource);
                }
                DuplicateBehavior::Ignore => {
                    // 不添加新资源
                }
            }
        } else {
            self.resources.insert(resource.uri.clone(), resource);
        }
    }
    
    /// 列出所有资源
    pub fn list_resources(&self) -> Vec<Resource> {
        self.resources.values().map(|r| {
            Resource {
                uri: r.uri.clone(),
                name: r.name.clone(),
                description: if r.description.is_empty() { None } else { Some(r.description.clone()) },
                mime_type: if r.mime_type.is_empty() { None } else { Some(r.mime_type.clone()) },
                tags: if r.tags.is_empty() { None } else { Some(r.tags.clone()) },
                annotations: if r.annotations.is_empty() { None } else { Some(r.annotations.clone()) },
                meta: r.meta.clone(),
            }
        }).collect()
    }
    
    /// 读取资源
    pub fn read_resource(&self, uri: &str) -> Result<Value, String> {
        if let Some(resource) = self.resources.get(uri) {
            resource.read()
        } else {
            Err(format!("Resource not found: {}", uri))
        }
    }
}
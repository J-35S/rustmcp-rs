use serde::Deserialize;

/// 应用设置
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    #[allow(dead_code)]
    pub host: String,
    #[allow(dead_code)]
    pub port: u16,
    #[allow(dead_code)]
    pub debug: bool,
    #[serde(default = "default_resource_prefix_format")]
    #[allow(dead_code)]
    pub resource_prefix_format: String,
}

impl Settings {
    /// 创建新的设置实例
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8000,
            debug: false,
            resource_prefix_format: default_resource_prefix_format(),
        }
    }
    
    /// 获取调试模式设置
    #[allow(dead_code)]
    pub fn debug(&self) -> bool {
        self.debug
    }
    
    /// 获取资源前缀格式
    #[allow(dead_code)]
    pub fn resource_prefix_format(&self) -> &str {
        &self.resource_prefix_format
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

fn default_resource_prefix_format() -> String {
    "resource://".to_string()
}
use thiserror::Error;
use std::path::Path;

/// 自定义错误类型
#[derive(Error, Debug)]
pub enum EspError {
    #[error("Invalid file format")]
    InvalidFormat,
    
    #[error("Unsupported record type: {0}")]
    UnsupportedRecordType(String),
    
    #[error("Compression error: {0}")]
    CompressionError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// 字符串验证配置
struct StringValidationConfig {
    blacklist: &'static [&'static str],
    whitelist: &'static [&'static str],
}

impl StringValidationConfig {
    const fn new() -> Self {
        Self {
            blacklist: &["<p>"],
            whitelist: &["Orcax"],
        }
    }
}

/// 字符串验证函数
pub fn is_valid_string(text: &str) -> bool {
    let text = text.trim();
    
    if text.is_empty() {
        return false;
    }
    
    let config = StringValidationConfig::new();
    
    // 黑名单检查
    if config.blacklist.contains(&text) {
        return false;
    }
    
    // 白名单检查
    if is_whitelisted(text, &config) {
        return true;
    }
    
    // 检查是否为变量名格式
    if is_variable_name(text) {
        return false;
    }
    
    // 检查字符有效性
    text.chars().all(|c| !c.is_control() || c.is_whitespace())
}

/// 检查是否在白名单中
fn is_whitelisted(text: &str, config: &StringValidationConfig) -> bool {
    config.whitelist.iter().any(|&w| text.contains(w)) || text.contains("<Alias")
}

/// 检查是否为变量名格式（驼峰或下划线）
fn is_variable_name(text: &str) -> bool {
    is_camel_case(text) || is_snake_case(text)
}

/// 检查是否为驼峰命名
fn is_camel_case(text: &str) -> bool {
    if text.len() < 3 || !text.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    
    let has_uppercase = text.chars().skip(2).any(|c| c.is_ascii_uppercase());
    let not_all_uppercase = !text.chars().all(|c| c.is_ascii_uppercase());
    
    has_uppercase && not_all_uppercase
}

/// 检查是否为下划线命名
fn is_snake_case(text: &str) -> bool {
    !text.contains(' ') && text.contains('_')
}

/// 创建文件备份
pub fn create_backup(file_path: &Path) -> Result<std::path::PathBuf, EspError> {
    if !file_path.exists() {
        return Err(EspError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "原文件不存在"
        )));
    }
    
    let timestamp = chrono::Local::now().format("%Y-%m-%d-%H-%M-%S");
    let backup_path = file_path.with_extension(format!("{}.bak", timestamp));
    
    std::fs::copy(file_path, &backup_path)
        .map_err(EspError::IoError)?;
    
    Ok(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_string_validation() {
        // 有效字符串
        assert!(is_valid_string("Iron Sword"));
        assert!(is_valid_string("This is a valid description."));
        assert!(is_valid_string("铁剑"));
        assert!(is_valid_string("这是一个有效的描述。"));
        assert!(is_valid_string("Mixed 中英文 text"));
        
        // 无效字符串
        assert!(!is_valid_string("CamelCaseVariable"));
        assert!(!is_valid_string("snake_case_var"));
        assert!(!is_valid_string(""));
        assert!(!is_valid_string("<p>"));
    }
    
    #[test]
    fn test_camel_case() {
        assert!(is_camel_case("CamelCase"));
        assert!(is_camel_case("myVariable"));
        assert!(!is_camel_case("lowercase"));
        assert!(!is_camel_case("UPPERCASE"));
        assert!(!is_camel_case("my"));
    }
    
    #[test]
    fn test_snake_case() {
        assert!(is_snake_case("snake_case"));
        assert!(is_snake_case("my_variable"));
        assert!(!is_snake_case("normal text"));
        assert!(!is_snake_case("CamelCase"));
    }
} 
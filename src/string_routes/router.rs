use std::collections::HashMap;

/// 字符串路由器 trait
///
/// 负责判断哪些记录类型和子记录类型包含可提取的字符串
pub trait StringRouter: Send + Sync + std::fmt::Debug {
    /// 获取某个记录类型支持的所有字符串子记录类型
    ///
    /// # 参数
    /// - `record_type`: 记录类型（如 "WEAP", "ARMO" 等）
    ///
    /// # 返回
    /// - `Some(&[String])`: 该记录类型支持的子记录类型列表
    /// - `None`: 该记录类型不包含字符串或不支持
    fn get_string_subrecord_types(&self, record_type: &str) -> Option<&[String]>;

    /// 检查某个记录类型的子记录类型是否支持字符串
    ///
    /// # 参数
    /// - `record_type`: 记录类型（如 "WEAP"）
    /// - `subrecord_type`: 子记录类型（如 "FULL", "DESC"）
    ///
    /// # 返回
    /// - `true`: 该组合支持字符串
    /// - `false`: 该组合不支持字符串
    fn supports_strings(&self, record_type: &str, subrecord_type: &str) -> bool {
        self.get_string_subrecord_types(record_type)
            .map(|types| types.iter().any(|t| t == subrecord_type))
            .unwrap_or(false)
    }
}

/// 默认字符串路由器实现
///
/// 使用 string_records.json 中的数据提供路由功能
#[derive(Debug)]
pub struct DefaultStringRouter {
    routes: HashMap<String, Vec<String>>,
}

impl DefaultStringRouter {
    /// 创建新的默认路由器实例
    ///
    /// # 参数
    /// - `routes`: 记录类型到子记录类型列表的映射
    pub fn new(routes: HashMap<String, Vec<String>>) -> Self {
        Self { routes }
    }

    /// 从内置的 string_records.json 创建默认路由器
    ///
    /// # 错误
    /// 如果 JSON 解析失败，返回错误
    pub fn from_embedded_data() -> Result<Self, Box<dyn std::error::Error>> {
        let routes = super::load_string_records()?;
        Ok(Self::new(routes))
    }
}

impl StringRouter for DefaultStringRouter {
    fn get_string_subrecord_types(&self, record_type: &str) -> Option<&[String]> {
        self.routes.get(record_type).map(|v| v.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_router() -> DefaultStringRouter {
        let mut routes = HashMap::new();
        routes.insert("WEAP".to_string(), vec!["FULL".to_string(), "DESC".to_string()]);
        routes.insert("ARMO".to_string(), vec!["FULL".to_string(), "DESC".to_string()]);
        routes.insert("NPC_".to_string(), vec!["FULL".to_string(), "SHRT".to_string()]);
        DefaultStringRouter::new(routes)
    }

    #[test]
    fn test_get_string_subrecord_types() {
        let router = create_test_router();

        // 已知类型
        let weap_types = router.get_string_subrecord_types("WEAP");
        assert!(weap_types.is_some());
        assert_eq!(weap_types.unwrap(), &["FULL", "DESC"]);

        // 未知类型
        let unknown = router.get_string_subrecord_types("UNKN");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_supports_strings() {
        let router = create_test_router();

        // 支持的组合
        assert!(router.supports_strings("WEAP", "FULL"));
        assert!(router.supports_strings("WEAP", "DESC"));
        assert!(router.supports_strings("NPC_", "SHRT"));

        // 不支持的组合
        assert!(!router.supports_strings("WEAP", "XXXX"));
        assert!(!router.supports_strings("UNKN", "FULL"));
    }

    #[test]
    fn test_from_embedded_data() {
        let router = DefaultStringRouter::from_embedded_data();
        assert!(router.is_ok());

        let router = router.unwrap();
        // 验证一些已知的记录类型
        assert!(router.supports_strings("WEAP", "FULL"));
        assert!(router.supports_strings("BOOK", "CNAM"));
        assert!(router.supports_strings("QUST", "NNAM"));
    }
}

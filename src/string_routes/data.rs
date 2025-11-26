use std::collections::HashMap;

/// 加载字符串记录定义
///
/// 从内置的 string_records.json 文件加载记录类型到字符串子记录类型的映射
///
/// # 返回
/// - `Ok(HashMap)`: 成功加载的映射数据
/// - `Err`: JSON 解析失败
///
/// # 示例
/// ```no_run
/// use esp_extractor::string_routes::load_string_records;
///
/// let records = load_string_records().unwrap();
/// let weap_types = records.get("WEAP");
/// assert!(weap_types.is_some());
/// ```
pub(crate) fn load_string_records() -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    let json_data = include_str!("../../data/string_records.json");
    Ok(serde_json::from_str(json_data)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_string_records() {
        let records = load_string_records();
        assert!(records.is_ok());

        let records = records.unwrap();

        // 验证一些已知的记录类型
        assert!(records.contains_key("WEAP"));
        assert!(records.contains_key("ARMO"));
        assert!(records.contains_key("BOOK"));

        // 验证 WEAP 的子记录类型
        let weap = records.get("WEAP").unwrap();
        assert_eq!(weap, &vec!["FULL".to_string(), "DESC".to_string()]);
    }

    #[test]
    fn test_json_format() {
        let records = load_string_records().unwrap();

        // 确保所有值都是非空数组
        for (key, value) in records.iter() {
            assert!(!value.is_empty(), "Record type {} has empty subrecord list", key);
        }
    }
}

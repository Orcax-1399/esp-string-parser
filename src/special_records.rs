// 特殊记录索引跟踪
// 根据 Python 版本的 mapping 文档实现特殊记录处理逻辑

use crate::record::Record;

/// 特殊记录处理器
///
/// 某些记录类型（INFO、PERK、QUST）需要特殊的索引跟踪逻辑，
/// 因为它们的子记录顺序和索引计算与普通记录不同。
pub struct SpecialRecordHandler;

impl SpecialRecordHandler {
    /// 判断记录类型是否需要特殊处理
    pub fn requires_special_handling(record_type: &str) -> bool {
        matches!(record_type, "INFO" | "PERK" | "QUST")
    }

    /// 获取特殊记录的子记录索引映射
    ///
    /// 返回：Vec<(subrecord_type, index)>
    /// - subrecord_type: 子记录类型（如 "NAM1", "RNAM", "CNAM"）
    /// - index: 该子记录应使用的索引
    pub fn get_special_indices(record: &Record) -> Vec<(String, i32)> {
        match record.record_type.as_str() {
            "INFO" => Self::handle_info_record(record),
            "PERK" => Self::handle_perk_record(record),
            "QUST" => Self::handle_qust_record(record),
            _ => Vec::new(),
        }
    }

    /// 处理 INFO 记录（对话信息）
    ///
    /// INFO 记录的 NAM1 和 RNAM 子记录需要追踪响应索引。
    /// 每个响应按照出现顺序递增索引。
    ///
    /// 参考 Python 版本：record.py 中的 INFO 特殊处理
    fn handle_info_record(record: &Record) -> Vec<(String, i32)> {
        let mut indices = Vec::new();
        let mut response_index = 0i32;

        for subrecord in &record.subrecords {
            match subrecord.record_type.as_str() {
                "NAM1" | "RNAM" => {
                    // 这些子记录使用响应索引
                    indices.push((subrecord.record_type.clone(), response_index));
                    response_index += 1;
                }
                _ => {
                    // 其他子记录不需要特殊索引
                }
            }
        }

        indices
    }

    /// 处理 PERK 记录（技能）
    ///
    /// PERK 记录根据不同的 perk 类型有不同的索引规则。
    ///
    /// 参考 Python 版本：record.py 中的 PERK 特殊处理
    ///
    /// 注意：当前实现为简化版本，仅处理基本情况。
    /// 完整实现需要解析 perk 数据结构来确定具体类型。
    fn handle_perk_record(record: &Record) -> Vec<(String, i32)> {
        let mut indices = Vec::new();
        let mut perk_index = 0i32;

        // PERK 记录中的字符串子记录
        // 根据 mapping 文档，可能包含 EPFT (Entry Point Function Type) 相关的字符串
        for subrecord in &record.subrecords {
            if matches!(subrecord.record_type.as_str(), "EPF2" | "EPF3") {
                indices.push((subrecord.record_type.clone(), perk_index));
                perk_index += 1;
            }
        }

        indices
    }

    /// 处理 QUST 记录（任务）
    ///
    /// QUST 记录需要计算条件索引（CNAM 子记录）。
    /// 每个 CNAM 按照出现顺序递增索引。
    ///
    /// 参考 Python 版本：record.py 中的 QUST 特殊处理
    fn handle_qust_record(record: &Record) -> Vec<(String, i32)> {
        let mut indices = Vec::new();
        let mut condition_index = 0i32;

        for subrecord in &record.subrecords {
            if subrecord.record_type == "CNAM" {
                // CNAM 子记录使用条件索引
                indices.push((subrecord.record_type.clone(), condition_index));
                condition_index += 1;
            }
        }

        indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subrecord::Subrecord;

    #[test]
    fn test_requires_special_handling() {
        assert!(SpecialRecordHandler::requires_special_handling("INFO"));
        assert!(SpecialRecordHandler::requires_special_handling("PERK"));
        assert!(SpecialRecordHandler::requires_special_handling("QUST"));
        assert!(!SpecialRecordHandler::requires_special_handling("WEAP"));
        assert!(!SpecialRecordHandler::requires_special_handling("ARMO"));
    }

    #[test]
    fn test_info_record_indices() {
        // 创建模拟的 INFO 记录
        let mut record = Record {
            record_type_bytes: *b"INFO",
            record_type: "INFO".to_string(),
            data_size: 0,
            flags: 0,
            form_id: 0,
            timestamp: 0,
            version_control_info: 0,
            internal_version: 0,
            unknown: 0,
            original_compressed_data: None,
            raw_data: Vec::new(),
            subrecords: Vec::new(),
            is_modified: false,
        };

        // 添加 NAM1 和 RNAM 子记录
        record.subrecords.push(Subrecord {
            record_type_bytes: *b"NAM1",
            record_type: "NAM1".to_string(),
            size: 4,
            data: vec![0; 4],
        });

        record.subrecords.push(Subrecord {
            record_type_bytes: *b"RNAM",
            record_type: "RNAM".to_string(),
            size: 4,
            data: vec![0; 4],
        });

        record.subrecords.push(Subrecord {
            record_type_bytes: *b"NAM1",
            record_type: "NAM1".to_string(),
            size: 4,
            data: vec![0; 4],
        });

        let indices = SpecialRecordHandler::get_special_indices(&record);

        assert_eq!(indices.len(), 3);
        assert_eq!(indices[0], ("NAM1".to_string(), 0));
        assert_eq!(indices[1], ("RNAM".to_string(), 1));
        assert_eq!(indices[2], ("NAM1".to_string(), 2));
    }

    #[test]
    fn test_qust_record_indices() {
        // 创建模拟的 QUST 记录
        let mut record = Record {
            record_type_bytes: *b"QUST",
            record_type: "QUST".to_string(),
            data_size: 0,
            flags: 0,
            form_id: 0,
            timestamp: 0,
            version_control_info: 0,
            internal_version: 0,
            unknown: 0,
            original_compressed_data: None,
            raw_data: Vec::new(),
            subrecords: Vec::new(),
            is_modified: false,
        };

        // 添加多个 CNAM 子记录
        for _ in 0..3 {
            record.subrecords.push(Subrecord {
                record_type_bytes: *b"CNAM",
                record_type: "CNAM".to_string(),
                size: 4,
                data: vec![0; 4],
            });
        }

        let indices = SpecialRecordHandler::get_special_indices(&record);

        assert_eq!(indices.len(), 3);
        assert_eq!(indices[0], ("CNAM".to_string(), 0));
        assert_eq!(indices[1], ("CNAM".to_string(), 1));
        assert_eq!(indices[2], ("CNAM".to_string(), 2));
    }
}

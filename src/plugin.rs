mod parser;
mod strings;
mod translate;
mod writer;
mod stats;
mod esl;

pub use stats::PluginStats;

use crate::group::Group;
use crate::record::Record;
use crate::string_file::{StringFileSet, StringFileType};
use crate::string_routes::StringRouter;
use memmap2::Mmap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// ESP插件解析器
#[derive(Debug)]
pub struct Plugin {
    /// 文件路径
    pub path: PathBuf,
    /// 头部记录
    pub header: Record,
    /// 组列表
    pub groups: Vec<Group>,
    /// 主文件列表
    pub masters: Vec<String>,
    /// 字符串记录定义（已弃用，使用 string_router）
    ///
    /// **注意**：此字段保留用于向后兼容，新代码应使用 `string_router` 字段
    #[deprecated(since = "0.6.0", note = "Use string_router instead")]
    pub string_records: HashMap<String, Vec<String>>,
    /// 字符串路由器（v0.6.0 新增 - P2.3）
    string_router: Arc<dyn StringRouter>,
    /// STRING文件集合（仅本地化插件有值）
    string_files: Option<StringFileSet>,
    /// 语言标识（用于STRING文件查找）
    /// 注意：此字段仅用于向后兼容 deprecated 的 `new()` 方法
    #[allow(dead_code)]
    language: String,
    /// 内存映射文件（性能优化：零拷贝访问文件数据）
    #[allow(dead_code)]
    mmap: Option<Arc<Mmap>>,
}

impl Plugin {
    /// 获取插件名称
    pub fn get_name(&self) -> &str {
        self.path.file_name().unwrap().to_str().unwrap()
    }

    /// 获取插件类型
    pub fn get_type(&self) -> &str {
        match self.path.extension().and_then(|ext| ext.to_str()) {
            Some("esp") => "插件 (ESP)",
            Some("esm") => "主文件 (ESM)",
            Some("esl") => "轻量级文件 (ESL)",
            _ => "未知",
        }
    }

    /// 是否为主文件
    pub fn is_master(&self) -> bool {
        self.path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase() == "esm")
            .unwrap_or(false)
    }

    /// 是否本地化
    pub fn is_localized(&self) -> bool {
        self.header.flags & 0x00000080 != 0
    }

    /// 获取字符串路由器引用
    ///
    /// 返回插件使用的字符串路由器，用于判断哪些记录类型和子记录类型包含字符串
    pub fn string_router(&self) -> &dyn StringRouter {
        self.string_router.as_ref()
    }

    /// 设置 STRING 文件集合（用于外部加载的 STRING 文件）
    ///
    /// 这个方法主要用于 LocalizedPluginContext 将加载的 STRING 文件
    /// 设置到 Plugin 对象中，以便 extract_strings() 等方法可以访问。
    pub fn set_string_files(&mut self, string_files: StringFileSet) {
        self.string_files = Some(string_files);
    }

    /// 是否为轻量插件 (Light Plugin/ESL)
    ///
    /// 检查插件是否为轻量插件，通过以下两种方式之一判断：
    /// 1. 文件扩展名为 .esl
    /// 2. 头部记录的 LightMaster 标志 (0x00000200) 被设置
    ///
    /// 根据 mapping 文档：Python 版本的 `is_light()` 方法
    pub fn is_light(&self) -> bool {
        // 方式1：检查文件扩展名
        if let Some(ext) = self.path.extension() {
            if ext.to_string_lossy().to_lowercase() == "esl" {
                return true;
            }
        }

        // 方式2：检查 LightMaster 标志 (0x00000200)
        const LIGHT_MASTER_FLAG: u32 = 0x00000200;
        (self.header.flags & LIGHT_MASTER_FLAG) != 0
    }

    /// 格式化FormID
    pub(crate) fn format_form_id(&self, form_id: u32) -> String {
        let master_index = (form_id >> 24) as usize;
        let master_file = if master_index < self.masters.len() {
            &self.masters[master_index]
        } else {
            self.path.file_name().unwrap().to_str().unwrap()
        };

        format!("{:08X}|{}", form_id, master_file)
    }

    /// 根据记录类型和子记录类型确定应该使用哪个STRING文件类型
    ///
    /// # 映射规则
    /// - INFO 记录 → ILSTRINGS（对话信息）
    /// - DESC/CNAM 子记录 → DLSTRINGS（描述文本/内容，通常是较长的文本）
    /// - 其他所有字符串子记录 (FULL/NNAM等) → STRINGS (默认)
    pub(crate) fn determine_string_file_type(
        record_type: &str,
        subrecord_type: &str,
    ) -> StringFileType {
        // INFO 记录 → ILSTRINGS
        // INFO 记录包含对话信息，按照 Bethesda 约定存储在 ILSTRINGS 中
        if record_type == "INFO" {
            return StringFileType::ILSTRINGS;
        }

        // DESC 和 CNAM 子记录 → DLSTRINGS
        // 这些通常是较长的描述性文本或内容，按照 Bethesda 约定存储在 DLSTRINGS 中
        if matches!(subrecord_type, "DESC" | "CNAM") {
            return StringFileType::DLSTRINGS;
        }

        // 默认 → STRINGS
        // 包括 FULL, NNAM, SHRT, DNAM 等常规名称和简短文本
        StringFileType::STRINGS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info_routes_to_ilstrings() {
        let file_type = Plugin::determine_string_file_type("INFO", "NAM1");
        assert_eq!(
            file_type,
            StringFileType::ILSTRINGS,
            "INFO记录应该路由到ILSTRINGS（对话信息）"
        );
    }

    #[test]
    fn test_desc_routes_to_dlstrings() {
        // 任何record的DESC都应该路由到DLSTRINGS
        let file_type = Plugin::determine_string_file_type("PERK", "DESC");
        assert_eq!(
            file_type,
            StringFileType::DLSTRINGS,
            "PERK DESC应该路由到DLSTRINGS"
        );

        let file_type = Plugin::determine_string_file_type("WEAP", "DESC");
        assert_eq!(
            file_type,
            StringFileType::DLSTRINGS,
            "WEAP DESC应该路由到DLSTRINGS"
        );

        let file_type = Plugin::determine_string_file_type("MESG", "DESC");
        assert_eq!(
            file_type,
            StringFileType::DLSTRINGS,
            "MESG DESC应该路由到DLSTRINGS"
        );
    }

    #[test]
    fn test_cnam_routes_to_dlstrings() {
        // 任何record的CNAM都应该路由到DLSTRINGS
        let file_type = Plugin::determine_string_file_type("QUST", "CNAM");
        assert_eq!(
            file_type,
            StringFileType::DLSTRINGS,
            "QUST CNAM应该路由到DLSTRINGS"
        );

        let file_type = Plugin::determine_string_file_type("BOOK", "CNAM");
        assert_eq!(
            file_type,
            StringFileType::DLSTRINGS,
            "BOOK CNAM应该路由到DLSTRINGS"
        );
    }

    #[test]
    fn test_full_routes_to_strings() {
        // FULL应该路由到STRINGS
        let file_type = Plugin::determine_string_file_type("WEAP", "FULL");
        assert_eq!(
            file_type,
            StringFileType::STRINGS,
            "WEAP FULL应该路由到STRINGS"
        );

        let file_type = Plugin::determine_string_file_type("PERK", "FULL");
        assert_eq!(
            file_type,
            StringFileType::STRINGS,
            "PERK FULL应该路由到STRINGS"
        );

        let file_type = Plugin::determine_string_file_type("DIAL", "FULL");
        assert_eq!(
            file_type,
            StringFileType::STRINGS,
            "DIAL FULL应该路由到STRINGS"
        );
    }
}


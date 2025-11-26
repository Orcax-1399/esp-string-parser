use super::Plugin;
use crate::group::{Group, GroupChild};

/// 插件统计信息
pub struct PluginStats {
    pub name: String,
    pub plugin_type: String,
    pub is_master: bool,
    pub is_localized: bool,
    pub master_count: usize,
    pub group_count: usize,
    pub record_count: usize,
    pub string_count: usize,
}

impl std::fmt::Display for PluginStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== 插件统计信息 ===")?;
        writeln!(f, "名称: {}", self.name)?;
        writeln!(f, "类型: {}", self.plugin_type)?;
        writeln!(f, "主文件: {}", if self.is_master { "是" } else { "否" })?;
        writeln!(f, "本地化: {}", if self.is_localized { "是" } else { "否" })?;
        writeln!(f, "依赖主文件数: {}", self.master_count)?;
        writeln!(f, "组数量: {}", self.group_count)?;
        writeln!(f, "记录数量: {}", self.record_count)?;
        writeln!(f, "可翻译字符串数: {}", self.string_count)?;
        Ok(())
    }
}

impl Plugin {
    /// 获取统计信息
    pub fn get_stats(&self) -> PluginStats {
        let strings = self.extract_strings();

        PluginStats {
            name: self.get_name().to_string(),
            plugin_type: self.get_type().to_string(),
            is_master: self.is_master(),
            is_localized: self.is_localized(),
            master_count: self.masters.len(),
            group_count: self.count_total_groups(),
            record_count: self.count_records(),
            string_count: strings.len(),
        }
    }

    /// 统计记录数量
    fn count_records(&self) -> usize {
        1 + self.groups.iter().map(|g| self.count_group_records(g)).sum::<usize>()
    }

    /// 统计组中的记录数量
    #[allow(clippy::only_used_in_recursion)]
    fn count_group_records(&self, group: &Group) -> usize {
        group.children.iter().map(|child| match child {
            GroupChild::Group(subgroup) => self.count_group_records(subgroup),
            GroupChild::Record(_) => 1,
        }).sum()
    }

    /// 统计总组数
    fn count_total_groups(&self) -> usize {
        self.groups.len() + self.groups.iter().map(|g| self.count_subgroups(g)).sum::<usize>()
    }

    /// 统计子组数量
    #[allow(clippy::only_used_in_recursion)]
    fn count_subgroups(&self, group: &Group) -> usize {
        group.children.iter().map(|child| match child {
            GroupChild::Group(subgroup) => 1 + self.count_subgroups(subgroup),
            GroupChild::Record(_) => 0,
        }).sum()
    }
}

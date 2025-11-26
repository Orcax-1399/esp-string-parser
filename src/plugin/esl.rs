use super::Plugin;
use crate::record::Record;
use crate::group::{Group, GroupChild};

impl Plugin {
    /// 重编号 FormID 以符合 ESL (Light Plugin) 规范
    ///
    /// 将插件中所有记录的 FormID 重新编号，从 0x800 开始，适用于轻量插件。
    /// 仅修改属于当前插件的记录（非来自外部主文件的记录）。
    ///
    /// # ESL 限制
    /// - 最多支持 2048 (0x800) 个记录
    /// - FormID 的低12位 (0x000-0xFFF) 用于记录编号
    ///
    /// # 错误
    /// - 如果记录数超过 2048 个，返回错误
    ///
    /// # 参考
    /// 根据 mapping 文档的 Python 版本 `eslify_formids()` 方法实现
    pub fn eslify_formids(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // 提取所有记录的可变引用
        let mut all_records = Vec::new();
        for group in &mut self.groups {
            Self::extract_group_records_mut(group, &mut all_records);
        }

        // 从 0x800 开始编号
        let mut current_formid = 0x800u32;

        for record in all_records {
            // 获取主文件索引（FormID 高字节）
            let master_index = (record.form_id >> 24) as usize;

            // 仅修改属于当前插件的记录（非外部主文件）
            if master_index >= self.masters.len() {
                // 保留高20位，替换低12位
                let high_bits = record.form_id & 0xFFFFF000;
                let new_formid = high_bits | (current_formid & 0xFFF);

                record.form_id = new_formid;
                record.is_modified = true;

                current_formid += 1;

                // ESL 限制：最多 2048 (0x800) 个记录
                if current_formid > 0xFFF {
                    return Err(format!(
                        "ESL 插件记录数超过限制！最多支持 2048 个记录，当前已处理 {} 个",
                        current_formid - 0x800
                    ).into());
                }
            }
        }

        #[cfg(debug_assertions)]
        println!("ESL FormID 重编号完成：共 {} 个记录", current_formid - 0x800);

        Ok(())
    }

    /// 递归提取组中所有记录的可变引用
    fn extract_group_records_mut<'a>(
        group: &'a mut Group,
        records: &mut Vec<&'a mut Record>
    ) {
        for child in &mut group.children {
            match child {
                GroupChild::Record(record) => records.push(record),
                GroupChild::Group(nested_group) => {
                    Self::extract_group_records_mut(nested_group, records);
                }
            }
        }
    }
}

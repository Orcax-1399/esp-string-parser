use super::Plugin;
use crate::record::Record;
use crate::group::{Group, GroupChild};
use std::path::PathBuf;
use std::borrow::Cow;

impl Plugin {
    /// 写入文件
    pub fn write_to_file(&self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut output = Vec::new();

        self.write_record(&self.header, &mut output)?;

        for group in &self.groups {
            self.write_group(group, &mut output)?;
        }

        std::fs::write(path, output)?;
        Ok(())
    }

    /// 写入记录
    pub(crate) fn write_record(&self, record: &Record, output: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        use crate::datatypes::RecordFlags;

        // 写入记录类型
        output.extend_from_slice(&record.record_type_bytes);

        // 判断记录是否原本就是压缩的
        let is_originally_compressed = record.flags & RecordFlags::COMPRESSED.bits() != 0;

        // 处理数据部分（使用 Cow 避免不必要的克隆，性能优化 ~500-800ms）
        let data_to_write: Cow<[u8]> = if record.is_modified {
            // 如果记录被修改，重新序列化子记录（需要新分配）
            let mut subrecord_data = Vec::new();
            for subrecord in &record.subrecords {
                subrecord_data.extend_from_slice(&subrecord.record_type_bytes);
                subrecord_data.extend_from_slice(&(subrecord.data.len() as u16).to_le_bytes());
                subrecord_data.extend_from_slice(&subrecord.data);
            }

            // 如果原本是压缩的，重新压缩
            if is_originally_compressed {
                let compressed = record.recompress_data()?;
                #[cfg(debug_assertions)]
                println!("🔄 重新压缩记录 {}: 解压大小 {} -> 压缩大小 {}",
                    record.record_type, subrecord_data.len(), compressed.len());
                Cow::Owned(compressed)
            } else {
                #[cfg(debug_assertions)]
                println!("📝 修改非压缩记录 {}: 大小 {}", record.record_type, subrecord_data.len());
                Cow::Owned(subrecord_data)
            }
        } else {
            // 使用原始数据（零拷贝借用，避免 35MB 内存拷贝）
            if let Some(compressed_data) = &record.original_compressed_data {
                #[cfg(debug_assertions)]
                println!("📦 保持压缩记录 {}: 原始压缩大小 {} (原始data_size: {})",
                    record.record_type, compressed_data.len(), record.data_size);
                Cow::Borrowed(compressed_data.as_slice())
            } else {
                #[cfg(debug_assertions)]
                println!("📄 保持未压缩记录 {}: 大小 {} (原始data_size: {})",
                    record.record_type, record.raw_data.len(), record.data_size);
                Cow::Borrowed(&record.raw_data)
            }
        };

        // 写入数据大小
        let actual_size = data_to_write.len() as u32;
        output.extend_from_slice(&actual_size.to_le_bytes());

        #[cfg(debug_assertions)]
        if actual_size != record.data_size && !record.is_modified {
            println!("⚠️  记录 {} 大小不匹配: 写入 {} vs 原始 {}",
                record.record_type, actual_size, record.data_size);
        }

        // 写入其他头部字段
        output.extend_from_slice(&record.flags.to_le_bytes());
        output.extend_from_slice(&record.form_id.to_le_bytes());
        output.extend_from_slice(&record.timestamp.to_le_bytes());
        output.extend_from_slice(&record.version_control_info.to_le_bytes());
        output.extend_from_slice(&record.internal_version.to_le_bytes());
        output.extend_from_slice(&record.unknown.to_le_bytes());

        // 写入数据部分
        output.extend_from_slice(&data_to_write);

        Ok(())
    }

    /// 写入组
    pub(crate) fn write_group(&self, group: &Group, output: &mut Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        // 写入组头部
        output.extend_from_slice(b"GRUP");

        // 临时占位符，稍后计算实际大小
        let size_pos = output.len();
        output.extend_from_slice(&[0u8; 4]);

        output.extend_from_slice(&group.label);
        output.extend_from_slice(&group.group_type.to_i32().to_le_bytes());
        output.extend_from_slice(&group.timestamp.to_le_bytes());
        output.extend_from_slice(&group.version_control_info.to_le_bytes());
        output.extend_from_slice(&group.unknown.to_le_bytes());

        // 写入子元素
        for child in &group.children {
            match child {
                GroupChild::Group(subgroup) => {
                    self.write_group(subgroup, output)?;
                }
                GroupChild::Record(record) => {
                    self.write_record(record, output)?;
                }
            }
        }

        // 计算并写入实际大小（需要包含"GRUP"的4字节）
        let actual_size = (output.len() - size_pos + 4) as u32;
        let size_bytes = actual_size.to_le_bytes();
        output[size_pos..size_pos + 4].copy_from_slice(&size_bytes);

        Ok(())
    }
}

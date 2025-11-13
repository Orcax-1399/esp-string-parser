use crate::plugin::Plugin;
use crate::record::Record;
use crate::group::{Group, GroupChild};
use crate::subrecord::Subrecord;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;

/// ESP文件结构调试器
pub struct EspDebugger;

impl EspDebugger {
    /// 生成详细的文件结构dump
    pub fn dump_file_structure(plugin: &Plugin, output_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut output = File::create(output_path)?;
        
        Self::write_header_info(&mut output, plugin)?;
        Self::write_masters_info(&mut output, plugin)?;
        Self::write_groups_info(&mut output, plugin)?;
        
        Ok(())
    }
    
    /// 写入头部信息
    fn write_header_info(output: &mut File, plugin: &Plugin) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(output, "=== ESP文件结构dump ===")?;
        writeln!(output, "文件: {}", plugin.get_name())?;
        writeln!(output, "类型: {}", plugin.get_type())?;
        writeln!(output, "主文件: {}", if plugin.is_master() { "是" } else { "否" })?;
        writeln!(output, "本地化: {}", if plugin.is_localized() { "是" } else { "否" })?;
        writeln!(output)?;
        
        writeln!(output, "=== 头部记录 ===")?;
        Self::dump_record(&plugin.header, output, 0)?;
        writeln!(output)?;
        
        Ok(())
    }
    
    /// 写入主文件信息
    fn write_masters_info(output: &mut File, plugin: &Plugin) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(output, "=== 主文件列表 ({}) ===", plugin.masters.len())?;
        for (i, master) in plugin.masters.iter().enumerate() {
            writeln!(output, "  {}: {}", i, master)?;
        }
        writeln!(output)?;
        Ok(())
    }
    
    /// 写入组信息
    fn write_groups_info(output: &mut File, plugin: &Plugin) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(output, "=== 组结构 ({}) ===", plugin.groups.len())?;
        for (i, group) in plugin.groups.iter().enumerate() {
            writeln!(output, "组 {}:", i)?;
            Self::dump_group(group, output, 0)?;
            writeln!(output)?;
        }
        Ok(())
    }
    
    /// Dump 组结构
    fn dump_group(group: &Group, output: &mut File, indent: usize) -> Result<(), Box<dyn std::error::Error>> {
        let prefix = "  ".repeat(indent);
        
        writeln!(output, "{}GRUP {{", prefix)?;
        writeln!(output, "{}  大小: {} bytes", prefix, group.size)?;
        writeln!(output, "{}  标签: {:?} ('{}')", prefix, group.label, String::from_utf8_lossy(&group.label))?;
        writeln!(output, "{}  类型: {:?} ({})", prefix, group.group_type, group.group_type.to_i32())?;
        writeln!(output, "{}  时间戳: {}", prefix, group.timestamp)?;
        writeln!(output, "{}  版本控制: {}", prefix, group.version_control_info)?;
        writeln!(output, "{}  未知字段: {}", prefix, group.unknown)?;
        writeln!(output, "{}  子元素数: {}", prefix, group.children.len())?;
        
        for (i, child) in group.children.iter().enumerate() {
            writeln!(output, "{}  子元素 {}:", prefix, i)?;
            match child {
                GroupChild::Group(subgroup) => {
                    Self::dump_group(subgroup, output, indent + 2)?;
                }
                GroupChild::Record(record) => {
                    Self::dump_record(record, output, indent + 2)?;
                }
            }
        }
        
        writeln!(output, "{}}}", prefix)?;
        Ok(())
    }
    
    /// Dump 记录结构
    fn dump_record(record: &Record, output: &mut File, indent: usize) -> Result<(), Box<dyn std::error::Error>> {
        let prefix = "  ".repeat(indent);
        
        writeln!(output, "{}{} {{", prefix, record.record_type)?;
        writeln!(output, "{}  原始类型字节: {:?}", prefix, record.record_type_bytes)?;
        writeln!(output, "{}  数据大小: {} bytes", prefix, record.data_size)?;
        writeln!(output, "{}  标志位: 0x{:08X}", prefix, record.flags)?;
        writeln!(output, "{}  FormID: 0x{:08X}", prefix, record.form_id)?;
        writeln!(output, "{}  时间戳: {}", prefix, record.timestamp)?;
        writeln!(output, "{}  版本控制: {}", prefix, record.version_control_info)?;
        writeln!(output, "{}  内部版本: {}", prefix, record.internal_version)?;
        writeln!(output, "{}  未知字段: {}", prefix, record.unknown)?;
        writeln!(output, "{}  子记录数: {}", prefix, record.subrecords.len())?;
        
        let total_subrecord_size = Self::calculate_subrecord_size(&record.subrecords);
        writeln!(output, "{}  计算的子记录总大小: {} bytes", prefix, total_subrecord_size)?;
        
        if total_subrecord_size != record.data_size as usize {
            writeln!(output, "{}  ⚠ 大小不匹配！差异: {} bytes", prefix, 
                (total_subrecord_size as i32) - (record.data_size as i32))?;
        }
        
        for (i, subrecord) in record.subrecords.iter().enumerate() {
            writeln!(output, "{}  子记录 {}:", prefix, i)?;
            Self::dump_subrecord(subrecord, output, indent + 2)?;
        }
        
        writeln!(output, "{}}}", prefix)?;
        Ok(())
    }
    
    /// 计算子记录总大小
    fn calculate_subrecord_size(subrecords: &[Subrecord]) -> usize {
        subrecords.iter()
            .map(|sr| 6 + sr.data.len()) // 6字节头部 + 数据
            .sum()
    }
    
    /// Dump 子记录结构
    fn dump_subrecord(subrecord: &Subrecord, output: &mut File, indent: usize) -> Result<(), Box<dyn std::error::Error>> {
        let prefix = "  ".repeat(indent);
        
        writeln!(output, "{}{} {{", prefix, subrecord.record_type)?;
        writeln!(output, "{}  原始类型字节: {:?}", prefix, subrecord.record_type_bytes)?;
        writeln!(output, "{}  大小: {} bytes", prefix, subrecord.size)?;
        writeln!(output, "{}  实际数据长度: {} bytes", prefix, subrecord.data.len())?;
        
        if subrecord.data.len() != subrecord.size as usize {
            writeln!(output, "{}  ⚠ 大小不匹配！差异: {} bytes", prefix, 
                (subrecord.data.len() as i32) - (subrecord.size as i32))?;
        }
        
        Self::dump_subrecord_data(output, &prefix, subrecord)?;
        writeln!(output, "{}}}", prefix)?;
        Ok(())
    }
    
    /// Dump子记录数据
    fn dump_subrecord_data(output: &mut File, prefix: &str, subrecord: &Subrecord) -> Result<(), Box<dyn std::error::Error>> {
        if !subrecord.data.is_empty() {
            let preview_len = std::cmp::min(32, subrecord.data.len());
            let hex_data: Vec<String> = subrecord.data[..preview_len]
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect();
            writeln!(output, "{}  数据预览: {}{}", prefix, 
                hex_data.join(" "), 
                if subrecord.data.len() > 32 { "..." } else { "" })?;
                
            if Self::is_likely_string_subrecord(&subrecord.record_type) {
                let text_content = String::from_utf8_lossy(&subrecord.data);
                let clean_text = text_content.trim_end_matches('\0');
                if !clean_text.is_empty() {
                    writeln!(output, "{}  文本内容: \"{}\"", prefix, clean_text)?;
                }
            }
        }
        Ok(())
    }
    
    /// 判断是否可能是字符串类型的子记录
    fn is_likely_string_subrecord(record_type: &str) -> bool {
        matches!(record_type, "EDID" | "FULL" | "DESC" | "FNAM" | "DNAM" | "NNAM" | 
                              "CNAM" | "NAM1" | "RNAM" | "ITXT" | "SHRT")
    }
    
    /// 对比两个文件的结构
    pub fn compare_structures(
        original_path: PathBuf,
        rebuilt_path: PathBuf,
        output_path: PathBuf
    ) -> Result<(), Box<dyn std::error::Error>> {
        let original = Plugin::new(original_path.clone(), None)?;
        let rebuilt = Plugin::new(rebuilt_path.clone(), None)?;
        
        let mut output = File::create(output_path)?;
        
        Self::write_comparison_header(&mut output, &original_path, &rebuilt_path)?;
        Self::compare_basic_info(&mut output, &original, &rebuilt)?;
        Self::compare_header_records(&mut output, &original, &rebuilt)?;
        Self::compare_group_structures(&mut output, &original, &rebuilt)?;
        
        Ok(())
    }
    
    /// 写入对比头部
    fn write_comparison_header(output: &mut File, original_path: &Path, rebuilt_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(output, "=== ESP文件结构对比 ===")?;
        writeln!(output, "原始文件: {}", original_path.display())?;
        writeln!(output, "重建文件: {}", rebuilt_path.display())?;
        writeln!(output)?;
        Ok(())
    }
    
    /// 对比基本信息
    fn compare_basic_info(output: &mut File, original: &Plugin, rebuilt: &Plugin) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(output, "=== 基本信息对比 ===")?;
        writeln!(output, "组数量: {} vs {}", original.groups.len(), rebuilt.groups.len())?;
        writeln!(output, "主文件数: {} vs {}", original.masters.len(), rebuilt.masters.len())?;
        writeln!(output)?;
        Ok(())
    }
    
    /// 对比头部记录
    fn compare_header_records(output: &mut File, original: &Plugin, rebuilt: &Plugin) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(output, "=== 头部记录对比 ===")?;
        Self::compare_records(&original.header, &rebuilt.header, output, "头部")?;
        writeln!(output)?;
        Ok(())
    }
    
    /// 对比组结构
    fn compare_group_structures(output: &mut File, original: &Plugin, rebuilt: &Plugin) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(output, "=== 组结构对比 ===")?;
        let min_groups = std::cmp::min(original.groups.len(), rebuilt.groups.len());
        for i in 0..min_groups {
            writeln!(output, "组 {}:", i)?;
            Self::compare_groups(&original.groups[i], &rebuilt.groups[i], output)?;
        }
        
        if original.groups.len() != rebuilt.groups.len() {
            writeln!(output, "⚠ 组数量不匹配！")?;
        }
        
        Ok(())
    }
    
    /// 对比两个组
    fn compare_groups(original: &Group, rebuilt: &Group, output: &mut File) -> Result<(), Box<dyn std::error::Error>> {
        if original.size != rebuilt.size {
            writeln!(output, "  ⚠ 组大小不匹配: {} vs {}", original.size, rebuilt.size)?;
        }
        if original.label != rebuilt.label {
            writeln!(output, "  ⚠ 组标签不匹配: {:?} vs {:?}", original.label, rebuilt.label)?;
        }
        if original.children.len() != rebuilt.children.len() {
            writeln!(output, "  ⚠ 子元素数量不匹配: {} vs {}", original.children.len(), rebuilt.children.len())?;
        }
        Ok(())
    }
    
    /// 对比两个记录
    fn compare_records(original: &Record, rebuilt: &Record, output: &mut File, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if original.record_type != rebuilt.record_type {
            writeln!(output, "  ⚠ {} 记录类型不匹配: {} vs {}", name, original.record_type, rebuilt.record_type)?;
        }
        if original.data_size != rebuilt.data_size {
            writeln!(output, "  ⚠ {} 数据大小不匹配: {} vs {}", name, original.data_size, rebuilt.data_size)?;
        }
        if original.flags != rebuilt.flags {
            writeln!(output, "  ⚠ {} 标志位不匹配: 0x{:08X} vs 0x{:08X}", name, original.flags, rebuilt.flags)?;
        }
        if original.subrecords.len() != rebuilt.subrecords.len() {
            writeln!(output, "  ⚠ {} 子记录数量不匹配: {} vs {}", name, original.subrecords.len(), rebuilt.subrecords.len())?;
        }
        Ok(())
    }
} 
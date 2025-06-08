use crate::datatypes::{read_u16, read_u32, read_i32};
use crate::record::Record;
use std::io::{Read, Cursor};

/// 组类型
#[derive(Debug, Clone)]
pub enum GroupType {
    /// 普通组
    Normal,
    /// 世界组
    World,
    /// 单元格组
    Cell,
    /// 未知类型
    Unknown(i32),
}

impl GroupType {
    /// 转换为i32值
    pub fn to_i32(&self) -> i32 {
        match self {
            GroupType::Normal => 0,
            GroupType::World => 1,
            GroupType::Cell => 6,
            GroupType::Unknown(value) => *value,
        }
    }
}

impl From<i32> for GroupType {
    fn from(value: i32) -> Self {
        match value {
            0 => GroupType::Normal,
            1 => GroupType::World,
            6 => GroupType::Cell,
            _ => GroupType::Unknown(value),
        }
    }
}

/// 组结构
#[derive(Debug)]
pub struct Group {
    /// 组大小(包含头部24字节)
    pub size: u32,
    /// 标签
    pub label: [u8; 4],
    /// 组类型
    pub group_type: GroupType,
    /// 时间戳
    pub timestamp: u16,
    /// 版本控制信息
    pub version_control_info: u16,
    /// 未知字段
    pub unknown: u32,
    /// 子元素
    pub children: Vec<GroupChild>,
}

/// 组子元素
#[derive(Debug)]
pub enum GroupChild {
    /// 子组
    Group(Box<Group>),
    /// 记录
    Record(Record),
}

impl Group {
    /// 解析组
    pub fn parse(cursor: &mut Cursor<&[u8]>) -> Result<Self, Box<dyn std::error::Error>> {
        // 检查是否有足够的数据读取头部
        if cursor.position() + 24 > cursor.get_ref().len() as u64 {
            return Err("Insufficient data for group header".into());
        }
        
        // 读取组头部(24字节)
        let mut type_bytes = [0u8; 4];
        cursor.read_exact(&mut type_bytes)?;
        
        // 验证是否为组类型
        if &type_bytes != b"GRUP" {
            return Err(format!("Expected GRUP, found {}", String::from_utf8_lossy(&type_bytes)).into());
        }
        
        let size = read_u32(cursor)?;
        
        // 验证组大小是否合理
        if size > 200_000_000 {  // 200MB限制
            return Err(format!("组大小异常: {} bytes (可能数据损坏)", size).into());
        }
        
        if size < 24 {
            return Err(format!("组大小太小: {} bytes (最小应为24字节)", size).into());
        }
        
        let mut label = [0u8; 4];
        cursor.read_exact(&mut label)?;
        let group_type = GroupType::from(read_i32(cursor)?);
        let timestamp = read_u16(cursor)?;
        let version_control_info = read_u16(cursor)?;
        let unknown = read_u32(cursor)?;
        
        // 计算数据大小(不包含头部)
        let data_size = size - 24;
        
        // 检查是否有足够的数据
        if cursor.position() + data_size as u64 > cursor.get_ref().len() as u64 {
            return Err(format!("Insufficient data for group data: expected {} bytes", data_size).into());
        }
        
        // 记录数据开始位置
        let data_start = cursor.position();
        let data_end = data_start + data_size as u64;
        
        // 解析子元素
        let mut children = Vec::new();
        while cursor.position() < data_end {
            // 预读取4字节判断类型
            let peek_pos = cursor.position();
            let mut peek_bytes = [0u8; 4];
            cursor.read_exact(&mut peek_bytes)?;
            cursor.set_position(peek_pos); // 恢复位置
            
            if &peek_bytes == b"GRUP" {
                // 是子组
                let child_group = Group::parse(cursor)?;
                children.push(GroupChild::Group(Box::new(child_group)));
            } else {
                // 是记录
                let record = Record::parse(cursor)?;
                children.push(GroupChild::Record(record));
            }
        }
        
        Ok(Group {
            size,
            label,
            group_type,
            timestamp,
            version_control_info,
            unknown,
            children,
        })
    }
    
    /// 获取组标签
    pub fn get_label(&self) -> &[u8; 4] {
        &self.label
    }
    
    /// 获取组类型
    pub fn get_type(&self) -> &GroupType {
        &self.group_type
    }
    
    /// 获取所有记录
    pub fn get_records(&self) -> Vec<&Record> {
        let mut records = Vec::new();
        self.collect_records(&mut records);
        records
    }
    
    /// 递归收集所有记录
    fn collect_records<'a>(&'a self, records: &mut Vec<&'a Record>) {
        for child in &self.children {
            match child {
                GroupChild::Group(group) => {
                    group.collect_records(records);
                }
                GroupChild::Record(record) => {
                    records.push(record);
                }
            }
        }
    }
    
    /// 获取组标签字符串
    pub fn get_label_string(&self) -> String {
        String::from_utf8_lossy(&self.label).into_owned()
    }
} 
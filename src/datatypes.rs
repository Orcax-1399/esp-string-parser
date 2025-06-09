use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write, Cursor};
use encoding_rs;

// 基础整数类型读取函数
pub fn read_u8(cursor: &mut Cursor<&[u8]>) -> Result<u8, std::io::Error> {
    cursor.read_u8()
}

pub fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, std::io::Error> {
    cursor.read_u16::<LittleEndian>()
}

pub fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, std::io::Error> {
    cursor.read_u32::<LittleEndian>()
}

pub fn read_i32(cursor: &mut Cursor<&[u8]>) -> Result<i32, std::io::Error> {
    cursor.read_i32::<LittleEndian>()
}

// 基础整数类型写入函数
pub fn write_u8(writer: &mut dyn Write, value: u8) -> Result<(), std::io::Error> {
    writer.write_u8(value)
}

pub fn write_u16(writer: &mut dyn Write, value: u16) -> Result<(), std::io::Error> {
    writer.write_u16::<LittleEndian>(value)
}

pub fn write_u32(writer: &mut dyn Write, value: u32) -> Result<(), std::io::Error> {
    writer.write_u32::<LittleEndian>(value)
}

pub fn write_i32(writer: &mut dyn Write, value: i32) -> Result<(), std::io::Error> {
    writer.write_i32::<LittleEndian>(value)
}

// 支持的编码
const SUPPORTED_ENCODINGS: &[&str] = &["utf-8", "windows-1252", "windows-1250", "windows-1251"];

#[derive(Debug, Clone)]
pub struct RawString {
    pub content: String,
    pub encoding: String,
}

impl RawString {
    /// 尝试多种编码解码
    pub fn decode(data: &[u8]) -> Self {
        for encoding_name in SUPPORTED_ENCODINGS {
            if let Some(encoding) = encoding_rs::Encoding::for_label(encoding_name.as_bytes()) {
                let (decoded, _, had_errors) = encoding.decode(data);
                if !had_errors {
                    return RawString {
                        content: decoded.into_owned(),
                        encoding: encoding_name.to_string(),
                    };
                }
            }
        }
        
        // 回退到UTF-8，忽略错误
        RawString {
            content: String::from_utf8_lossy(data).into_owned(),
            encoding: "utf-8".to_string(),
        }
    }
    
    /// Z字符串解析(以null结尾)
    pub fn parse_zstring(data: &[u8]) -> Self {
        let null_pos = data.iter().position(|&b| b == 0).unwrap_or(data.len());
        Self::decode(&data[..null_pos])
    }
    
    /// B字符串解析(长度前缀)
    pub fn parse_bstring(cursor: &mut Cursor<&[u8]>) -> Result<Self, std::io::Error> {
        let length = read_u8(cursor)? as usize;
        let mut buffer = vec![0u8; length];
        cursor.read_exact(&mut buffer)?;
        
        // 移除末尾的null字符
        if let Some(null_pos) = buffer.iter().position(|&b| b == 0) {
            buffer.truncate(null_pos);
        }
        
        Ok(Self::decode(&buffer))
    }
}

// 记录标志位定义
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct RecordFlags: u32 {
        const MASTER_FILE = 0x00000001;        // ESM标志
        const UNKNOWN_02 = 0x00000002;         // 未知标志位 0x02
        const UNKNOWN_04 = 0x00000004;         // 未知标志位 0x04 (在用户数据中出现)
        const UNKNOWN_08 = 0x00000008;         // 未知标志位 0x08 (在用户数据中出现)
        const UNKNOWN_10 = 0x00000010;         // 未知标志位 0x10 (在用户数据中出现)
        const DELETED = 0x00000020;            // 已删除
        const UNKNOWN_40 = 0x00000040;         // 未知标志位 0x40
        const LOCALIZED = 0x00000080;          // 本地化
        const UNKNOWN_100 = 0x00000100;       // 未知标志位 0x100
        const LIGHT_MASTER = 0x00000200;       // 轻量级主文件
        const PERSISTENT = 0x00000400;         // 持久化
        const DISABLED = 0x00000800;           // 禁用
        const UNKNOWN_1000 = 0x00001000;      // 未知标志位 0x1000 (在用户数据中出现)
        const UNKNOWN_2000 = 0x00002000;      // 未知标志位 0x2000
        const UNKNOWN_4000 = 0x00004000;      // 未知标志位 0x4000
        const VISIBLE_DISTANT = 0x00008000;    // 远距离可见
        const UNKNOWN_10000 = 0x00010000;     // 未知标志位 0x10000 (在用户数据中出现)
        const UNKNOWN_20000 = 0x00020000;     // 未知标志位 0x20000
        const COMPRESSED = 0x00040000;         // 压缩
        const UNKNOWN_80000 = 0x00080000;     // 未知标志位 0x80000
        const UNKNOWN_100000 = 0x00100000;    // 未知标志位 0x100000
        const UNKNOWN_200000 = 0x00200000;    // 未知标志位 0x200000
        const UNKNOWN_400000 = 0x00400000;    // 未知标志位 0x400000
        const UNKNOWN_800000 = 0x00800000;    // 未知标志位 0x800000
        const UNKNOWN_1000000 = 0x01000000;   // 未知标志位 0x1000000 (在用户数据中出现)
        const UNKNOWN_2000000 = 0x02000000;   // 未知标志位 0x2000000
        const UNKNOWN_4000000 = 0x04000000;   // 未知标志位 0x4000000
        const UNKNOWN_8000000 = 0x08000000;   // 未知标志位 0x8000000
        const UNKNOWN_10000000 = 0x10000000;  // 未知标志位 0x10000000 (在用户数据中出现)
        const UNKNOWN_20000000 = 0x20000000;  // 未知标志位 0x20000000
        const UNKNOWN_40000000 = 0x40000000;  // 未知标志位 0x40000000
        const UNKNOWN_80000000 = 0x80000000;  // 未知标志位 0x80000000
    }
} 
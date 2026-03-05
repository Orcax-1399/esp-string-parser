//! 流式字符串提取快路径（仅用于提取，不构建 GRUP/Record/Subrecord 树）
//!
//! 设计目标：
//! - mmap 后按字节扫描 group/record/subrecord
//! - 仅解析 EDID 与白名单字符串子记录
//! - 压缩记录仅在 record_type 命中白名单时才解压
//! - 本地化插件：按既有 determine_string_file_type 路由读取 STRING 文件

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

use flate2::read::ZlibDecoder;
use memmap2::Mmap;
use rayon::prelude::*;

use crate::datatypes::{RawString, RecordFlags};
use crate::string_file::{StringFileSet, StringFileType};
use crate::string_types::ExtractedString;
use crate::utils::{is_valid_string, EspError};

const GRUP: &[u8; 4] = b"GRUP";
const TES4: &[u8; 4] = b"TES4";
const TES3: &[u8; 4] = b"TES3";
const XXXX_U32: u32 = u32::from_le_bytes(*b"XXXX");
const EDID_U32: u32 = u32::from_le_bytes(*b"EDID");
const MAST_U32: u32 = u32::from_le_bytes(*b"MAST");

static ROUTES_U32: OnceLock<HashMap<u32, Box<[u32]>>> = OnceLock::new();

#[derive(Debug)]
struct ScanCtx {
    routes: &'static HashMap<u32, Box<[u32]>>,
    plugin_filename: String,
    masters: Vec<String>,
    is_localized: bool,
    string_files: Option<StringFileSet>,
}

/// 快路径提取入口：默认语言由调用者传入（通常为 "english"）
pub fn extract_strings_fast(path: &Path, language: &str) -> Result<Vec<ExtractedString>, EspError> {
    let file = File::open(path).map_err(EspError::IoError)?;
    let mmap = unsafe { Mmap::map(&file).map_err(EspError::IoError)? };
    let data = &mmap[..];

    let (flags, masters, cursor_pos) = parse_tes4_header(data)?;
    let is_localized = (flags & RecordFlags::LOCALIZED.bits()) != 0;

    let plugin_filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("plugin.esp")
        .to_string();

    let string_files = if is_localized {
        let plugin_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("plugin");

        match StringFileSet::load_auto_for_plugin(path, plugin_name, language) {
            Ok(set) if !set.files.is_empty() => Some(set),
            _ => None, // 缺失时按既有策略降级为 StringID 占位符
        }
    } else {
        None
    };

    let routes = routes_u32();
    let group_ranges = scan_top_level_group_ranges(data, cursor_pos)?;

    let ctx = ScanCtx {
        routes,
        plugin_filename,
        masters,
        is_localized,
        string_files,
    };

    let per_group: Vec<Vec<ExtractedString>> = group_ranges
        .par_iter()
        .map(|&(start, size)| {
            let end = start
                .checked_add(size)
                .ok_or(EspError::InvalidFormat)?;
            if end > data.len() {
                return Err(EspError::InvalidFormat);
            }
            scan_group_for_strings(&data[start..end], &ctx)
        })
        .collect::<Result<_, _>>()?;

    Ok(per_group.into_iter().flatten().collect())
}

fn routes_u32() -> &'static HashMap<u32, Box<[u32]>> {
    ROUTES_U32.get_or_init(|| {
        let json_data = include_str!("../data/string_records.json");
        let routes: HashMap<String, Vec<String>> =
            serde_json::from_str(json_data).expect("内置 string_records.json 解析失败");

        let mut map: HashMap<u32, Box<[u32]>> = HashMap::with_capacity(routes.len());

        for (record_type, mut subrecords) in routes {
            let Some(record_u32) = fourcc_str_to_u32(&record_type) else {
                continue;
            };

            let mut subs_u32: Vec<u32> = subrecords
                .drain(..)
                .filter_map(|s| fourcc_str_to_u32(&s))
                .collect();

            subs_u32.sort_unstable();
            subs_u32.dedup();

            map.insert(record_u32, subs_u32.into_boxed_slice());
        }

        map
    })
}

fn fourcc_str_to_u32(s: &str) -> Option<u32> {
    let bytes = s.as_bytes();
    if bytes.len() != 4 {
        return None;
    }
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u16_le(data: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = data.get(offset..offset + 2)?.try_into().ok()?;
    Some(u16::from_le_bytes(bytes))
}

fn read_u32_le(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn read_fourcc(data: &[u8], offset: usize) -> Option<[u8; 4]> {
    data.get(offset..offset + 4)?.try_into().ok()
}

fn parse_tes4_header(data: &[u8]) -> Result<(u32, Vec<String>, usize), EspError> {
    if data.len() < 24 {
        return Err(EspError::InvalidFormat);
    }

    let Some(record_type) = read_fourcc(data, 0) else {
        return Err(EspError::InvalidFormat);
    };

    if &record_type != TES4 && &record_type != TES3 {
        return Err(EspError::InvalidFormat);
    }

    let data_size = read_u32_le(data, 4).ok_or(EspError::InvalidFormat)? as usize;
    let flags = read_u32_le(data, 8).ok_or(EspError::InvalidFormat)?;

    let end = 24usize
        .checked_add(data_size)
        .ok_or(EspError::InvalidFormat)?;
    if end > data.len() {
        return Err(EspError::InvalidFormat);
    }

    let header_data = &data[24..end];
    let masters = scan_header_masters(header_data)?;

    Ok((flags, masters, end))
}

fn scan_header_masters(header_data: &[u8]) -> Result<Vec<String>, EspError> {
    let mut masters = Vec::new();
    let mut offset = 0usize;

    while offset < header_data.len() {
        let remaining = header_data.len() - offset;
        if remaining < 6 {
            if header_data[offset..].iter().all(|&b| b == 0) {
                break;
            }
            return Err(EspError::InvalidFormat);
        }

        let view = parse_subrecord_view(header_data, offset)?;
        if view.record_type_u32 == MAST_U32 {
            masters.push(RawString::parse_zstring(view.data).content);
        }
        offset = view.next_offset;
    }

    Ok(masters)
}

fn scan_top_level_group_ranges(data: &[u8], start_pos: usize) -> Result<Vec<(usize, usize)>, EspError> {
    let mut ranges = Vec::new();
    let mut pos = start_pos;

    while pos < data.len() {
        if pos + 8 > data.len() {
            break;
        }

        let Some(tag) = read_fourcc(data, pos) else {
            return Err(EspError::InvalidFormat);
        };

        if &tag != GRUP {
            return Err(EspError::InvalidFormat);
        }

        let size = read_u32_le(data, pos + 4).ok_or(EspError::InvalidFormat)? as usize;
        if size < 24 || size > 200_000_000 {
            return Err(EspError::InvalidFormat);
        }

        let end = pos.checked_add(size).ok_or(EspError::InvalidFormat)?;
        if end > data.len() {
            return Err(EspError::InvalidFormat);
        }

        ranges.push((pos, size));
        pos = end;
    }

    Ok(ranges)
}

fn scan_group_for_strings(group_data: &[u8], ctx: &ScanCtx) -> Result<Vec<ExtractedString>, EspError> {
    if group_data.len() < 24 {
        return Err(EspError::InvalidFormat);
    }
    if &group_data[0..4] != GRUP {
        return Err(EspError::InvalidFormat);
    }

    let declared_size = read_u32_le(group_data, 4).ok_or(EspError::InvalidFormat)? as usize;
    if declared_size != group_data.len() {
        return Err(EspError::InvalidFormat);
    }

    let mut out = Vec::new();
    let mut stack: Vec<&[u8]> = vec![group_data];

    while let Some(group) = stack.pop() {
        if group.len() < 24 || &group[0..4] != GRUP {
            return Err(EspError::InvalidFormat);
        }

        let mut offset = 24usize;
        while offset < group.len() {
            if offset + 4 > group.len() {
                return Err(EspError::InvalidFormat);
            }

            if &group[offset..offset + 4] == GRUP {
                let size = read_u32_le(group, offset + 4).ok_or(EspError::InvalidFormat)? as usize;
                if size < 24 || size > 200_000_000 {
                    return Err(EspError::InvalidFormat);
                }

                let end = offset.checked_add(size).ok_or(EspError::InvalidFormat)?;
                if end > group.len() {
                    return Err(EspError::InvalidFormat);
                }

                stack.push(&group[offset..end]);
                offset = end;
                continue;
            }

            let record = parse_record_view(group, offset)?;

            if let Some(allowed_subs) = ctx.routes.get(&record.record_type_u32) {
                // 仅在命中白名单时才考虑解压/扫描
                let decompressed;
                let data_to_scan: &[u8] = if (record.flags & RecordFlags::COMPRESSED.bits()) != 0 {
                    match decompress_record_data(record.data) {
                        Ok(data) => {
                            decompressed = data;
                            &decompressed
                        }
                        Err(_) => {
                            // 与旧路径一致：解压失败时跳过该记录的子记录扫描
                            offset = record.next_offset;
                            continue;
                        }
                    }
                } else {
                    record.data
                };
                out.extend(scan_record_for_strings(
                    record.record_type_bytes,
                    record.flags,
                    record.form_id,
                    data_to_scan,
                    allowed_subs,
                    ctx,
                )?);
            }

            offset = record.next_offset;
        }
    }

    Ok(out)
}

#[derive(Debug)]
struct RecordView<'a> {
    record_type_bytes: [u8; 4],
    record_type_u32: u32,
    flags: u32,
    form_id: u32,
    data: &'a [u8],
    next_offset: usize,
}

fn parse_record_view<'a>(data: &'a [u8], offset: usize) -> Result<RecordView<'a>, EspError> {
    if offset + 24 > data.len() {
        return Err(EspError::InvalidFormat);
    }

    let record_type_bytes = read_fourcc(data, offset).ok_or(EspError::InvalidFormat)?;
    let record_type_u32 = u32::from_le_bytes(record_type_bytes);

    let data_size_u32 = read_u32_le(data, offset + 4).ok_or(EspError::InvalidFormat)?;
    if data_size_u32 > 100_000_000 {
        return Err(EspError::InvalidFormat);
    }
    let data_size = data_size_u32 as usize;

    let flags = read_u32_le(data, offset + 8).ok_or(EspError::InvalidFormat)?;
    let form_id = read_u32_le(data, offset + 12).ok_or(EspError::InvalidFormat)?;

    let data_start = offset + 24;
    let data_end = data_start
        .checked_add(data_size)
        .ok_or(EspError::InvalidFormat)?;
    if data_end > data.len() {
        return Err(EspError::InvalidFormat);
    }

    Ok(RecordView {
        record_type_bytes,
        record_type_u32,
        flags,
        form_id,
        data: &data[data_start..data_end],
        next_offset: data_end,
    })
}

#[derive(Debug)]
struct SubrecordView<'a> {
    record_type_bytes: [u8; 4],
    record_type_u32: u32,
    data: &'a [u8],
    next_offset: usize,
}

fn parse_subrecord_view<'a>(data: &'a [u8], offset: usize) -> Result<SubrecordView<'a>, EspError> {
    if offset + 6 > data.len() {
        return Err(EspError::InvalidFormat);
    }

    let record_type_bytes = read_fourcc(data, offset).ok_or(EspError::InvalidFormat)?;
    let record_type_u32 = u32::from_le_bytes(record_type_bytes);
    let size = read_u16_le(data, offset + 4).ok_or(EspError::InvalidFormat)? as usize;

    if record_type_u32 == XXXX_U32 {
        // XXXX size 必须是 4，随后是真实大小(u32) + 目标 subrecord 的头部(6)
        if size != 4 || offset + 16 > data.len() {
            return Err(EspError::InvalidFormat);
        }

        let field_size = read_u32_le(data, offset + 6).ok_or(EspError::InvalidFormat)? as usize;
        let next_type_bytes = read_fourcc(data, offset + 10).ok_or(EspError::InvalidFormat)?;

        // next_size 通常为 0（这里只做读取/边界校验）
        let _next_size = read_u16_le(data, offset + 14).ok_or(EspError::InvalidFormat)?;

        let data_start = offset + 16;
        let data_end = data_start
            .checked_add(field_size)
            .ok_or(EspError::InvalidFormat)?;
        if data_end > data.len() {
            return Err(EspError::InvalidFormat);
        }

        Ok(SubrecordView {
            record_type_bytes: next_type_bytes,
            record_type_u32: u32::from_le_bytes(next_type_bytes),
            data: &data[data_start..data_end],
            next_offset: data_end,
        })
    } else {
        let data_start = offset + 6;
        let data_end = data_start.checked_add(size).ok_or(EspError::InvalidFormat)?;
        if data_end > data.len() {
            return Err(EspError::InvalidFormat);
        }

        Ok(SubrecordView {
            record_type_bytes,
            record_type_u32,
            data: &data[data_start..data_end],
            next_offset: data_end,
        })
    }
}

fn decompress_record_data(data: &[u8]) -> Result<Vec<u8>, EspError> {
    if data.len() < 4 {
        return Err(EspError::CompressionError("压缩数据太短，无法包含解压大小".to_string()));
    }

    let expected_size = read_u32_le(data, 0).ok_or_else(|| {
        EspError::CompressionError("无法读取解压大小".to_string())
    })?;

    if expected_size == 0 || expected_size > 50_000_000 {
        return Err(EspError::CompressionError(format!(
            "解压大小异常: {} bytes",
            expected_size
        )));
    }

    let compressed = &data[4..];
    if compressed.is_empty() {
        return Err(EspError::CompressionError("没有压缩数据".to_string()));
    }

    let mut decoder = ZlibDecoder::new(compressed);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| EspError::CompressionError(e.to_string()))?;

    if out.len() != expected_size as usize {
        return Err(EspError::CompressionError(format!(
            "解压大小不匹配: 期望 {} bytes，实际 {} bytes",
            expected_size,
            out.len()
        )));
    }

    Ok(out)
}

fn format_form_id(form_id: u32, masters: &[String], plugin_filename: &str) -> String {
    let master_index = (form_id >> 24) as usize;
    let master_file = if master_index < masters.len() {
        &masters[master_index]
    } else {
        plugin_filename
    };
    format!("{:08X}|{}", form_id, master_file)
}

fn scan_record_for_strings(
    record_type_bytes: [u8; 4],
    flags: u32,
    form_id: u32,
    record_data: &[u8],
    allowed_subs: &[u32],
    ctx: &ScanCtx,
) -> Result<Vec<ExtractedString>, EspError> {
    let record_type = String::from_utf8_lossy(&record_type_bytes).into_owned();
    let form_id_str = format_form_id(form_id, &ctx.masters, &ctx.plugin_filename);

    let mut editor_id: Option<String> = None;
    let mut index = 0i32;
    let mut out = Vec::new();
    let mut offset = 0usize;

    while offset < record_data.len() {
        let remaining = record_data.len() - offset;
        if remaining < 6 {
            if record_data[offset..].iter().all(|&b| b == 0) {
                break;
            }
            return Err(EspError::InvalidFormat);
        }

        let view = parse_subrecord_view(record_data, offset)?;
        offset = view.next_offset;

        if view.record_type_u32 == EDID_U32 {
            editor_id = Some(parse_edid(view.data));
            continue;
        }

        if allowed_subs.binary_search(&view.record_type_u32).is_err() {
            continue;
        }

        let subrecord_type = String::from_utf8_lossy(&view.record_type_bytes).into_owned();

        let extracted = extract_string_from_subrecord_fast(
            ctx,
            &editor_id,
            &form_id_str,
            &record_type,
            &subrecord_type,
            view.data,
            flags,
            index,
        );

        if let Some(s) = extracted {
            out.push(s);
        }

        // 与旧逻辑一致：只要命中字符串子记录，index 就递增（无论是否成功提取）
        index += 1;
    }

    Ok(out)
}

fn parse_edid(data: &[u8]) -> String {
    let null_pos = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    String::from_utf8_lossy(&data[..null_pos]).into_owned()
}

fn extract_string_from_subrecord_fast(
    ctx: &ScanCtx,
    editor_id: &Option<String>,
    form_id_str: &str,
    record_type: &str,
    subrecord_type: &str,
    subrecord_data: &[u8],
    _record_flags: u32,
    index: i32,
) -> Option<ExtractedString> {
    let raw_text = if ctx.is_localized {
        if subrecord_data.len() < 4 {
            return None;
        }
        let string_id = u32::from_le_bytes(subrecord_data[0..4].try_into().ok()?);

        // StringID 为 0 表示无字符串或空字段
        if string_id == 0 {
            return None;
        }

        let file_type: StringFileType = crate::Plugin::determine_string_file_type(record_type, subrecord_type);

        if let Some(ref set) = ctx.string_files {
            if let Some(entry) = set.get_string_by_type(file_type, string_id) {
                entry.content.clone()
            } else {
                format!("StringID_{}_{:?}", string_id, file_type)
            }
        } else {
            format!("StringID_{}", string_id)
        }
    } else {
        RawString::parse_zstring(subrecord_data).content
    };

    if !is_valid_string(&raw_text) {
        return None;
    }

    Some(ExtractedString::new(
        editor_id.clone(),
        form_id_str.to_string(),
        record_type.to_string(),
        subrecord_type.to_string(),
        raw_text,
        index,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn test_parse_subrecord_view_xxxx_extends_size() {
        let mut buf = Vec::new();

        // XXXX header
        buf.extend_from_slice(b"XXXX");
        buf.extend_from_slice(&(4u16).to_le_bytes());
        buf.extend_from_slice(&(10u32).to_le_bytes()); // 实际字段大小

        // next header (size 应为 0)
        buf.extend_from_slice(b"FULL");
        buf.extend_from_slice(&(0u16).to_le_bytes());

        // data (10 bytes)
        buf.extend_from_slice(b"1234567890");

        let view = parse_subrecord_view(&buf, 0).expect("XXXX subrecord 应解析成功");
        assert_eq!(&view.record_type_bytes, b"FULL");
        assert_eq!(view.record_type_u32, u32::from_le_bytes(*b"FULL"));
        assert_eq!(view.data, b"1234567890");
        assert_eq!(view.next_offset, buf.len());
    }

    #[test]
    fn test_decompress_record_data_roundtrip() {
        let payload = b"EDID\x04\0test\0";

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(payload).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut record_data = Vec::new();
        record_data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        record_data.extend_from_slice(&compressed);

        let decompressed = decompress_record_data(&record_data).expect("应能解压成功");
        assert_eq!(&decompressed, payload);
    }

    #[test]
    fn test_scan_group_skips_invalid_compressed_record() {
        let record_data = [0x0A, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03];
        let mut group = Vec::new();

        // GRUP header (24 bytes)
        group.extend_from_slice(b"GRUP");
        group.extend_from_slice(&0u32.to_le_bytes()); // size placeholder
        group.extend_from_slice(&[0u8; 16]);

        // CELL record header (24 bytes)
        group.extend_from_slice(b"CELL");
        group.extend_from_slice(&(record_data.len() as u32).to_le_bytes());
        group.extend_from_slice(&RecordFlags::COMPRESSED.bits().to_le_bytes());
        group.extend_from_slice(&0x0100_0000u32.to_le_bytes()); // form id
        group.extend_from_slice(&0u16.to_le_bytes()); // timestamp
        group.extend_from_slice(&0u16.to_le_bytes()); // version control
        group.extend_from_slice(&0u16.to_le_bytes()); // internal version
        group.extend_from_slice(&0u16.to_le_bytes()); // unknown
        group.extend_from_slice(&record_data);

        // fill GRUP size
        let total_size = group.len() as u32;
        group[4..8].copy_from_slice(&total_size.to_le_bytes());

        let ctx = ScanCtx {
            routes: routes_u32(),
            plugin_filename: "test.esp".to_string(),
            masters: Vec::new(),
            is_localized: false,
            string_files: None,
        };

        let out = scan_group_for_strings(&group, &ctx)
            .expect("解压失败的压缩记录应被跳过，而不是导致整个扫描失败");
        assert!(out.is_empty());
    }
}

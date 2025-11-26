use std::path::{Path, PathBuf};

use super::StringFileType;

/// 解析文件名获取插件名、语言和文件类型
pub(crate) fn parse_filename(path: &Path) -> Result<(String, String, StringFileType), Box<dyn std::error::Error>> {
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("无效的文件名")?;

    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or("无效的文件扩展名")?;

    let file_type = StringFileType::from_extension(extension).ok_or("不支持的字符串文件类型")?;

    let parts: Vec<&str> = filename.rsplitn(2, '_').collect();
    if parts.len() != 2 {
        return Err("文件名格式错误，应为：PluginName_Language.EXTENSION".into());
    }

    let language = parts[0].to_string();
    let plugin_name = parts[1].to_string();

    Ok((plugin_name, language, file_type))
}

pub(crate) fn build_filename_variants(
    directory: &Path,
    plugin_name: &str,
    language: &str,
    file_type: StringFileType,
) -> Vec<PathBuf> {
    let name_variants = vec![
        plugin_name.to_string(),
        plugin_name.to_lowercase(),
        plugin_name.to_uppercase(),
    ];

    let mut candidates = Vec::new();

    for name_variant in name_variants {
        let filename = format!("{}_{}.{}", name_variant, language, file_type.to_extension());
        candidates.push(directory.join(&filename));

        let filename_lower_ext =
            format!("{}_{}.{}", name_variant, language, file_type.to_extension().to_lowercase());
        candidates.push(directory.join(filename_lower_ext));
    }

    candidates
}

use serde_json;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedString {
    pub editor_id: Option<String>,
    pub form_id: String,
    pub original_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    pub record_type: String,
    pub subrecord_type: String,
}

fn main() {
    // 模拟提取字符串（translated_text 是 None）
    let extracted = ExtractedString {
        editor_id: Some("IronSword".to_string()),
        form_id: "00012E8B|Skyrim.esm".to_string(),
        original_text: "Iron Sword".to_string(),
        translated_text: None,  // 提取时是 None
        record_type: "WEAP".to_string(),
        subrecord_type: "FULL".to_string(),
    };
    
    println!("=== 导出时的JSON（translated_text=None） ===");
    let json = serde_json::to_string_pretty(&extracted).unwrap();
    println!("{}\n", json);
    
    // 模拟应用翻译（translated_text 有值）
    let translated = ExtractedString {
        editor_id: Some("IronSword".to_string()),
        form_id: "00012E8B|Skyrim.esm".to_string(),
        original_text: "Iron Sword".to_string(),
        translated_text: Some("铁剑".to_string()),  // 应用翻译时有值
        record_type: "WEAP".to_string(),
        subrecord_type: "FULL".to_string(),
    };
    
    println!("=== 应用翻译时的JSON（translated_text=Some） ===");
    let json = serde_json::to_string_pretty(&translated).unwrap();
    println!("{}", json);
}

use esp_extractor::LoadedPlugin;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 检查 AMMO 记录 ===\n");

    let loaded = LoadedPlugin::load_auto(
        PathBuf::from("testFile/ccbgssse002-exoticarrows.esl"),
        Some("english"),
    )?;

    let plugin = loaded.plugin();

    // 查找所有 AMMO 记录
    for group in &plugin.groups {
        check_group_for_ammo(group, 0);
    }

    Ok(())
}

fn check_group_for_ammo(group: &esp_extractor::Group, depth: usize) {
    use esp_extractor::group::GroupChild;

    for child in &group.children {
        match child {
            GroupChild::Group(subgroup) => {
                check_group_for_ammo(subgroup, depth + 1);
            }
            GroupChild::Record(record) => {
                if record.record_type == "AMMO" {
                    println!("AMMO 记录: FormID={:08X}", record.form_id);

                    // 查找 EDID
                    let editor_id = record.get_editor_id();
                    if let Some(ref edid) = editor_id {
                        println!("  EDID: {}", edid);
                    }

                    // 查找所有子记录
                    for subrecord in &record.subrecords {
                        println!(
                            "  子记录: {} (size={})",
                            subrecord.record_type,
                            subrecord.data.len()
                        );

                        if subrecord.record_type == "DESC" {
                            // 显示 DESC 的内容（作为 StringID）
                            if subrecord.data.len() >= 4 {
                                let string_id = u32::from_le_bytes([
                                    subrecord.data[0],
                                    subrecord.data[1],
                                    subrecord.data[2],
                                    subrecord.data[3],
                                ]);
                                println!("    ⚠️ DESC StringID: {}", string_id);
                            }
                        }
                    }
                    println!();
                }
            }
        }
    }
}

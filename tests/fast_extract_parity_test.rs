use std::collections::HashMap;
use std::path::PathBuf;

use esp_extractor::{ExtractedString, LoadedPlugin};

fn to_unique_map(strings: Vec<ExtractedString>) -> HashMap<String, String> {
    let mut map = HashMap::with_capacity(strings.len());
    for s in strings {
        let key = s.get_unique_key();
        let old = map.insert(key, s.text);
        assert!(old.is_none(), "发现重复 unique_key（旧值被覆盖）");
    }
    map
}

#[test]
fn fast_extract_parity_gosted_dimensional_rift() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("TestFile/GostedDimensionalRift.esp");

    let legacy = LoadedPlugin::load_auto(path.clone(), Some("english"))?.extract_strings();
    let fast = esp_extractor::fast_extract::extract_strings_fast(path.as_ref(), "english")?;

    let legacy_map = to_unique_map(legacy);
    let fast_map = to_unique_map(fast);

    assert_eq!(legacy_map, fast_map);
    Ok(())
}

#[test]
fn fast_extract_parity_ccbgssse001_fish_localized() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("TestFile/ccbgssse001-fish.esm");

    let legacy = LoadedPlugin::load_auto(path.clone(), Some("english"))?.extract_strings();
    let fast = esp_extractor::fast_extract::extract_strings_fast(path.as_ref(), "english")?;

    let legacy_map = to_unique_map(legacy);
    let fast_map = to_unique_map(fast);

    assert_eq!(legacy_map, fast_map);
    Ok(())
}


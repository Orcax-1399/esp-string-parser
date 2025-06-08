pub mod datatypes;
pub mod record;
pub mod group;
pub mod plugin;
pub mod subrecord;
pub mod string_types;
pub mod utils;
pub mod debug;

// 重新导出主要结构
pub use plugin::Plugin;
pub use record::Record;
pub use group::Group;
pub use subrecord::Subrecord;
pub use string_types::ExtractedString;
pub use utils::{is_valid_string, EspError};
pub use debug::EspDebugger;

// 常量定义
pub const SUPPORTED_EXTENSIONS: &[&str] = &["esp", "esm", "esl"]; 
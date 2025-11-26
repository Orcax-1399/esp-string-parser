//! 字符串路由模块
//!
//! 负责管理记录类型到字符串子记录类型的映射关系

mod data;
mod router;

pub use router::{StringRouter, DefaultStringRouter};
pub(crate) use data::load_string_records;

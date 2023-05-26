use super::layout::Split;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::Path;

#[allow(dead_code)]
pub const SESSION_EXAMPLE: &str = "sort_by = \"Name\"
show_hidden = false
preview = false
split = Vertical
";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Session {
    pub sort_by: SortKey,
    pub show_hidden: bool,
    pub preview: Option<bool>,
    pub split: Option<Split>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SortKey {
    Name,
    Time,
}

pub fn read_session(session_path: &Path) -> Session {
    match read_to_string(session_path) {
        Ok(s) => match serde_yaml::from_str(&s) {
            Ok(de) => de,
            Err(_) => Session {
                sort_by: SortKey::Name,
                show_hidden: true,
                preview: Some(false),
                split: Some(Split::Vertical),
            },
        },
        Err(_) => Session {
            sort_by: SortKey::Name,
            show_hidden: true,
            preview: Some(false),
            split: Some(Split::Vertical),
        },
    }
}

use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use super::config::SortKey;

use crate::state::FX_CONFIG_DIR;

pub const SESSION_FILE: &str = ".session";

pub const SESSION_EXAMPLE: &str = "sort_by = \"Time\"
show_hidden = false
";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Session {
    pub sort_by: SortKey,
    pub show_hidden: bool
}

#[derive(Deserialize,Serialize, Debug, Clone)]
pub enum SortKey {
    Name,
    Time,
}

pub fn read_session() -> Option<Session> {
    let mut session = dirs::config_dir().unwrap_or_else(|| panic!("cannot read config dir."));
    session.push(FX_CONFIG_DIR);
    session.push(SESSION_FILE);
    let session = read_to_string(session.as_path());
    if let Ok(session) = session {
        let deserialized: Session = toml::from_str(&session).unwrap();
        Some(deserialized)
    } else {
        None
    }
}
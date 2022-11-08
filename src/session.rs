use crate::layout::Split;

use super::errors::FxError;
use super::state::FX_CONFIG_DIR;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::Path;

pub const SESSION_FILE: &str = ".session";
pub const SESSION_EXAMPLE: &str = "sort_by = \"Name\"
show_hidden = false
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

pub fn read_session() -> Result<Session, FxError> {
    let mut session = dirs::config_dir().unwrap_or_else(|| panic!("Cannot read config dir."));
    session.push(FX_CONFIG_DIR);
    session.push(SESSION_FILE);
    let session = read_to_string(session.as_path())?;
    let deserialized: Session = toml::from_str(&session)?;
    Ok(deserialized)
}

pub fn make_session(session_file: &Path) -> Result<(), FxError> {
    std::fs::write(&session_file, SESSION_EXAMPLE)?;
    Ok(())
}

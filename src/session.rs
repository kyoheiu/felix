use super::errors::FxError;
use super::layout::Split;
use super::state::FX_CONFIG_DIR;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::Path;

pub const SESSION_FILE: &str = ".session";
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

pub fn read_session() -> Result<Session, FxError> {
    let mut session =
        dirs::config_dir().ok_or_else(|| FxError::Dirs("Cannot read config dir.".to_string()))?;
    session.push(FX_CONFIG_DIR);
    session.push(SESSION_FILE);
    let session = read_to_string(session.as_path())?;
    match toml::from_str(&session) {
        Ok(de) => Ok(de),
        Err(_) => Ok(Session {
            sort_by: SortKey::Name,
            show_hidden: true,
            preview: Some(false),
            split: Some(Split::Vertical),
        }),
    }
}

pub fn make_session(session_file: &Path) -> Result<(), FxError> {
    std::fs::write(&session_file, SESSION_EXAMPLE)?;
    Ok(())
}

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
    let session = read_to_string(session.as_path());
    if let Ok(session) = session {
        let deserialized: Session = toml::from_str(&session)?;
        Ok(deserialized)
    } else {
        panic!("Cannot deserialize session file.");
    }
}

pub fn make_session(session_file: &Path) -> Result<(), FxError> {
    std::fs::write(&session_file, SESSION_EXAMPLE)
        .unwrap_or_else(|_| panic!("Cannot write new session file."));
    Ok(())
}

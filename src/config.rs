use serde::Deserialize;
use std::collections::HashMap;
use std::fs::read_to_string;

use crate::state::FM_CONFIG_DIR;

const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub default: String,
    pub warning: bool,
    pub sort_by: SortKey,
    pub exec: HashMap<String, Vec<String>>,
    pub color: Color,
}

#[derive(Deserialize, Debug, Clone)]
pub enum SortKey {
    Name,
    Time,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Color {
    pub dir_fg: Colorname,
    pub file_fg: Colorname,
    pub symlink_fg: Colorname,
}

#[derive(Deserialize, Debug, Clone)]
pub enum Colorname {
    AnsiValue(u8),
    Black,
    Blue,
    Cyan,
    Green,
    LightBlack,
    LightBlue,
    LightCyan,
    LightGreen,
    LightMagenta,
    LightRed,
    LightWhite,
    LightYellow,
    Magenta,
    Red,
    Rgb(u8, u8, u8),
    White,
    Yellow,
}

pub fn read_config() -> Option<Config> {
    let mut config = dirs::config_dir().unwrap_or_else(|| panic!("cannot read config dir."));
    config.push(FM_CONFIG_DIR);
    config.push(CONFIG_FILE);
    let config = read_to_string(config.as_path());
    if let Ok(config) = config {
        let deserialized: Config = toml::from_str(&config).unwrap();
        return Some(deserialized);
    } else {
        None
    }
}

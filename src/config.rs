use serde::Deserialize;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::Path;

const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize, Debug)]
pub struct Config {
    pub color: Color,
    pub exec: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct Color {
    pub current_dir_bg: Colorname,
    pub current_dir_fg: Colorname,
    pub dir_fg: Colorname,
    pub file_fg: Colorname,
}

#[derive(Deserialize, Debug)]
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
    let path = Path::new(CONFIG_FILE);
    let config = read_to_string(path);
    if let Ok(config) = config {
        let deserialized: Config = toml::from_str(&config).unwrap();
        return Some(deserialized);
    } else {
        None
    }
}

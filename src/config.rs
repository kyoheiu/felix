use serde::Deserialize;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::io::BufReader;
use std::path::Path;
use toml::*;

const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize, Debug)]
pub struct Config {
    color: Color,
    exec: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct Color {
    current_dir_bg: Colorname,
    current_dir_fg: Colorname,
    dir_fg: Colorname,
    file_fg: Colorname,
}

#[derive(Deserialize, Debug)]
enum Colorname {
    AnsiValue,
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

use serde::Deserialize;
use std::collections::HashMap;
use std::fs::read_to_string;

use crate::items::CONFIG_DIR;

const CONFIG_FILE: &str = "config.toml";
pub const CONFIG_EXAMPLE: &str = "
# Choose the foreground color of file and directory in the list.
# Pick one of the following:
#   AnsiValue(u8)
#   Black
#   Blue
#   Cyan
#   Green
#   LightBlack
#   LightBlue
#   LightCyan
#   LightGreen
#   LightMagenta
#   LightRed
#   LightWhite
#   LightYellow
#   Magenta
#   Red
#   Rgb(u8, u8, u8)
#   White
#   Yellow
# For more detail, read https://docs.rs/termion/1.5.6/termion/color/index.html
[color]
dir_fg = \"LightCyan\"
file_fg = \"LightWhite\"

# key(extention) = value(command you want to use)
# `default` is mandatory
[exec]
default = \"nvim\"
jpg = \"feh\"
pdf = \"zathura\"
png = \"feh\"";

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub color: Color,
    pub exec: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Color {
    pub dir_fg: Colorname,
    pub file_fg: Colorname,
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
    config.push(CONFIG_DIR);
    config.push(CONFIG_FILE);
    let config = read_to_string(config.as_path());
    if let Ok(config) = config {
        let deserialized: Config = toml::from_str(&config).unwrap();
        return Some(deserialized);
    } else {
        None
    }
}

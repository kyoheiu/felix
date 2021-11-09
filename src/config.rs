use serde::Deserialize;
use std::collections::HashMap;
use std::fs::read_to_string;

use crate::state::FM_CONFIG_DIR;

const CONFIG_FILE: &str = "config.toml";

pub const CONFIG_EXAMPLE: &str = "
# default exec command when open files
default = \"nvim\"

# default key for sorting item list (\"Name\" or \"Time\")
sort_by = \"Name\"

# key(command you want to use) = values(extensions)
[exec]
feh = [\"jpg\", \"jpeg\", \"png\", \"gif\", \"svg\"]
zathura = [\"pdf\"]

# the foreground color of file and directory in the list
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
symlink_fg = \"LightYellow\"
";

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub default: String,
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
        Some(deserialized)
    } else {
        None
    }
}

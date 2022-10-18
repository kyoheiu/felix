use super::errors::FxError;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::Path;

use crate::state::FX_CONFIG_DIR;

const CONFIG_FILE: &str = "config.toml";

pub const CONFIG_EXAMPLE: &str = "
# (Optional) Default exec command when open files.
# If not set, will default to $EDITOR
default = \"nvim\"

# (Optional) Whether to use the full width of terminal.
# If not set, this will be true.
# use_full_width = true

# (Optional) Set the max length of item name to be displayed.
# This works only when use_full_width is set to false.
# If the terminal size is not enough, the length will be changed to fit it.
# If not set, this will be 30.
# item_name_length = 30

# (Optional)
# key (the command you want to use) = [values] (extensions)
# [exec]
# feh = [\"jpg\", \"jpeg\", \"png\", \"gif\", \"svg\"]
# zathura = [\"pdf\"]

# The foreground color of directory, file and symlink.
# Pick one of the following:
#     Black
#     Red
#     Green
#     Yellow
#     Blue
#     Magenta
#     Cyan
#     White
#     LightBlack
#     LightRed
#     LightGreen
#     LightYellow
#     LightBlue
#     LightMagenta
#     LightCyan
#     LightWhite
#     Rgb(u8, u8, u8)
#     AnsiValue(u8)
# For more details, see https://docs.rs/termion/1.5.6/termion/color/index.html
[color]
dir_fg = \"LightCyan\"
file_fg = \"LightWhite\"
symlink_fg = \"LightYellow\"
";

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub default: Option<String>,
    pub exec: Option<HashMap<String, Vec<String>>>,
    pub color: ConfigColor,
    pub use_full_width: Option<bool>,
    pub item_name_length: Option<usize>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigColor {
    pub dir_fg: Colorname,
    pub file_fg: Colorname,
    pub symlink_fg: Colorname,
}

#[derive(Deserialize, Debug, Clone)]
pub enum Colorname {
    Black,        // 0
    Red,          // 1
    Green,        // 2
    Yellow,       // 3
    Blue,         // 4
    Magenta,      // 5
    Cyan,         // 6
    White,        // 7
    LightBlack,   // 8
    LightRed,     // 9
    LightGreen,   // 10
    LightYellow,  // 11
    LightBlue,    // 12
    LightMagenta, // 13
    LightCyan,    // 14
    LightWhite,   // 15
    Rgb(u8, u8, u8),
    AnsiValue(u8),
}

pub fn read_config() -> Result<Config, FxError> {
    let mut config = dirs::config_dir().unwrap_or_else(|| panic!("Cannot read config dir."));
    config.push(FX_CONFIG_DIR);
    config.push(CONFIG_FILE);
    let config = read_to_string(config.as_path());
    if let Ok(config) = config {
        let deserialized: Config = toml::from_str(&config)?;
        Ok(deserialized)
    } else {
        panic!("Cannot deserialize config file.");
    }
}

pub fn make_config_if_not_exists(config_file: &Path, trash_dir: &Path) -> Result<(), FxError> {
    if !trash_dir.exists() {
        std::fs::create_dir_all(trash_dir)?;
    }

    if !config_file.exists() {
        println!(
            "Config file not found: To set up, Please enter the default command to open a file. (e.g. nvim)"
        );

        let mut buffer = String::new();
        let stdin = std::io::stdin();
        stdin.read_line(&mut buffer)?;

        let mut trimmed = buffer.trim();
        if trimmed.is_empty() {
            match std::env::var("EDITOR") {
                Ok(_) => {
                    let config = CONFIG_EXAMPLE.replace("default = \"nvim\"", "# default = \"\"");
                    std::fs::write(&config_file, config)
                        .unwrap_or_else(|_| panic!("Cannot write new config file."));
                    if cfg!(target_os = "mac_os") {
                        println!(
                "Config file created.\nSee ~/Library/Application Support/felix/config.toml");
                    } else if cfg!(target_os = "windows") {
                        println!(
                            "Config file created.\nSee ~\\AppData\\Roaming\\felix\\config.toml"
                        );
                    } else {
                        println!("Config file created.\nSee ~/.config/felix/config.toml");
                    }
                }
                Err(_) => {
                    while trimmed.is_empty() {
                        println!("Cannot detect $EDITOR: Enter your default command.");
                        buffer = String::new();
                        std::io::stdin().read_line(&mut buffer)?;
                        trimmed = buffer.trim();
                    }
                    let config = CONFIG_EXAMPLE.replace("nvim", trimmed);
                    std::fs::write(&config_file, config)
                        .unwrap_or_else(|_| panic!("cannot write new config file."));
                    if cfg!(target_os = "mac_os") {
                        println!(
                "Default command set as [{}].\nSee ~/Library/Application Support/felix/config.toml",
                trimmed
            );
                    } else if cfg!(target_os = "windows") {
                        println!(
                "Default command set as [{}].\nSee ~\\AppData\\Roaming\\felix\\config.toml",
                trimmed
            );
                    } else {
                        println!(
                            "Default command set as [{}].\nSee ~/.config/felix/config.toml",
                            trimmed
                        );
                    }
                }
            }
        } else {
            let config = CONFIG_EXAMPLE.replace("nvim", trimmed);
            std::fs::write(&config_file, config)
                .unwrap_or_else(|_| panic!("cannot write new config file."));
            if cfg!(target_os = "mac_os") {
                println!(
                "Default command set as [{}].\nSee ~/Library/Application Support/felix/config.toml",
                trimmed
            );
            } else if cfg!(target_os = "windows") {
                println!(
                    "Default command set as [{}].\nSee ~\\AppData\\Roaming\\felix\\config.toml",
                    trimmed
                );
            } else {
                println!(
                    "Default command set as [{}].\nSee ~/.config/felix/config.toml",
                    trimmed
                );
            }
        }
    }
    Ok(())
}

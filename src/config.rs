use super::errors::FxError;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use crate::state::FX_CONFIG_DIR;

pub const CONFIG_FILE: &str = "config.toml";

pub const CONFIG_EXAMPLE: &str = "# (Optional) Default exec command when open files.
# If not set, will default to $EDITOR.
default = \"nvim\"

# (Optional)
# key (the command you want to use) = [values] (extensions)
# [exec]
# feh = [\"jpg\", \"jpeg\", \"png\", \"gif\", \"svg\"]
# zathura = [\"pdf\"]

# (Optional) Whether to use syntax highlighting in the preview mode.
# If not set, will default to false.
# syntax_highlight = false

# (Optional) Default theme for syntax highlighting.
# Pick one from the following:
#    Base16OceanDark
#    Base16EightiesDark
#    Base16MochaDark
#    Base16OceanLight
#    InspiredGitHub
#    SolarizedDark
#    SolarizedLight
# If not set, will default to \"Base16OceanDark\".
# default_theme = \"Base16OceanDark\"

# (Optional) Path to .tmtheme file for the syntax highlighting.
# If not set, default_theme will be used.
# theme_path = \"\"

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
    pub syntax_highlight: Option<bool>,
    pub default_theme: Option<DefaultTheme>,
    pub theme_path: Option<PathBuf>,
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

#[derive(Deserialize, Debug, Clone)]
pub enum DefaultTheme {
    Base16OceanDark,
    Base16EightiesDark,
    Base16MochaDark,
    Base16OceanLight,
    InspiredGitHub,
    SolarizedDark,
    SolarizedLight,
}

pub fn read_config() -> Result<Config, FxError> {
    let mut config = dirs::config_dir().unwrap_or_else(|| panic!("Cannot read config dir."));
    config.push(FX_CONFIG_DIR);
    config.push(CONFIG_FILE);
    let config = read_to_string(config.as_path())?;
    let deserialized: Config = toml::from_str(&config)?;
    Ok(deserialized)
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
                    println!("Config file created. See {}", config_file_path());
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
                    println!(
                        "Default command set as [{}]. See {}",
                        trimmed,
                        config_file_path()
                    );
                }
            }
        } else {
            let config = CONFIG_EXAMPLE.replace("nvim", trimmed);
            std::fs::write(&config_file, config)
                .unwrap_or_else(|_| panic!("cannot write new config file."));
            println!(
                "Default command set as [{}]. See {}",
                trimmed,
                config_file_path()
            );
        }
    }
    Ok(())
}

fn config_file_path() -> String {
    if cfg!(target_os = "mac_os") {
        "~/Library/Application Support/felix/config.toml".to_owned()
    } else if cfg!(target_os = "windows") {
        "~\\AppData\\Roaming\\felix\\config.toml".to_owned()
    } else {
        "~/.config/felix/config.toml".to_owned()
    }
}

use crate::errors::FxError;

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

pub const FELIX: &str = "felix";
const CONFIG_FILE: &str = "config.yaml";

#[allow(dead_code)]
const CONFIG_EXAMPLE: &str = r###"
# (Optional)
# Default exec command when open files.
# If not set, will default to $EDITOR.
# default: nvim

# (Optional)
# Whether to match the behavior of vim exit keybindings
# i.e. `ZQ` exits without cd to LWD (Last Working Directory) while `ZZ` cd to LWD
# match_vim_exit_behavior: false

# (Optional)
# key (the command you want to use when opening files): [values] (extensions)
# In the key, You can use arguments.
# exec:
#   zathura:
#     [pdf]
#  'feh -.':
#   [jpg, jpeg, png, gif, svg, hdr]

# (Optional)
# Whether to use syntax highlighting in the preview mode.
# If not set, will default to false.
syntax_highlight: true

# (Optional)
# Default theme for syntax highlighting.
# Pick one from the following:
#    Base16OceanDark
#    Base16EightiesDark
#    Base16MochaDark
#    Base16OceanLight
#    InspiredGitHub
#    SolarizedDark
#    SolarizedLight
# If not set, will default to \"Base16OceanDark\".
# default_theme: Base16OceanDark

# (Optional)
# Path to .tmtheme file for the syntax highlighting.
# If not set, default_theme will be used.
# theme_path: \"/home/kyohei/.config/felix/monokai.tmtheme\"

# The foreground color of directory, file and symlink.
# Pick one of the following:
#     Black           // 0
#     Red             // 1
#     Green           // 2
#     Yellow          // 3
#     Blue            // 4
#     Magenta         // 5
#     Cyan            // 6
#     White           // 7
#     LightBlack      // 8
#     LightRed        // 9
#     LightGreen      // 10
#     LightYellow     // 11
#     LightBlue       // 12
#     LightMagenta    // 13
#     LightCyan       // 14
#     LightWhite      // 15
#     Rgb(u8, u8, u8)
#     AnsiValue(u8)
# For more details, see https://docs.rs/termion/1.5.6/termion/color/index.html
color:
  dir_fg: LightCyan
  file_fg: LightWhite
  symlink_fg: LightYellow
"###;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub default: Option<String>,
    pub match_vim_exit_behavior: Option<bool>,
    pub exec: Option<BTreeMap<String, Vec<String>>>,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            default: Default::default(),
            match_vim_exit_behavior: Default::default(),
            exec: Default::default(),
            color: ConfigColor {
                dir_fg: Colorname::LightCyan,
                file_fg: Colorname::LightWhite,
                symlink_fg: Colorname::LightYellow,
            },
            syntax_highlight: Default::default(),
            default_theme: Default::default(),
            theme_path: Default::default(),
        }
    }
}

fn read_config(p: &Path) -> Result<Config, FxError> {
    let s = read_to_string(p)?;
    read_config_from_str(&s)
}

fn read_config_from_str(s: &str) -> Result<Config, FxError> {
    let deserialized: Config = serde_yaml::from_str(s)?;
    Ok(deserialized)
}

pub fn read_config_or_default() -> Result<Config, FxError> {
    //First, declare default config file path.
    let config_file_path = {
        let mut config_path = {
            let mut path = dirs::config_dir()
                .ok_or_else(|| FxError::Dirs("Cannot read the config directory.".to_string()))?;
            path.push(FELIX);
            path
        };
        config_path.push(CONFIG_FILE);
        config_path
    };

    //On macOS, felix looks for 2 paths:
    //First `$HOME/Library/Application Support/felix/config.yaml`,
    //and if it fails,
    //`$HOME/.config/felix/config.yaml`.
    let config_file_paths = if cfg!(target_os = "macos") {
        let alt_config_file_path = {
            let mut path = dirs::home_dir()
                .ok_or_else(|| FxError::Dirs("Cannot read the home directory.".to_string()))?;
            path.push(".config");
            path.push("FELIX");
            path.push(CONFIG_FILE);
            path
        };
        vec![config_file_path, alt_config_file_path]
    } else {
        vec![config_file_path]
    };

    let mut config_file: Option<PathBuf> = None;
    for p in config_file_paths {
        if p.exists() {
            config_file = Some(p);
            break;
        }
    }

    if let Some(config_file) = config_file {
        read_config(&config_file)
    } else {
        println!("Config file not found: felix launches with default configuration.");
        Ok(Config::default())
    }
}

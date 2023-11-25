use crate::errors::FxError;

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

pub const FELIX: &str = "felix";
const CONFIG_FILE: &str = "config.yaml";
const CONFIG_FILE_ANOTHER_EXT: &str = "config.yml";

#[allow(dead_code)]
const CONFIG_EXAMPLE: &str = r###"
# Default exec command when open files.
# If not set, will default to $EDITOR.
# default: nvim

# Whether to match the behavior of vim exit keybindings
# i.e. `ZQ` exits without cd to LWD (Last Working Directory) while `ZZ` cd to LWD
# match_vim_exit_behavior: false

# key (the command you want to use when opening files): [values] (extensions)
# In the key, You can use arguments.
# exec:
#   zathura:
#     [pdf]
#  'feh -.':
#   [jpg, jpeg, png, gif, svg, hdr]

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
# Default to LightCyan(dir), LightWhite(file), LightYellow(symlink) and Red(changed/untracked files in git repositories).
# color:
#   dir_fg: LightCyan
#   file_fg: LightWhite
#   symlink_fg: LightYellow
#   dirty_fg: Red
"###;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub default: Option<String>,
    pub match_vim_exit_behavior: Option<bool>,
    pub exec: Option<BTreeMap<String, Vec<String>>>,
    pub color: Option<ConfigColor>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConfigColor {
    pub dir_fg: Colorname,
    pub file_fg: Colorname,
    pub symlink_fg: Colorname,
    pub dirty_fg: Colorname,
}

impl Default for ConfigColor {
    fn default() -> Self {
        Self {
            dir_fg: Colorname::LightCyan,
            file_fg: Colorname::LightWhite,
            symlink_fg: Colorname::LightYellow,
            dirty_fg: Colorname::Red,
        }
    }
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

impl Default for Config {
    fn default() -> Self {
        Self {
            default: Default::default(),
            match_vim_exit_behavior: Default::default(),
            exec: Default::default(),
            color: Some(Default::default()),
        }
    }
}

fn read_config(p: &Path) -> Result<Config, FxError> {
    let s = read_to_string(p)?;
    let deserialized: Config = serde_yaml::from_str(&s)?;
    Ok(deserialized)
}

pub fn read_config_or_default() -> Result<Config, FxError> {
    //First, declare default config file path.
    let (config_file_path1, config_file_path2) = {
        let mut config_path = {
            let mut path = dirs::config_dir()
                .ok_or_else(|| FxError::Dirs("Cannot read the config directory.".to_string()))?;
            path.push(FELIX);
            path
        };
        let mut another = config_path.clone();
        config_path.push(CONFIG_FILE);
        another.push(CONFIG_FILE_ANOTHER_EXT);
        (config_path, another)
    };

    //On macOS, felix looks for 2 paths:
    //First `$HOME/Library/Application Support/felix/config.yaml(yml)`,
    //and if it fails,
    //`$HOME/.config/felix/config.yaml(yml)`.
    let config_file_paths = if cfg!(target_os = "macos") {
        let (alt_config_file_path1, alt_config_file_path2) = {
            let mut config_path = dirs::home_dir()
                .ok_or_else(|| FxError::Dirs("Cannot read the home directory.".to_string()))?;
            config_path.push(".config");
            config_path.push("FELIX");
            let mut another = config_path.clone();
            config_path.push(CONFIG_FILE);
            another.push(CONFIG_FILE_ANOTHER_EXT);
            (config_path, another)
        };
        vec![
            config_file_path1,
            config_file_path2,
            alt_config_file_path1,
            alt_config_file_path2,
        ]
    } else {
        vec![config_file_path1, config_file_path2]
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
        Ok(Config::default())
    }
}

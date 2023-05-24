use super::errors::FxError;
use super::state::FELIX;

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "config.yaml";

pub const CONFIG_EXAMPLE: &str = "# (Optional)
# Default exec command when open files.
# If not set, will default to $EDITOR.
# default: nvim

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
";

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub default: Option<String>,
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

pub fn read_config(p: &Path) -> Result<Config, FxError> {
    let s = read_to_string(p)?;
    read_config_from_str(&s)
}

pub fn read_config_from_str(s: &str) -> Result<Config, FxError> {
    let deserialized: Config = serde_yaml::from_str(s)?;
    Ok(deserialized)
}

pub fn read_or_create_config(config_paths: &Vec<PathBuf>) -> Result<Config, FxError> {
    let mut config_file: Option<PathBuf> = None;
    for p in config_paths {
        if p.exists() {
            config_file = Some(p.to_path_buf());
        }
    }

    if config_file.is_some() {
        read_config(&config_file.unwrap())
    } else {
        println!(
            "Config file not found: To set up, please enter the default command name to open a file. (e.g. nvim)\nIf you want to use the default $EDITOR, just press Enter."
        );

        let mut buffer = String::new();
        let stdin = std::io::stdin();
        stdin.read_line(&mut buffer)?;

        let mut trimmed = buffer.trim();
        if trimmed.is_empty() {
            match std::env::var("EDITOR") {
                Ok(_) => {
                    let config = CONFIG_EXAMPLE.replace("default = \"nvim\"", "# default = \"\"");
                    std::fs::write(&config_paths[0], config.clone())?;
                    println!("Config file created. See {}", config_file_path_output()?);
                    read_config_from_str(&config)
                }
                Err(_) => {
                    while trimmed.is_empty() {
                        println!("Cannot detect $EDITOR: Enter your default command.");
                        buffer = String::new();
                        std::io::stdin().read_line(&mut buffer)?;
                        trimmed = buffer.trim();
                    }
                    let config =
                        CONFIG_EXAMPLE.replace("# default: nvim", &format!("default: {}", trimmed));
                    std::fs::write(&config_paths[0], config.clone())?;
                    println!(
                        "Default command set as [{}]. See {}",
                        trimmed,
                        config_file_path_output()?
                    );
                    read_config_from_str(&config)
                }
            }
        } else {
            let config =
                CONFIG_EXAMPLE.replace("# default: nvim", &format!("default: {}", trimmed));
            std::fs::write(&config_paths[0], config.clone())?;
            println!(
                "Default command set as [{}]. See {}",
                trimmed,
                config_file_path_output()?
            );
            read_config_from_str(&config)
        }
    }
}

fn config_file_path() -> Result<PathBuf, FxError> {
    let mut config =
        dirs::config_dir().ok_or_else(|| FxError::Dirs("Cannot read config dir.".to_string()))?;
    config.push(FELIX);
    config.push(CONFIG_FILE);
    Ok(config)
}

fn config_file_path_output() -> Result<String, FxError> {
    if cfg!(target_os = "windows") {
        Ok("~\\AppData\\Roaming\\felix\\config.yaml".to_owned())
    } else {
        Ok(config_file_path()?.to_str().unwrap().to_owned())
    }
}

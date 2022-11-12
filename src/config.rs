use super::errors::FxError;
use super::state::FX_CONFIG_DIR;

use serde::Deserialize;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "config.yaml";

pub const CONFIG_EXAMPLE: &str = "# (Optional)
# Default exec command when open files.
# If not set, will default to $EDITOR.
# default: nvim

# (Optional)
# key (the command you want to use): [values] (extensions)
# exec:
#   feh:
#     [jpg, jpeg, png, gif, svg]
#   zathura:
#     [pdf]

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
    let config = read_to_string(p)?;
    let deserialized: Config = serde_yaml::from_str(&config)?;
    Ok(deserialized)
}

pub fn make_config_if_not_exists(config_file: &Path, trash_dir: &Path) -> Result<(), FxError> {
    if !trash_dir.exists() {
        std::fs::create_dir_all(trash_dir)?;
    }

    if !config_file.exists() {
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
                    std::fs::write(&config_file, config)?;
                    println!("Config file created. See {}", config_file_path_output()?);
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
                    std::fs::write(&config_file, config)?;
                    println!(
                        "Default command set as [{}]. See {}",
                        trimmed,
                        config_file_path_output()?
                    );
                }
            }
        } else {
            let config =
                CONFIG_EXAMPLE.replace("# default: nvim", &format!("default: {}", trimmed));
            std::fs::write(&config_file, config)?;
            println!(
                "Default command set as [{}]. See {}",
                trimmed,
                config_file_path_output()?
            );
        }
    }
    Ok(())
}

fn config_file_path() -> Result<PathBuf, FxError> {
    let mut config =
        dirs::config_dir().ok_or_else(|| FxError::Dirs("Cannot read config dir.".to_string()))?;
    config.push(FX_CONFIG_DIR);
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

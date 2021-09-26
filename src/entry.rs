use super::config::{Colorname, Config};
use std::process::Command;
use termion::color;

#[derive(PartialEq, PartialOrd, Eq, Ord, Copy, Clone, Debug)]
pub enum FileType {
    Directory,
    File,
}

pub struct EntryInfo {
    pub file_path: std::path::PathBuf,
    pub file_name: String,
    pub file_type: FileType,
}

impl EntryInfo {
    pub fn open_file(&self, config: &Config) {
        let path = &self.file_path;
        let extention = path
            .extension()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap();
        let ext_map = &config.exec;

        match ext_map.get(&extention) {
            Some(exec) => {
                let mut ex = Command::new(exec);
                ex.arg(path).status().expect("failed");
            }
            None => {
                let default = ext_map.get("default").unwrap();
                let mut ex = Command::new(default);
                ex.arg(path).status().expect("failed");
            }
        }
    }

    pub fn print(&self, config: &Config) {
        if self.file_type == FileType::File {
            match config.color.file_fg {
                Colorname::AnsiValue(n) => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::AnsiValue(n)),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Black => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Black),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Blue => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Blue),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Cyan => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Cyan),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Green => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Green),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightBlack => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightBlack),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightBlue => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightBlue),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightCyan => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightCyan),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightGreen => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightGreen),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightMagenta => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightMagenta),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightRed => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightRed),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightWhite => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightWhite),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightYellow => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightYellow),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Magenta => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Magenta),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Red => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Red),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Rgb(x, y, z) => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Rgb(x, y, z)),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::White => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::White),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Yellow => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Yellow),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
            }
        } else {
            match config.color.dir_fg {
                Colorname::AnsiValue(n) => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::AnsiValue(n)),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Black => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Black),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Blue => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Blue),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Cyan => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Cyan),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Green => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Green),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightBlack => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightBlack),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightBlue => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightBlue),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightCyan => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightCyan),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightGreen => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightGreen),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightMagenta => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightMagenta),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightRed => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightRed),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightWhite => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightWhite),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightYellow => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightYellow),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Magenta => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Magenta),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Red => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Red),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Rgb(x, y, z) => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Rgb(x, y, z)),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::White => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::White),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Yellow => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Yellow),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
            }
        }
    }
}

use super::config::Colorname;
use super::config::Config;
use chrono::prelude::*;
use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;
use termion::{color, cursor, style};

pub const STARTING_POINT: u16 = 3;
pub const DOWN_ARROW: char = '\u{21D3}';
pub const RIGHT_ARROW: char = '\u{21D2}';
pub const CONFIG_FILE: &str = "fm/config.toml";
pub const TRASH: &str = "fm/trash";
pub const NAME_MAX_LEN: usize = 30;
pub const TIME_START_POS: u16 = 32;
pub const CONFIRMATION: &str = "Are you sure to empty the trash directory? (if yes: y)";

macro_rules! print_entry {
    ($color: expr, $name: expr, $time: expr) => {
        let len = TIME_START_POS - $name.len() as u16;
        print!(
            "{}{}{}{}{}",
            $color,
            $name,
            cursor::Right(len),
            $time,
            color::Fg(color::Reset)
        );
    };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    Directory,
    File,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntryInfo {
    pub file_type: FileType,
    pub file_name: String,
    pub file_path: std::path::PathBuf,
    pub modified: Option<String>,
}

impl EntryInfo {
    //Open file according to config.toml.
    pub fn open_file(&self, config: &Config) {
        let path = &self.file_path;
        let ext_map = &config.exec;
        let extention = path.extension();
        let default = ext_map.get("default").unwrap();
        match extention {
            Some(extention) => {
                let ext = extention.to_os_string().into_string().unwrap();
                match ext_map.get(&ext) {
                    Some(exec) => {
                        let mut ex = Command::new(exec);
                        ex.arg(path).status().expect("failed");
                    }
                    None => {
                        let mut ex = Command::new(default);
                        ex.arg(path).status().expect("failed");
                    }
                }
            }

            None => {
                let mut ex = Command::new(default);
                ex.arg(path).status().expect("failed");
            }
        }
    }

    //Move selected file or directory recursively to trash_dir(by default ~/.config/fm/trash).
    pub fn remove(&self, trash_dir: &PathBuf) -> fs_extra::error::Result<()> {
        let options = fs_extra::dir::CopyOptions::new();
        let arr = [&self.file_path.as_path()];
        match fs_extra::move_items(&arr, trash_dir, &options) {
            Ok(_) => Ok(()),
            Err(_) => panic!("cannot remove item."),
        }
    }

    //Print name of file or directory.
    fn print(&self, config: &Config) {
        let name = &self.file_name;
        let time = format_time(&self.modified);
        let color = match &self.file_type {
            &FileType::File => &config.color.file_fg,
            &FileType::Directory => &config.color.dir_fg,
        };
        match color {
            Colorname::AnsiValue(n) => {
                print_entry!(color::Fg(color::AnsiValue(*n)), name, time);
            }
            Colorname::Black => {
                print_entry!(color::Fg(color::Black), name, time);
            }
            Colorname::Blue => {
                print_entry!(color::Fg(color::Blue), name, time);
            }
            Colorname::Cyan => {
                print_entry!(color::Fg(color::Cyan), name, time);
            }
            Colorname::Green => {
                print_entry!(color::Fg(color::Green), name, time);
            }
            Colorname::LightBlack => {
                print_entry!(color::Fg(color::LightBlack), name, time);
            }
            Colorname::LightBlue => {
                print_entry!(color::Fg(color::LightBlue), name, time);
            }
            Colorname::LightCyan => {
                print_entry!(color::Fg(color::LightCyan), name, time);
            }
            Colorname::LightGreen => {
                print_entry!(color::Fg(color::LightGreen), name, time);
            }
            Colorname::LightMagenta => {
                print_entry!(color::Fg(color::LightMagenta), name, time);
            }
            Colorname::LightRed => {
                print_entry!(color::Fg(color::LightRed), name, time);
            }
            Colorname::LightWhite => {
                print_entry!(color::Fg(color::LightWhite), name, time);
            }
            Colorname::LightYellow => {
                print_entry!(color::Fg(color::LightYellow), name, time);
            }
            Colorname::Magenta => {
                print_entry!(color::Fg(color::Magenta), name, time);
            }
            Colorname::Red => {
                print_entry!(color::Fg(color::Red), name, time);
            }
            Colorname::Rgb(x, y, z) => {
                print_entry!(color::Fg(color::Rgb(*x, *y, *z)), name, time);
            }
            Colorname::White => {
                print_entry!(color::Fg(color::White), name, time);
            }
            Colorname::Yellow => {
                print_entry!(color::Fg(color::Yellow), name, time);
            }
        }
    }
}

fn make_parent_dir(p: PathBuf) -> EntryInfo {
    return EntryInfo {
        file_type: FileType::Directory,
        file_name: String::from("../"),
        file_path: p,
        modified: None,
    };
}

fn make_entry(dir: fs::DirEntry) -> EntryInfo {
    let path = dir.path();
    let metadata = fs::metadata(&path).unwrap();
    let sometime = metadata.modified();
    let time = if sometime.is_ok() {
        let chrono_time: DateTime<Local> = DateTime::from(sometime.unwrap());
        Some(chrono_time.to_rfc3339_opts(SecondsFormat::Secs, false))
    } else {
        None
    };

    let name = path
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap();

    return EntryInfo {
        //todo: Is this chain even necessary?
        file_type: if dir.path().is_file() {
            FileType::File
        } else {
            FileType::Directory
        },
        file_name: if name.len() > NAME_MAX_LEN {
            let name = format!("{}..", &name[0..=NAME_MAX_LEN - 2]);
            name
        } else {
            name
        },
        file_path: path,
        modified: time,
    };
}

pub fn push_entries(p: &PathBuf) -> Result<Vec<EntryInfo>, Error> {
    let mut entry_v = vec![];

    match p.parent() {
        Some(parent_p) => {
            let parent_dir = make_parent_dir(parent_p.to_path_buf());
            entry_v.push(parent_dir);
        }
        None => {}
    }
    for entry in fs::read_dir(p)? {
        let e = entry?;
        let entry = make_entry(e);
        entry_v.push(entry);
    }
    entry_v.sort();
    Ok(entry_v)
}

pub fn make_config(config_file: &PathBuf, trash_dir: &PathBuf) -> std::io::Result<()> {
    if !config_file.exists() {
        fs::File::create(config_file)?;
    }

    if !trash_dir.exists() {
        fs::create_dir_all(trash_dir)?;
    }

    Ok(())
}

fn format_time(time: &Option<String>) -> String {
    match time {
        Some(datetime) => format!("{} {}", &datetime[0..10], &datetime[11..16]),
        None => "".to_string(),
    }
}

pub fn list_up(config: &Config, p: &PathBuf, v: &std::vec::Vec<EntryInfo>, skip_number: u16) {
    //Show current directory path
    println!(
        " {}{}{}{}{}{}{}",
        style::Bold,
        color::Bg(color::Cyan),
        color::Fg(color::Black),
        p.display(),
        style::Reset,
        color::Bg(color::Reset),
        color::Fg(color::Reset)
    );

    //Show arrow
    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);

    let (_, row) = termion::terminal_size().unwrap();
    let len = v.len();

    //if lists exceeds max-row
    if row > STARTING_POINT - 1 && v.len() > (row - STARTING_POINT) as usize - 1 {
        let mut row_count = 0;
        for (i, entry) in v.iter().enumerate() {
            let i = i as u16;

            if i < skip_number {
                continue;
            }

            print!("{}", cursor::Goto(3, i + STARTING_POINT - skip_number));

            if row_count == row - STARTING_POINT {
                print!(
                    "  {}{}{}lines {}-{}({}){}{}",
                    cursor::Left(2),
                    color::Bg(color::LightWhite),
                    color::Fg(color::Black),
                    skip_number,
                    row - STARTING_POINT + skip_number,
                    len,
                    color::Bg(color::Reset),
                    color::Fg(color::Reset)
                );
                break;
            } else {
                entry.print(config);
                row_count += 1;
            }
        }
    } else {
        for (i, entry) in v.iter().enumerate() {
            let i = i as u16;
            print!("{}", cursor::Goto(3, i + STARTING_POINT));
            entry.print(config);
        }
    }
}

use super::config::*;
use super::functions::*;
use chrono::prelude::*;
use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;
use termion::{color, cursor};

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
        let mut result: String;
        let chars: Vec<char> = $name.chars().collect();
        let name = if chars.len() > NAME_MAX_LEN {
            result = chars.iter().take(NAME_MAX_LEN - 2).collect::<String>();
            result.push_str("..");
            &result
        } else {
            $name
        };
        print!(
            "{}{}{}{}{}{}",
            $color,
            name,
            cursor::Left(34),
            cursor::Right(34),
            $time,
            color::Fg(color::Reset)
        );
    };
}
#[derive(Clone)]
pub struct Items {
    pub list: Vec<ItemInfo>,
    pub item_buf: Option<ItemInfo>,
    pub trash_dir: PathBuf,
    pub config: Config,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ItemInfo {
    pub file_type: FileType,
    pub file_name: String,
    pub file_path: std::path::PathBuf,
    pub modified: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    Directory,
    File,
}

impl Default for Items {
    fn default() -> Self {
        Items {
            list: Vec::new(),
            item_buf: None,
            trash_dir: PathBuf::new(),
            config: read_config().unwrap_or_else(|| panic!("cannot read config file.")),
        }
    }
}

impl Items {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn get_item(&self, index: usize) -> &ItemInfo {
        &self
            .list
            .get(index)
            .unwrap_or_else(|| panic!("cannot choose item."))
    }

    pub fn open_file(&self, index: usize) {
        let item = self.get_item(index);
        let path = &item.file_path;
        let ext_map = &self.config.exec;
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
    pub fn remove(&self, index: usize) {
        let options = fs_extra::dir::CopyOptions::new();
        let arr = [&self.get_item(index).file_path.as_path()];
        fs_extra::move_items(&arr, &self.trash_dir, &options)
            .unwrap_or_else(|_| panic!("cannot remove item."));
    }

    pub fn print(&self, index: usize) {
        let item = &self.get_item(index);
        let name = &item.file_name;
        let time = format_time(&item.modified);
        let color = match &item.file_type {
            &FileType::File => &self.config.color.file_fg,
            &FileType::Directory => &self.config.color.dir_fg,
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

    pub fn list_up(&self, skip_number: u16) {
        //Show arrow
        print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);

        let (_, row) = termion::terminal_size().unwrap();
        let len = self.list.len();

        //if lists exceeds max-row
        if row > STARTING_POINT - 1 && len > (row - STARTING_POINT) as usize - 1 {
            let mut row_count = 0;
            for (i, _) in self.list.iter().enumerate() {
                if i < skip_number as usize {
                    continue;
                }

                print!(
                    "{}",
                    cursor::Goto(3, i as u16 + STARTING_POINT - skip_number)
                );

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
                    self.print(i);
                    row_count += 1;
                }
            }
        } else {
            for (i, _) in self.list.iter().enumerate() {
                print!("{}", cursor::Goto(3, i as u16 + STARTING_POINT));
                self.print(i);
            }
        }
    }

    pub fn update_list(&mut self, path: &PathBuf) {
        self.list = push_entries(path).unwrap();
    }
}

fn make_parent_dir(p: PathBuf) -> ItemInfo {
    return ItemInfo {
        file_type: FileType::Directory,
        file_name: String::from("../"),
        file_path: p,
        modified: None,
    };
}

fn make_entry(dir: fs::DirEntry) -> ItemInfo {
    let path = dir.path();
    let metadata =
        fs::metadata(&path).unwrap_or_else(|_| panic!("cannot read metadata of directory."));
    let sometime = metadata.modified();
    let time = if let Ok(time) = sometime {
        let chrono_time: DateTime<Local> = DateTime::from(time);
        Some(chrono_time.to_rfc3339_opts(SecondsFormat::Secs, false))
    } else {
        None
    };

    let name = dir
        .file_name()
        .into_string()
        .unwrap_or_else(|_| panic!("failed to get file name."));

    return ItemInfo {
        //todo: Is this chain even necessary?
        file_type: if dir.path().is_file() {
            FileType::File
        } else {
            FileType::Directory
        },
        file_name: name,
        file_path: path,
        modified: time,
    };
}

pub fn push_entries(p: &PathBuf) -> Result<Vec<ItemInfo>, Error> {
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

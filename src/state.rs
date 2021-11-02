use super::config::*;
use super::functions::*;
use chrono::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use termion::{color, cursor};

pub const STARTING_POINT: u16 = 3;
pub const DOWN_ARROW: char = '\u{21D3}';
pub const RIGHT_ARROW: char = '\u{21D2}';
pub const FM_CONFIG_DIR: &str = "fm";
pub const CONFIG_FILE: &str = "config.toml";
pub const TRASH: &str = "trash";
pub const NAME_MAX_LEN: usize = 30;
pub const TIME_START_POS: u16 = 32;
pub const WHEN_EMPTY: &str = "Are you sure to empty the trash directory? (if yes: y)";
pub const WHEN_DELETE: &str = "Are you sure to cut this item? (if yes: y)";

macro_rules! print_item {
    ($color: expr, $name: expr, $time: expr, $selected: expr) => {
        if *($selected) {
            print!(
                "{}{}{}{}{}{}{}{}",
                $color,
                color::Bg(color::LightBlack),
                $name,
                color::Bg(color::Reset),
                cursor::Left(60),
                cursor::Right(34),
                $time,
                color::Fg(color::Reset),
            );
        } else {
            print!(
                "{}{}{}{}{}{}",
                $color,
                $name,
                cursor::Left(60),
                cursor::Right(34),
                $time,
                color::Fg(color::Reset)
            );
        }
    };
}
#[derive(Clone)]
pub struct State {
    pub list: Vec<ItemInfo>,
    pub item_buf: Option<ItemInfo>,
    pub trash_dir: PathBuf,
    pub colors: (Colorname, Colorname, Colorname),
    pub default: String,
    pub commands: HashMap<String, String>,
    pub warning: bool,
    pub sort_by: SortKey,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ItemInfo {
    pub file_type: FileType,
    pub file_name: String,
    pub file_path: std::path::PathBuf,
    pub modified: Option<String>,
    pub selected: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    Directory,
    File,
    Symlink,
}

impl Default for State {
    fn default() -> Self {
        let config = read_config().unwrap();
        State {
            list: Vec::new(),
            item_buf: None,
            trash_dir: PathBuf::new(),
            colors: (
                config.color.dir_fg,
                config.color.file_fg,
                config.color.symlink_fg,
            ),
            default: config.default,
            commands: to_extension_map(&config.exec),
            warning: config.warning,
            sort_by: config.sort_by,
        }
    }
}

impl State {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn get_item(&self, index: usize) -> &ItemInfo {
        &self
            .list
            .get(index)
            .unwrap_or_else(|| panic!("cannot choose item."))
    }

    pub fn open_file(&self, index: usize) -> std::io::Result<ExitStatus> {
        let item = self.get_item(index);
        let path = &item.file_path;
        let map = &self.commands;
        let extention = path.extension();

        match extention {
            Some(extention) => {
                let ext = extention.to_os_string().into_string().unwrap();
                match map.get(&ext) {
                    Some(command) => {
                        let mut ex = Command::new(command);
                        ex.arg(path).status()
                    }
                    None => {
                        let mut ex = Command::new(&self.default);
                        ex.arg(path).status()
                    }
                }
            }

            None => {
                let mut ex = Command::new(&self.default);
                ex.arg(path).status()
            }
        }
    }

    pub fn remove_file(&mut self, index: usize) -> std::io::Result<()> {
        //prepare from and to for copy
        let item = self.get_item(index).clone();
        let from = &item.file_path;

        let name = &item.file_name;
        let mut rename = Local::now().timestamp().to_string();
        rename.push('_');
        rename.push_str(name);

        let to = self.trash_dir.join(&rename);

        //copy
        std::fs::copy(from, &to)?;

        //copy original information to item_buf
        self.to_item_buf(&item, to, rename);

        //remove original
        std::fs::remove_file(from)?;

        let _ = self.list.remove(index);
        Ok(())
    }

    pub fn remove_dir(&mut self, index: usize) -> std::io::Result<()> {
        let mut trash_name = String::new();
        let mut base: usize = 0;
        let mut trash_path: std::path::PathBuf = PathBuf::new();
        let mut target: PathBuf;
        let item = self.get_item(index).clone();

        let mut i = 0;
        for entry in walkdir::WalkDir::new(&item.file_path).sort_by_key(|x| x.path().to_path_buf())
        {
            let entry = entry?;
            if i == 0 {
                base = entry.path().iter().count();

                trash_name = chrono::Local::now().timestamp().to_string();
                trash_name.push('_');
                trash_name.push_str(entry.file_name().to_str().unwrap());
                trash_path = self.trash_dir.join(&trash_name);
                std::fs::create_dir(&self.trash_dir.join(&trash_path))?;

                i += 1;
                continue;
            } else {
                target = entry.path().iter().skip(base).collect();
                target = trash_path.join(target);
                if entry.file_type().is_dir() {
                    std::fs::create_dir(&target)?;
                    continue;
                }

                if let Some(parent) = entry.path().parent() {
                    if !parent.exists() {
                        std::fs::create_dir(parent)?;
                    }
                }

                std::fs::copy(entry.path(), &target)?;
            }
        }

        //copy original information to item_buf
        self.to_item_buf(&item, trash_path, trash_name);

        //remove original
        std::fs::remove_dir_all(&item.file_path)?;

        let _ = self.list.remove(index);

        Ok(())
    }

    fn to_item_buf(&mut self, item: &ItemInfo, file_path: PathBuf, file_name: String) {
        let mut buf = item.clone();
        buf.file_path = file_path;
        buf.file_name = file_name;
        self.item_buf = Some(buf);
    }

    pub fn paste_file(&mut self, current_dir: &PathBuf) -> std::io::Result<()> {
        let item = &self.item_buf.clone();
        match item {
            None => Ok(()),
            Some(item) => {
                if item.file_path.parent() == Some(&self.trash_dir) {
                    let mut item = item.clone();
                    let rename = item.file_name.chars().skip(11).collect();
                    item.file_name = rename;
                    let rename = rename_file(&item, &self);
                    std::fs::copy(&item.file_path, current_dir.join(&rename))?;
                } else {
                    let rename = rename_file(&item, &self);
                    std::fs::copy(&item.file_path, current_dir.join(&rename))?;
                }

                self.update_list(current_dir);
                Ok(())
            }
        }
    }

    pub fn paste_dir(&mut self, current_dir: &PathBuf) -> std::io::Result<()> {
        let mut base: usize = 0;
        let mut target: PathBuf = PathBuf::new();
        let buf = self.item_buf.clone();
        let mut buf = buf.unwrap();
        let original_path = &(buf).file_path;

        let mut i = 0;
        for entry in walkdir::WalkDir::new(&original_path).sort_by_key(|x| x.path().to_path_buf()) {
            let entry = entry.unwrap();
            if i == 0 {
                base = entry.path().iter().count();

                let parent = &original_path.parent().unwrap();
                if parent == &self.trash_dir {
                    let rename = buf.file_name.chars().skip(11).collect();
                    buf.file_name = rename;

                    let rename = rename_dir(&buf, &self);
                    target = current_dir.join(rename);
                    std::fs::create_dir(&target)?;
                } else {
                    let rename = rename_dir(&buf, &self);
                    target = current_dir.join(rename);
                    std::fs::create_dir(&target)?;
                }
                i += 1;
                continue;
            } else {
                let child: PathBuf = entry.path().iter().skip(base).collect();
                let child = target.join(child);

                if entry.file_type().is_dir() {
                    std::fs::create_dir(child)?;
                    continue;
                } else {
                    if let Some(parent) = entry.path().parent() {
                        if !parent.exists() {
                            std::fs::create_dir(parent)?;
                        }
                    }
                }

                std::fs::copy(entry.path(), &child)?;
            }
        }
        self.update_list(current_dir);
        Ok(())
    }

    pub fn print(&self, index: usize) {
        let item = &self.get_item(index);
        let chars: Vec<char> = item.file_name.chars().collect();
        let name = if chars.len() > NAME_MAX_LEN {
            let mut result = chars.iter().take(NAME_MAX_LEN - 2).collect::<String>();
            result.push_str("..");
            result
        } else {
            item.file_name.clone()
        };
        let time = format_time(&item.modified);
        let selected = &item.selected;
        let color = match &item.file_type {
            &FileType::Directory => &self.colors.0,
            &FileType::File => &self.colors.1,
            &FileType::Symlink => &self.colors.2,
        };
        match color {
            Colorname::AnsiValue(n) => {
                print_item!(color::Fg(color::AnsiValue(*n)), name, time, selected);
            }
            Colorname::Black => {
                print_item!(color::Fg(color::Black), name, time, selected);
            }
            Colorname::Blue => {
                print_item!(color::Fg(color::Blue), name, time, selected);
            }
            Colorname::Cyan => {
                print_item!(color::Fg(color::Cyan), name, time, selected);
            }
            Colorname::Green => {
                print_item!(color::Fg(color::Green), name, time, selected);
            }
            Colorname::LightBlack => {
                print_item!(color::Fg(color::LightBlack), name, time, selected);
            }
            Colorname::LightBlue => {
                print_item!(color::Fg(color::LightBlue), name, time, selected);
            }
            Colorname::LightCyan => {
                print_item!(color::Fg(color::LightCyan), name, time, selected);
            }
            Colorname::LightGreen => {
                print_item!(color::Fg(color::LightGreen), name, time, selected);
            }
            Colorname::LightMagenta => {
                print_item!(color::Fg(color::LightMagenta), name, time, selected);
            }
            Colorname::LightRed => {
                print_item!(color::Fg(color::LightRed), name, time, selected);
            }
            Colorname::LightWhite => {
                print_item!(color::Fg(color::LightWhite), name, time, selected);
            }
            Colorname::LightYellow => {
                print_item!(color::Fg(color::LightYellow), name, time, selected);
            }
            Colorname::Magenta => {
                print_item!(color::Fg(color::Magenta), name, time, selected);
            }
            Colorname::Red => {
                print_item!(color::Fg(color::Red), name, time, selected);
            }
            Colorname::Rgb(x, y, z) => {
                print_item!(color::Fg(color::Rgb(*x, *y, *z)), name, time, selected);
            }
            Colorname::White => {
                print_item!(color::Fg(color::White), name, time, selected);
            }
            Colorname::Yellow => {
                print_item!(color::Fg(color::Yellow), name, time, selected);
            }
        }
    }

    pub fn list_up(&self, skip_number: u16) {
        let (_, row) = termion::terminal_size().unwrap();
        let len = self.list.len();

        //if lists exceed max-row
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
        self.list = push_items(path, &self.sort_by).unwrap();
    }

    pub fn reset_selection(&mut self) {
        for mut item in self.list.iter_mut() {
            item.selected = false;
        }
    }
}

fn make_parent_dir(p: PathBuf) -> ItemInfo {
    return ItemInfo {
        file_type: FileType::Directory,
        file_name: String::from("../"),
        file_path: p,
        modified: None,
        selected: false,
    };
}

fn make_item(dir: fs::DirEntry) -> ItemInfo {
    let path = dir.path();

    let time = if let Ok(metadata) = fs::metadata(&path) {
        let sometime = metadata.modified().unwrap();
        let chrono_time: DateTime<Local> = DateTime::from(sometime);
        Some(chrono_time.to_rfc3339_opts(SecondsFormat::Secs, false))
    } else {
        None
    };

    let name = dir
        .file_name()
        .into_string()
        .unwrap_or_else(|_| panic!("failed to get file name."));

    return ItemInfo {
        file_type: if path.is_symlink() {
            FileType::Symlink
        } else if path.is_file() {
            FileType::File
        } else {
            FileType::Directory
        },
        file_name: name,
        file_path: path,
        modified: time,
        selected: false,
    };
}

pub fn push_items(p: &PathBuf, key: &SortKey) -> Result<Vec<ItemInfo>, Error> {
    let mut result = Vec::new();
    let mut dir_v = Vec::new();
    let mut file_v = Vec::new();

    match p.parent() {
        Some(parent_p) => {
            let parent_dir = make_parent_dir(parent_p.to_path_buf());
            result.push(parent_dir);
        }
        None => {}
    }
    for entry in fs::read_dir(p)? {
        let e = entry?;
        let entry = make_item(e);
        match entry.file_type {
            FileType::Directory => dir_v.push(entry),
            FileType::File | FileType::Symlink => file_v.push(entry),
        }
    }

    match key {
        SortKey::Name => {
            dir_v.sort_by(|a, b| natord::compare(&a.file_name, &b.file_name));
            file_v.sort_by(|a, b| natord::compare(&a.file_name, &b.file_name));
        }
        SortKey::Time => {
            dir_v.sort_by(|a, b| b.modified.partial_cmp(&a.modified).unwrap());
            file_v.sort_by(|a, b| b.modified.partial_cmp(&a.modified).unwrap());
        }
    }

    result.append(&mut dir_v);
    result.append(&mut file_v);
    Ok(result)
}

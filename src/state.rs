use super::config::*;
use super::errors::MyError;
use super::functions::*;
use super::nums::*;
use super::session::*;
use chrono::prelude::*;
use log::error;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::fs::DirEntry;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use termion::{clear, color, cursor, style};

pub const STARTING_POINT: u16 = 3;
pub const TIME_WIDTH: u16 = 17;
pub const DOWN_ARROW: char = '\u{21D3}';
pub const RIGHT_ARROW: char = '\u{21D2}';
pub const FX_CONFIG_DIR: &str = "felix";
pub const CONFIG_FILE: &str = "config.toml";
pub const TRASH: &str = "trash";
pub const WHEN_EMPTY: &str = "Are you sure to empty the trash directory? (if yes: y)";

macro_rules! print_item {
    ($color: expr, $name: expr, $time: expr, $selected: expr, $layout: expr) => {
        if *($selected) {
            print!(
                "{}{}{}{}{}{} {}{}{}",
                $color,
                style::Invert,
                $name,
                style::Reset,
                cursor::Left(100),
                cursor::Right($layout.time_start_pos - 1),
                style::Invert,
                $time,
                style::Reset
            );
        } else {
            print!(
                "{}{}{}{} {}{}",
                $color,
                $name,
                cursor::Left(100),
                cursor::Right($layout.time_start_pos - 1),
                $time,
                color::Fg(color::Reset)
            );
        }
        if $layout.terminal_column > $layout.time_start_pos + 16 {
            print!(
                "{}",
                (0..($layout.terminal_column - $layout.time_start_pos - 16))
                    .map(|_| ' ')
                    .collect::<String>()
            );
        }
    };
}
#[derive(Clone)]
pub struct State {
    pub list: Vec<ItemInfo>,
    pub registered: Vec<ItemInfo>,
    pub current_dir: PathBuf,
    pub trash_dir: PathBuf,
    pub colors: (Colorname, Colorname, Colorname),
    pub default: String,
    pub commands: HashMap<String, String>,
    pub sort_by: SortKey,
    pub layout: Layout,
    pub show_hidden: bool,
    pub rust_log: Option<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ItemInfo {
    pub file_type: FileType,
    pub file_name: String,
    pub file_path: std::path::PathBuf,
    pub symlink_dir_path: Option<PathBuf>,
    pub file_size: u64,
    pub file_ext: Option<OsString>,
    pub modified: Option<String>,
    pub selected: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    Directory,
    File,
    Symlink,
}

#[derive(Clone)]
pub struct Layout {
    pub terminal_row: u16,
    pub terminal_column: u16,
    pub name_max_len: usize,
    pub time_start_pos: u16,
    pub option_name_len: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        let config = read_config().unwrap();
        let session = read_session().unwrap();
        let (column, row) = termion::terminal_size().unwrap();
        if column < 21 {
            error!("Too small terminal size.");
            panic!("Panic due to terminal size (less than 21 columns).")
        };
        let mut time_start: u16;
        let mut name_max: usize;
        match config.use_full_width {
            Some(true) => {
                time_start = column - 16;
                name_max = (time_start - 3).into();
            }
            Some(false) | None => match config.item_name_length {
                Some(option_max) => {
                    time_start = option_max as u16 + 3;
                    name_max = option_max;
                }
                None => {
                    time_start = if column >= 49 {
                        33
                    } else {
                        column - TIME_WIDTH
                    };
                    name_max = if column >= 49 {
                        30
                    } else {
                        (time_start - 3).into()
                    };
                }
            },
        }

        let required = time_start + TIME_WIDTH - 1;
        if required > column {
            let diff = required - column;
            name_max -= diff as usize;
            time_start -= diff;
        }

        State {
            list: Vec::new(),
            registered: Vec::new(),
            current_dir: PathBuf::new(),
            trash_dir: PathBuf::new(),
            colors: (
                config.color.dir_fg,
                config.color.file_fg,
                config.color.symlink_fg,
            ),
            default: config.default,
            commands: to_extension_map(&config.exec),
            sort_by: session.sort_by,
            layout: Layout {
                terminal_row: row,
                terminal_column: column,
                name_max_len: name_max,
                time_start_pos: time_start,
                option_name_len: config.item_name_length,
            },
            show_hidden: session.show_hidden,
            rust_log: std::env::var("RUST_LOG").ok(),
        }
    }
}

impl State {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn get_item(&self, index: usize) -> Result<&ItemInfo, MyError> {
        self.list.get(index).ok_or_else(|| {
            MyError::IoError(std::io::Error::new(
                ErrorKind::NotFound,
                "Cannot choose item.",
            ))
        })
    }

    pub fn open_file(&self, index: usize) -> Result<ExitStatus, MyError> {
        let item = self.get_item(index)?;
        let path = &item.file_path;
        let map = &self.commands;
        let extention = path.extension();

        match extention {
            Some(extention) => {
                let ext = extention.to_ascii_lowercase().into_string().unwrap();
                match map.get(&ext) {
                    Some(command) => {
                        let mut ex = Command::new(command);
                        ex.arg(path).status().map_err(MyError::IoError)
                    }
                    None => {
                        let mut ex = Command::new(&self.default);
                        ex.arg(path).status().map_err(MyError::IoError)
                    }
                }
            }

            None => {
                let mut ex = Command::new(&self.default);
                ex.arg(path).status().map_err(MyError::IoError)
            }
        }
    }

    pub fn remove_and_yank_file(&mut self, item: ItemInfo) -> Result<(), MyError> {
        //prepare from and to for copy
        let from = &item.file_path;

        if item.file_type == FileType::Symlink && !from.exists() {
            match Command::new("rm").arg(from).status() {
                Ok(_) => Ok(()),
                Err(e) => Err(MyError::IoError(e)),
            }
        } else {
            let name = &item.file_name;
            let mut rename = Local::now().timestamp().to_string();
            rename.push('_');
            rename.push_str(name);

            let to = self.trash_dir.join(&rename);

            //copy
            if std::fs::copy(from, &to).is_err() {
                return Err(MyError::FileCopyError {
                    msg: format!("Cannot copy item: {:?}", from),
                });
            }

            self.to_registered_mut(&item, to, rename);

            //remove original
            if std::fs::remove_file(from).is_err() {
                return Err(MyError::FileRemoveError {
                    msg: format!("Cannot Remove item: {:?}", from),
                });
            }

            Ok(())
        }
    }

    pub fn remove_and_yank_dir(&mut self, item: ItemInfo) -> Result<(), MyError> {
        let mut trash_name = String::new();
        let mut base: usize = 0;
        let mut trash_path: std::path::PathBuf = PathBuf::new();
        let mut target: PathBuf;

        let len = walkdir::WalkDir::new(&item.file_path).into_iter().count();
        let unit = len / 5;
        for (i, entry) in walkdir::WalkDir::new(&item.file_path)
            .into_iter()
            .enumerate()
        {
            if i > unit * 4 {
                print_process("[»»»»-]");
            } else if i > unit * 3 {
                print_process("[»»»--]");
            } else if i > unit * 2 {
                print_process("[»»---]");
            } else if i > unit {
                print_process("[»----]");
            } else if i == 0 {
                print_process(" [-----]");
            }
            let entry = entry?;
            let entry_path = entry.path();
            if i == 0 {
                base = entry_path.iter().count();

                trash_name = chrono::Local::now().timestamp().to_string();
                trash_name.push('_');
                let file_name = entry.file_name().to_str();
                if file_name == None {
                    return Err(MyError::UTF8Error {
                        msg: "Cannot convert filename to UTF-8.".to_string(),
                    });
                }
                trash_name.push_str(file_name.unwrap());
                trash_path = self.trash_dir.join(&trash_name);
                std::fs::create_dir(&self.trash_dir.join(&trash_path))?;

                continue;
            } else {
                target = entry_path.iter().skip(base).collect();
                target = trash_path.join(target);
                if entry.file_type().is_dir() {
                    std::fs::create_dir_all(&target)?;
                    continue;
                }

                if let Some(parent) = entry_path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir(parent)?;
                    }
                }

                if std::fs::copy(entry_path, &target).is_err() {
                    return Err(MyError::FileCopyError {
                        msg: format!("Cannot copy item: {:?}", entry_path),
                    });
                }
            }
        }

        self.to_registered_mut(&item, trash_path, trash_name);

        //remove original
        if std::fs::remove_dir_all(&item.file_path).is_err() {
            return Err(MyError::FileRemoveError {
                msg: format!("Cannot Remove directory: {:?}", item.file_name),
            });
        }

        Ok(())
    }

    fn to_registered_mut(&mut self, item: &ItemInfo, file_path: PathBuf, file_name: String) {
        let mut buf = item.clone();
        buf.file_path = file_path;
        buf.file_name = file_name;
        buf.selected = false;
        self.registered.push(buf);
    }

    pub fn yank_item(&mut self, index: usize, selected: bool) {
        self.registered.clear();
        if selected {
            for item in self.list.iter_mut().filter(|item| item.selected) {
                self.registered.push(item.clone());
            }
        } else {
            let item = self.get_item(index).unwrap().clone();
            self.registered.push(item);
        }
    }

    pub fn put_items(&mut self) -> Result<(), MyError> {
        //make HashSet<String> of file_name
        let mut name_set = HashSet::new();
        for item in self.list.iter() {
            name_set.insert(item.file_name.clone());
        }

        let total_selected = self.registered.len();
        for (i, item) in self.registered.clone().into_iter().enumerate() {
            print!(
                " {}{}{}",
                cursor::Goto(2, 2),
                clear::CurrentLine,
                display_count(i, total_selected)
            );
            match item.file_type {
                FileType::Directory => {
                    self.put_dir(&item, &mut name_set)?;
                }
                FileType::File | FileType::Symlink => {
                    self.put_file(&item, &mut name_set)?;
                }
            }
        }
        Ok(())
    }

    fn put_file(&mut self, item: &ItemInfo, name_set: &mut HashSet<String>) -> Result<(), MyError> {
        if item.file_path.parent() == Some(&self.trash_dir) {
            let mut item = item.clone();
            let rename = item.file_name.chars().skip(11).collect();
            item.file_name = rename;
            let rename = rename_file(&item, name_set);
            if std::fs::copy(&item.file_path, &self.current_dir.join(&rename)).is_err() {
                return Err(MyError::FileCopyError {
                    msg: format!("Cannot copy item: {:?}", &item.file_path),
                });
            }
            name_set.insert(rename);
        } else {
            let rename = rename_file(item, name_set);
            if std::fs::copy(&item.file_path, &self.current_dir.join(&rename)).is_err() {
                return Err(MyError::FileCopyError {
                    msg: format!("Cannot copy item: {:?}", &item.file_path),
                });
            }
            name_set.insert(rename);
        }
        Ok(())
    }

    fn put_dir(&mut self, buf: &ItemInfo, name_set: &mut HashSet<String>) -> Result<(), MyError> {
        let mut base: usize = 0;
        let mut target: PathBuf = PathBuf::new();
        let original_path = &(buf).file_path;

        let len = walkdir::WalkDir::new(&original_path).into_iter().count();
        let unit = len / 5;
        for (i, entry) in walkdir::WalkDir::new(&original_path)
            .into_iter()
            .enumerate()
        {
            if i > unit * 4 {
                print_process("[»»»»-]");
            } else if i > unit * 3 {
                print_process("[»»»--]");
            } else if i > unit * 2 {
                print_process("[»»---]");
            } else if i > unit {
                print_process("[»----]");
            } else if i == 0 {
                print_process(" [»----]");
            }
            let entry = entry?;
            let entry_path = entry.path();
            if i == 0 {
                base = entry_path.iter().count();

                let parent = &original_path.parent().unwrap();
                if parent == &self.trash_dir {
                    let mut buf = buf.clone();
                    let rename = buf.file_name.chars().skip(11).collect();
                    buf.file_name = rename;

                    let rename = rename_dir(&buf, name_set);
                    target = self.current_dir.join(&rename);
                    name_set.insert(rename);
                } else {
                    let rename = rename_dir(buf, name_set);
                    target = self.current_dir.join(&rename);
                    name_set.insert(rename);
                }
                std::fs::create_dir(&target)?;
                continue;
            } else {
                let child: PathBuf = entry_path.iter().skip(base).collect();
                let child = target.join(child);

                if entry.file_type().is_dir() {
                    std::fs::create_dir_all(child)?;
                    continue;
                } else if let Some(parent) = entry_path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir(parent)?;
                    }
                }

                if std::fs::copy(entry_path, &child).is_err() {
                    return Err(MyError::FileCopyError {
                        msg: format!("Cannot copy item: {:?}", entry_path),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn print(&self, index: usize) {
        let item = &self.get_item(index).unwrap();
        let chars: Vec<char> = item.file_name.chars().collect();
        let name = if chars.len() > self.layout.name_max_len {
            let mut result = chars
                .iter()
                .take(self.layout.name_max_len - 2)
                .collect::<String>();
            result.push_str("..");
            result
        } else {
            item.file_name.clone()
        };
        let time = format_time(&item.modified);
        let selected = &item.selected;
        let color = match item.file_type {
            FileType::Directory => &self.colors.0,
            FileType::File => &self.colors.1,
            FileType::Symlink => &self.colors.2,
        };
        match color {
            Colorname::AnsiValue(n) => {
                print_item!(
                    color::Fg(color::AnsiValue(*n)),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::Black => {
                print_item!(color::Fg(color::Black), name, time, selected, self.layout);
            }
            Colorname::Blue => {
                print_item!(color::Fg(color::Blue), name, time, selected, self.layout);
            }
            Colorname::Cyan => {
                print_item!(color::Fg(color::Cyan), name, time, selected, self.layout);
            }
            Colorname::Green => {
                print_item!(color::Fg(color::Green), name, time, selected, self.layout);
            }
            Colorname::LightBlack => {
                print_item!(
                    color::Fg(color::LightBlack),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::LightBlue => {
                print_item!(
                    color::Fg(color::LightBlue),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::LightCyan => {
                print_item!(
                    color::Fg(color::LightCyan),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::LightGreen => {
                print_item!(
                    color::Fg(color::LightGreen),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::LightMagenta => {
                print_item!(
                    color::Fg(color::LightMagenta),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::LightRed => {
                print_item!(
                    color::Fg(color::LightRed),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::LightWhite => {
                print_item!(
                    color::Fg(color::LightWhite),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::LightYellow => {
                print_item!(
                    color::Fg(color::LightYellow),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::Magenta => {
                print_item!(color::Fg(color::Magenta), name, time, selected, self.layout);
            }
            Colorname::Red => {
                print_item!(color::Fg(color::Red), name, time, selected, self.layout);
            }
            Colorname::Rgb(x, y, z) => {
                print_item!(
                    color::Fg(color::Rgb(*x, *y, *z)),
                    name,
                    time,
                    selected,
                    self.layout
                );
            }
            Colorname::White => {
                print_item!(color::Fg(color::White), name, time, selected, self.layout);
            }
            Colorname::Yellow => {
                print_item!(color::Fg(color::Yellow), name, time, selected, self.layout);
            }
        }
    }

    pub fn list_up(&self, skip_number: u16) {
        let row = self.layout.terminal_row;

        //if list exceeds max-row
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
                break;
            } else {
                self.print(i);
                row_count += 1;
            }
        }
    }

    pub fn update_list(&mut self) -> Result<(), MyError> {
        self.list = push_items(&self.current_dir, &self.sort_by, self.show_hidden)?;
        Ok(())
    }

    pub fn reset_selection(&mut self) {
        for mut item in self.list.iter_mut() {
            item.selected = false;
        }
    }

    pub fn select_from_top(&mut self, start_pos: usize) {
        for (i, item) in self.list.iter_mut().enumerate() {
            if i <= start_pos {
                item.selected = true;
            } else {
                item.selected = false;
            }
        }
    }

    pub fn select_to_bottom(&mut self, start_pos: usize) {
        for (i, item) in self.list.iter_mut().enumerate() {
            if i < start_pos {
                item.selected = false;
            } else {
                item.selected = true;
            }
        }
    }

    pub fn move_cursor(&self, nums: &Num, y: u16) {
        print!("{}", cursor::Goto(1, self.layout.terminal_row));
        print!("{}", clear::CurrentLine);

        let item = self.get_item(nums.index);
        if let Ok(item) = item {
            match &item.file_ext {
                Some(ext) => {
                    print!(
                        "[{}/{}] {} {}",
                        nums.index + 1,
                        self.list.len(),
                        ext.clone().into_string().unwrap_or_default(),
                        to_proper_size(item.file_size)
                    );
                }
                None => {
                    print!(
                        "[{}/{}] {}",
                        nums.index + 1,
                        self.list.len(),
                        to_proper_size(item.file_size)
                    );
                }
            }
            if self.rust_log == Some("debug".to_string()) {
                print!(
                    " index:{} skip:{} max:{} column:{}",
                    nums.index, nums.skip, self.layout.name_max_len, self.layout.terminal_column
                );
            }
        }
        print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
    }

    pub fn write_session(&self, session_path: PathBuf) -> Result<(), MyError> {
        let session = Session {
            sort_by: self.sort_by.clone(),
            show_hidden: self.show_hidden,
        };
        let serialized = toml::to_string(&session)?;
        fs::write(&session_path, serialized)?;
        Ok(())
    }
}

fn make_item(entry: fs::DirEntry) -> ItemInfo {
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path);

    let name = entry
        .file_name()
        .into_string()
        .unwrap_or_else(|_| "Invalid unicode name".to_string());

    let ext = path.extension().map(|s| s.to_os_string());

    match metadata {
        Ok(metadata) => {
            let time = {
                let sometime = metadata.modified().unwrap();
                let chrono_time: DateTime<Local> = DateTime::from(sometime);
                Some(chrono_time.to_rfc3339_opts(SecondsFormat::Secs, false))
            };

            let filetype = {
                let file_type = metadata.file_type();
                if file_type.is_dir() {
                    FileType::Directory
                } else if file_type.is_file() {
                    FileType::File
                } else if file_type.is_symlink() {
                    FileType::Symlink
                } else {
                    FileType::File
                }
            };

            let sym_dir_path = {
                if filetype == FileType::Symlink {
                    if let Ok(sym_meta) = fs::metadata(&path) {
                        if sym_meta.is_dir() {
                            fs::canonicalize(path.clone()).ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let size = metadata.len();
            ItemInfo {
                file_type: filetype,
                file_name: name,
                file_path: path,
                symlink_dir_path: sym_dir_path,
                file_size: size,
                file_ext: ext,
                modified: time,
                selected: false,
            }
        }
        Err(_) => ItemInfo {
            file_type: FileType::File,
            file_name: name,
            file_path: path,
            symlink_dir_path: None,
            file_size: 0,
            file_ext: ext,
            modified: None,
            selected: false,
        },
    }
}

fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| !s.starts_with('.'))
        .unwrap_or(false)
}

pub fn push_items(p: &Path, key: &SortKey, show_hidden: bool) -> Result<Vec<ItemInfo>, MyError> {
    let mut result = Vec::new();
    let mut dir_v = Vec::new();
    let mut file_v = Vec::new();

    for entry in fs::read_dir(p)? {
        let e = entry?;
        if !show_hidden && !is_not_hidden(&e) {
            continue;
        }
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

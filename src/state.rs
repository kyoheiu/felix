use super::config::*;
use super::errors::FxError;
use super::functions::*;
use super::layout::*;
use super::nums::*;
use super::op::*;
use super::session::*;
use super::term::*;

use chrono::prelude::*;
use crossterm::event;
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::Stylize;
use log::{error, info};
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::UNIX_EPOCH;
use syntect::highlighting::{Theme, ThemeSet};

pub const BEGINNING_ROW: u16 = 3;
pub const FX_CONFIG_DIR: &str = "felix";
pub const TRASH: &str = "trash";
pub const EMPTY_WARNING: &str = "Are you sure to empty the trash directory? (if yes: y)";

#[derive(Debug)]
pub struct State {
    pub list: Vec<ItemInfo>,
    pub current_dir: PathBuf,
    pub trash_dir: PathBuf,
    pub default: String,
    pub commands: Option<HashMap<String, String>>,
    pub registered: Vec<ItemInfo>,
    pub operations: Operation,
    pub c_memo: Vec<StateMemo>,
    pub p_memo: Vec<StateMemo>,
    pub keyword: Option<String>,
    pub layout: Layout,
    pub rust_log: Option<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct ItemInfo {
    pub file_type: FileType,
    pub file_name: String,
    pub file_path: std::path::PathBuf,
    pub symlink_dir_path: Option<PathBuf>,
    pub file_size: u64,
    pub file_ext: Option<String>,
    pub modified: Option<String>,
    pub is_hidden: bool,
    pub selected: bool,
    pub matches: bool,
    pub preview_type: Option<PreviewType>,
    pub preview_scroll: usize,
    pub content: Option<String>,
    pub permissions: Option<u32>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    Directory,
    File,
    Symlink,
}

impl State {
    /// Initialize the state of the app.
    pub fn new(p: &std::path::Path) -> Result<Self, FxError> {
        let config = match read_config(p) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Cannot read the config file properly.\nError: {}\nDo you want to use the default config? [press Enter to continue]", e);
                enter_raw_mode();
                loop {
                    match event::read()? {
                        event::Event::Key(KeyEvent { code, .. }) => match code {
                            KeyCode::Enter => break,
                            _ => {
                                leave_raw_mode();
                                return Err(FxError::Yaml("Exit the app.".to_owned()));
                            }
                        },
                        _ => {
                            continue;
                        }
                    }
                }
                leave_raw_mode();
                Config::default()
            }
        };
        let session = read_session()?;
        let (original_column, original_row) =
            crossterm::terminal::size().unwrap_or_else(|_| panic!("Cannot detect terminal size."));

        // Return error if terminal size may cause panic
        if original_column < 4 {
            error!("Too small terminal size (less than 4 columns).");
            return Err(FxError::TooSmallWindowSize);
        };
        if original_row < 4 {
            error!("Too small terminal size. (less than 4 rows)");
            return Err(FxError::TooSmallWindowSize);
        };

        let (time_start, name_max) = make_layout(original_column);

        let ts = set_theme(&config);
        let split = session.split.unwrap_or(Split::Vertical);

        let has_chafa = check_chafa();
        let is_kitty = check_kitty_support();

        Ok(State {
            list: Vec::new(),
            registered: Vec::new(),
            operations: Operation {
                pos: 0,
                op_list: Vec::new(),
            },
            current_dir: PathBuf::new(),
            trash_dir: PathBuf::new(),
            default: config
                .default
                .unwrap_or_else(|| env::var("EDITOR").unwrap_or_default()),
            commands: to_extension_map(&config.exec),
            layout: Layout {
                nums: Num::new(),
                y: BEGINNING_ROW,
                terminal_row: original_row,
                terminal_column: original_column,
                name_max_len: name_max,
                time_start_pos: time_start,
                colors: ConfigColor {
                    dir_fg: config.color.dir_fg,
                    file_fg: config.color.file_fg,
                    symlink_fg: config.color.symlink_fg,
                },
                sort_by: session.sort_by,
                show_hidden: session.show_hidden,
                preview: session.preview.unwrap_or(false),
                split,
                preview_start: match split {
                    Split::Vertical => (0, 0),
                    Split::Horizontal => (0, 0),
                },
                preview_space: match split {
                    Split::Vertical => (0, 0),
                    Split::Horizontal => (0, 0),
                },
                syntax_highlight: config.syntax_highlight.unwrap_or(false),
                syntax_set: syntect::parsing::SyntaxSet::load_defaults_newlines(),
                theme: ts,
                has_chafa,
                is_kitty,
            },
            c_memo: Vec::new(),
            p_memo: Vec::new(),
            keyword: None,
            rust_log: std::env::var("RUST_LOG").ok(),
        })
    }

    /// Select an item that the cursor points to.
    pub fn get_item(&self) -> Result<&ItemInfo, FxError> {
        self.list
            .get(self.layout.nums.index)
            .ok_or(FxError::GetItem)
    }

    /// Select an item that the cursor points to, as mut.
    pub fn get_item_mut(&mut self) -> Result<&mut ItemInfo, FxError> {
        self.list
            .get_mut(self.layout.nums.index)
            .ok_or(FxError::GetItem)
    }

    /// Open the selected file according to the config.
    pub fn open_file(&self, item: &ItemInfo) -> Result<ExitStatus, FxError> {
        let path = &item.file_path;
        let map = &self.commands;
        let extension = item.file_ext.as_ref();

        let mut default = Command::new(&self.default);

        info!("OPEN: {:?}", path);

        match map {
            None => default.arg(path).status().or(Err(FxError::OpenItem)),
            Some(map) => match extension {
                None => default.arg(path).status().or(Err(FxError::OpenItem)),
                Some(extension) => match map.get(extension) {
                    Some(command) => {
                        let mut ex = Command::new(command);
                        ex.arg(path).status().or(Err(FxError::OpenItem))
                    }
                    None => default.arg(path).status().or(Err(FxError::OpenItem)),
                },
            },
        }
    }

    /// Open the selected file in a new window, according to the config.
    pub fn open_file_in_new_window(&self) -> Result<Child, FxError> {
        let item = self.get_item()?;
        let path = &item.file_path;
        let map = &self.commands;
        let extension = &item.file_ext;

        info!("OPEN(new window): {:?}", path);

        match map {
            None => Err(FxError::OpenNewWindow("No exec configuration".to_owned())),
            Some(map) => match extension {
                Some(extension) => match map.get(extension) {
                    Some(command) => {
                        let mut ex = Command::new(command);
                        ex.arg(path)
                            .stdout(Stdio::null())
                            .stdin(Stdio::null())
                            .spawn()
                            .or(Err(FxError::OpenItem))
                    }
                    None => Err(FxError::OpenNewWindow(
                        "Cannot open this type of item in new window".to_owned(),
                    )),
                },

                None => Err(FxError::OpenNewWindow(
                    "Cannot open this type of item in new window".to_owned(),
                )),
            },
        }
    }

    /// Move items from the current directory to trash directory.
    /// This does not actually delete items.
    /// If you'd like to delete, use `:empty` after this, or just `:rm`.  
    pub fn remove_and_yank(&mut self, targets: &[ItemInfo], new_op: bool) -> Result<(), FxError> {
        self.registered.clear();
        let total_selected = targets.len();
        let mut trash_vec = Vec::new();
        for (i, item) in targets.iter().enumerate() {
            let item = item.clone();

            print!(" ");
            to_info_bar();
            clear_current_line();
            print!("{}", display_count(i, total_selected));

            match item.file_type {
                FileType::Directory => match self.remove_and_yank_dir(item.clone(), new_op) {
                    Err(e) => {
                        return Err(e);
                    }
                    Ok(path) => trash_vec.push(path),
                },
                FileType::File | FileType::Symlink => {
                    match self.remove_and_yank_file(item.clone(), new_op) {
                        Err(e) => {
                            return Err(e);
                        }
                        Ok(path) => trash_vec.push(path),
                    }
                }
            }
        }
        if new_op {
            self.operations.branch();
            //push deleted item information to operations
            self.operations.push(OpKind::Delete(DeletedFiles {
                trash: trash_vec,
                original: targets.to_vec(),
                dir: self.current_dir.clone(),
            }));
        }

        Ok(())
    }

    /// Move single file to trash directory.
    fn remove_and_yank_file(&mut self, item: ItemInfo, new_op: bool) -> Result<PathBuf, FxError> {
        //prepare from and to for copy
        let from = &item.file_path;
        let mut to = PathBuf::new();

        if item.file_type == FileType::Symlink && !from.exists() {
            match Command::new("rm").arg(from).status() {
                Ok(_) => Ok(PathBuf::new()),
                Err(_) => Err(FxError::RemoveItem(from.to_owned())),
            }
        } else {
            let name = &item.file_name;
            let mut rename = Local::now().timestamp().to_string();
            rename.push('_');
            rename.push_str(name);

            if new_op {
                to = self.trash_dir.join(&rename);

                //copy
                if std::fs::copy(from, &to).is_err() {
                    return Err(FxError::PutItem(from.to_owned()));
                }

                self.push_to_registered(&item, to.clone(), rename);
            }

            //remove original
            if std::fs::remove_file(from).is_err() {
                return Err(FxError::RemoveItem(from.to_owned()));
            }

            Ok(to)
        }
    }

    /// Move single directory recursively to trash directory.
    fn remove_and_yank_dir(&mut self, item: ItemInfo, new_op: bool) -> Result<PathBuf, FxError> {
        let mut trash_name = String::new();
        let mut base: usize = 0;
        let mut trash_path: std::path::PathBuf = PathBuf::new();
        let mut target: PathBuf;

        if new_op {
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
                    if file_name.is_none() {
                        return Err(FxError::Encode);
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
                        return Err(FxError::PutItem(entry_path.to_owned()));
                    }
                }
            }

            self.push_to_registered(&item, trash_path.clone(), trash_name);
        }

        //remove original
        if std::fs::remove_dir_all(&item.file_path).is_err() {
            return Err(FxError::RemoveItem(item.file_path));
        }

        Ok(trash_path)
    }

    /// Register removed items to the registry.
    fn push_to_registered(&mut self, item: &ItemInfo, file_path: PathBuf, file_name: String) {
        let mut buf = item.clone();
        buf.file_path = file_path;
        buf.file_name = file_name;
        buf.selected = false;
        self.registered.push(buf);
    }

    /// Register selected items to the registry.
    pub fn yank_item(&mut self, selected: bool) {
        self.registered.clear();
        if selected {
            for item in self.list.iter_mut().filter(|item| item.selected) {
                self.registered.push(item.clone());
            }
        } else {
            let item = self.get_item().unwrap().clone();
            self.registered.push(item);
        }
    }

    /// Put items in registry to the current directory or target directory.
    /// Only Redo command uses target directory.
    pub fn put_items(
        &mut self,
        targets: &[ItemInfo],
        target_dir: Option<PathBuf>,
    ) -> Result<(), FxError> {
        //make HashSet<String> of file_name
        let mut name_set = HashSet::new();
        match &target_dir {
            None => {
                for item in self.list.iter() {
                    name_set.insert(item.file_name.clone());
                }
            }
            Some(path) => {
                for entry in std::fs::read_dir(path)? {
                    let entry = entry?;
                    name_set.insert(
                        entry
                            .file_name()
                            .into_string()
                            .unwrap_or_else(|_| "".to_string()),
                    );
                }
            }
        }

        //prepare for operations.push
        let mut put_v = Vec::new();

        let total_selected = targets.len();
        for (i, item) in targets.iter().enumerate() {
            print!(" ");
            to_info_bar();
            clear_current_line();
            print!("{}", display_count(i, total_selected));

            match item.file_type {
                FileType::Directory => {
                    if let Ok(p) = self.put_dir(item, &target_dir, &mut name_set) {
                        put_v.push(p);
                    }
                }
                FileType::File | FileType::Symlink => {
                    if let Ok(q) = self.put_file(item, &target_dir, &mut name_set) {
                        put_v.push(q);
                    }
                }
            }
        }
        if target_dir.is_none() {
            self.operations.branch();
            //push put item information to operations
            self.operations.push(OpKind::Put(PutFiles {
                original: targets.to_owned(),
                put: put_v,
                dir: self.current_dir.clone(),
            }));
        }

        Ok(())
    }

    /// Put single item to current or target directory.
    fn put_file(
        &mut self,
        item: &ItemInfo,
        target_dir: &Option<PathBuf>,
        name_set: &mut HashSet<String>,
    ) -> Result<PathBuf, FxError> {
        match target_dir {
            None => {
                if item.file_path.parent() == Some(&self.trash_dir) {
                    let rename: String = item.file_name.chars().skip(11).collect();
                    let rename = rename_file(&rename, name_set);
                    let to = &self.current_dir.join(&rename);
                    if std::fs::copy(&item.file_path, to).is_err() {
                        return Err(FxError::PutItem(item.file_path.clone()));
                    }
                    name_set.insert(rename);
                    Ok(to.to_path_buf())
                } else {
                    let rename = rename_file(&item.file_name, name_set);
                    let to = &self.current_dir.join(&rename);
                    if std::fs::copy(&item.file_path, to).is_err() {
                        return Err(FxError::PutItem(item.file_path.clone()));
                    }
                    name_set.insert(rename);
                    Ok(to.to_path_buf())
                }
            }
            Some(path) => {
                if item.file_path.parent() == Some(&self.trash_dir) {
                    let rename: String = item.file_name.chars().skip(11).collect();
                    let rename = rename_file(&rename, name_set);
                    let to = path.join(&rename);
                    if std::fs::copy(&item.file_path, to.clone()).is_err() {
                        return Err(FxError::PutItem(item.file_path.clone()));
                    }
                    name_set.insert(rename);
                    Ok(to)
                } else {
                    let rename = rename_file(&item.file_name, name_set);
                    let to = &path.join(&rename);
                    if std::fs::copy(&item.file_path, to).is_err() {
                        return Err(FxError::PutItem(item.file_path.clone()));
                    }
                    name_set.insert(rename);
                    Ok(to.to_path_buf())
                }
            }
        }
    }

    /// Put single directory recursively to current or target directory.
    fn put_dir(
        &mut self,
        buf: &ItemInfo,
        target_dir: &Option<PathBuf>,
        name_set: &mut HashSet<String>,
    ) -> Result<PathBuf, FxError> {
        let mut base: usize = 0;
        let mut target: PathBuf = PathBuf::new();
        let original_path = &buf.file_path;

        let len = walkdir::WalkDir::new(original_path).into_iter().count();
        let unit = len / 5;
        for (i, entry) in walkdir::WalkDir::new(original_path).into_iter().enumerate() {
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

                let parent = &original_path
                    .parent()
                    .ok_or_else(|| FxError::Io("Cannot read parent dir.".to_string()))?;
                if parent == &self.trash_dir {
                    let rename: String = buf.file_name.chars().skip(11).collect();
                    target = match &target_dir {
                        None => self.current_dir.join(&rename),
                        Some(path) => path.join(&rename),
                    };
                    let rename = rename_dir(&rename, name_set);
                    name_set.insert(rename);
                } else {
                    let rename = rename_dir(&buf.file_name, name_set);
                    target = match &target_dir {
                        None => self.current_dir.join(&rename),
                        Some(path) => path.join(&rename),
                    };
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
                    return Err(FxError::PutItem(entry_path.to_owned()));
                }
            }
        }
        Ok(target)
    }

    /// Undo operations (put/delete/rename).
    pub fn undo(&mut self, op: &OpKind) -> Result<(), FxError> {
        match op {
            OpKind::Rename(op) => {
                std::fs::rename(&op.new_name, &op.original_name)?;
                self.operations.pos += 1;
                self.update_list()?;
                self.clear_and_show_headline();
                self.list_up();
                print_info("UNDONE: RENAME", BEGINNING_ROW);
            }
            OpKind::Put(op) => {
                for x in &op.put {
                    if x.is_dir() {
                        std::fs::remove_dir_all(x)?;
                    } else {
                        std::fs::remove_file(x)?;
                    }
                }
                self.operations.pos += 1;
                self.update_list()?;
                self.clear_and_show_headline();
                self.list_up();
                print_info("UNDONE: PUT", BEGINNING_ROW);
            }
            OpKind::Delete(op) => {
                let targets = trash_to_info(&self.trash_dir, &op.trash)?;
                self.put_items(&targets, Some(op.dir.clone()))?;
                self.operations.pos += 1;
                self.update_list()?;
                self.clear_and_show_headline();
                self.list_up();
                print_info("UNDONE: DELETE", BEGINNING_ROW);
            }
        }
        relog(op, true);
        Ok(())
    }

    /// Redo operations (put/delete/rename)
    pub fn redo(&mut self, op: &OpKind) -> Result<(), FxError> {
        match op {
            OpKind::Rename(op) => {
                std::fs::rename(&op.original_name, &op.new_name)?;
                self.operations.pos -= 1;
                self.update_list()?;
                self.clear_and_show_headline();
                self.list_up();
                print_info("REDONE: RENAME", BEGINNING_ROW);
            }
            OpKind::Put(op) => {
                self.put_items(&op.original, Some(op.dir.clone()))?;
                self.operations.pos -= 1;
                self.update_list()?;
                self.clear_and_show_headline();
                self.list_up();
                print_info("REDONE: PUT", BEGINNING_ROW);
            }
            OpKind::Delete(op) => {
                self.remove_and_yank(&op.original, false)?;
                self.operations.pos -= 1;
                self.update_list()?;
                self.clear_and_show_headline();
                self.list_up();
                print_info("REDONE DELETE", BEGINNING_ROW);
            }
        }
        relog(op, false);
        Ok(())
    }

    /// Redraw the contents.
    pub fn redraw(&mut self, y: u16) {
        self.clear_and_show_headline();
        self.list_up();
        self.move_cursor(y);
    }

    /// Reload the item list and redraw it.
    pub fn reload(&mut self, y: u16) -> Result<(), FxError> {
        self.update_list()?;
        self.clear_and_show_headline();
        self.list_up();
        self.move_cursor(y);
        Ok(())
    }

    /// Reload the app layout when terminal size changes.
    pub fn refresh(&mut self, column: u16, row: u16, mut cursor_pos: u16) {
        let (time_start, name_max) = make_layout(column);

        let (original_column, original_row) =
            crossterm::terminal::size().unwrap_or_else(|_| panic!("Cannot detect terminal size."));

        self.layout.terminal_row = row;
        self.layout.terminal_column = column;
        self.layout.preview_start = match self.layout.split {
            Split::Vertical => (column + 2, BEGINNING_ROW),
            Split::Horizontal => (1, row + 2),
        };
        self.layout.preview_space = match self.layout.preview {
            true => match self.layout.split {
                Split::Vertical => (original_column - column - 1, row - BEGINNING_ROW),
                Split::Horizontal => (column, original_row - row - 1),
            },
            false => (0, 0),
        };
        self.layout.name_max_len = name_max;
        self.layout.time_start_pos = time_start;

        if cursor_pos > row - 1 {
            self.layout.nums.index -= (cursor_pos - row + 1) as usize;
            cursor_pos = row - 1;
        }

        self.redraw(cursor_pos);
    }

    /// Clear all and show the current directory information.
    pub fn clear_and_show_headline(&mut self) {
        clear_all();
        move_to(1, 1);

        //Show current directory path.
        //crossterm's Stylize cannot be applied to PathBuf,
        //current directory does not have any text attribute for now.
        set_color(&TermColor::ForeGround(&Colorname::Cyan));
        print!(" {}", self.current_dir.display(),);
        reset_color();

        //If .git directory exists, get the branch information and print it.
        let git = self.current_dir.join(".git");
        if git.exists() {
            let head = git.join("HEAD");
            if let Ok(head) = std::fs::read(head) {
                let branch: Vec<u8> = head.into_iter().skip(16).collect();
                if let Ok(branch) = std::str::from_utf8(&branch) {
                    print!(" on ",);
                    set_color(&TermColor::ForeGround(&Colorname::LightMagenta));
                    print!("{}", branch.trim().bold());
                    reset_color();
                }
            }
        }
    }

    /// Print an item in the directory.
    fn print_item(&self, item: &ItemInfo) {
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
        let color = match item.file_type {
            FileType::Directory => &self.layout.colors.dir_fg,
            FileType::File => &self.layout.colors.file_fg,
            FileType::Symlink => &self.layout.colors.symlink_fg,
        };
        if self.layout.terminal_column < PROPER_WIDTH {
            if item.selected {
                set_color(&TermColor::ForeGround(color));
                print!("{}", name.negative(),);
                reset_color();
            } else if item.matches {
                set_color(&TermColor::ForeGround(color));
                print!("{}", name.bold(),);
                reset_color();
            } else {
                set_color(&TermColor::ForeGround(color));
                print!("{}", name);
                reset_color();
            }
            if self.layout.terminal_column > self.layout.time_start_pos + TIME_WIDTH {
                clear_until_newline();
            }
        } else if item.selected {
            set_color(&TermColor::ForeGround(color));
            print!("{}", name.negative(),);
            move_left(1000);
            move_right(self.layout.time_start_pos - 1);
            print!(" {}", time.negative());
            reset_color();
        } else if item.matches {
            set_color(&TermColor::ForeGround(color));
            print!("{}", name.bold(),);
            move_left(1000);
            move_right(self.layout.time_start_pos - 1);
            set_color(&TermColor::ForeGround(color));
            print!(" {}", time);
            reset_color();
        } else {
            set_color(&TermColor::ForeGround(color));
            print!("{}", name);
            move_left(1000);
            move_right(self.layout.time_start_pos - 1);
            print!(" {}", time);
            reset_color();
        }
    }

    /// Print items in the directory.
    pub fn list_up(&self) {
        let visible = &self.list[..];

        visible.iter().enumerate().for_each(|(index, item)| {
            if index >= self.layout.nums.skip.into()
                && index < (self.layout.terminal_row + self.layout.nums.skip - BEGINNING_ROW).into()
            {
                move_to(3, (index as u16 + BEGINNING_ROW) - self.layout.nums.skip);
                self.print_item(item);
            }
        });
    }

    /// Update state's list of items.
    pub fn update_list(&mut self) -> Result<(), FxError> {
        let mut result = Vec::new();
        let mut dir_v = Vec::new();
        let mut file_v = Vec::new();

        for entry in fs::read_dir(&self.current_dir)? {
            let e = entry?;
            let entry = read_item(e);
            match entry.file_type {
                FileType::Directory => dir_v.push(entry),
                FileType::File | FileType::Symlink => file_v.push(entry),
            }
        }

        match self.layout.sort_by {
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

        if !self.layout.show_hidden {
            result.retain(|x| !x.is_hidden);
        }

        self.list = result;
        Ok(())
    }

    /// Reset all item's selected state and exit the select mode.
    pub fn reset_selection(&mut self) {
        for mut item in self.list.iter_mut() {
            item.selected = false;
        }
    }

    pub fn highlight_matches(&mut self, keyword: &str) {
        for item in self.list.iter_mut() {
            if item.file_name.contains(keyword) {
                item.matches = true;
            } else {
                item.matches = false;
            }
        }
    }

    /// Select items from the top to current position.
    pub fn select_from_top(&mut self, start_pos: usize) {
        for (i, item) in self.list.iter_mut().enumerate() {
            if i <= start_pos {
                item.selected = true;
            } else {
                item.selected = false;
            }
        }
    }

    /// Select items from the current position to bottom.
    pub fn select_to_bottom(&mut self, start_pos: usize) {
        for (i, item) in self.list.iter_mut().enumerate() {
            if i < start_pos {
                item.selected = false;
            } else {
                item.selected = true;
            }
        }
    }

    pub fn chdir(&mut self, p: &std::path::Path, mv: Move) -> Result<(), FxError> {
        std::env::set_current_dir(p)?;
        match mv {
            Move::Up => {
                // Push current state to c_memo
                let cursor_memo = StateMemo {
                    path: self.current_dir.clone(),
                    num: self.layout.nums,
                    cursor_pos: self.layout.y,
                };
                self.c_memo.push(cursor_memo);

                //Pop p_memo if exists; identify the dir from which we come
                match self.p_memo.pop() {
                    Some(memo) => {
                        self.current_dir = memo.path;
                        self.keyword = None;
                        self.layout.nums.index = memo.num.index;
                        self.layout.nums.skip = memo.num.skip;
                        self.reload(memo.cursor_pos)?;
                    }
                    None => {
                        let pre = self.current_dir.clone();
                        self.current_dir = p.to_owned();
                        self.update_list()?;
                        match pre.file_name() {
                            Some(name) => {
                                let new_pos = self
                                    .list
                                    .iter()
                                    .position(|x| {
                                        let file_name = x.file_name.as_ref() as &OsStr;
                                        file_name == name
                                    })
                                    .unwrap_or(0);
                                self.keyword = None;
                                if new_pos < 3 {
                                    self.layout.nums.skip = 0;
                                    self.layout.nums.index = new_pos;
                                    self.redraw((new_pos as u16) + BEGINNING_ROW);
                                } else {
                                    self.layout.nums.skip = (new_pos - 3) as u16;
                                    self.layout.nums.index = new_pos;
                                    self.redraw(BEGINNING_ROW + 3);
                                }
                            }
                            None => {
                                self.current_dir = p.to_owned();
                                self.keyword = None;
                                self.layout.nums.reset();
                                self.redraw(BEGINNING_ROW);
                            }
                        }
                    }
                }
            }
            Move::Down => {
                // Push current state to p_memo
                let cursor_memo = StateMemo {
                    path: self.current_dir.clone(),
                    num: self.layout.nums,
                    cursor_pos: self.layout.y,
                };
                self.p_memo.push(cursor_memo);
                self.current_dir = p.to_owned();
                self.keyword = None;

                // Pop c_memo
                match self.c_memo.pop() {
                    Some(memo) => {
                        if p == memo.path {
                            self.layout.nums.index = memo.num.index;
                            self.layout.nums.skip = memo.num.skip;
                            self.reload(memo.cursor_pos)?;
                        } else {
                            self.layout.nums.reset();
                            self.reload(BEGINNING_ROW)?;
                        }
                    }
                    None => {
                        self.layout.nums.reset();
                        self.reload(BEGINNING_ROW)?;
                    }
                }
            }
            Move::Jump => {
                self.current_dir = p.to_owned();
                self.p_memo = Vec::new();
                self.c_memo = Vec::new();
                self.keyword = None;
                self.layout.nums.reset();
                self.reload(BEGINNING_ROW)?;
            }
        }
        Ok(())
    }

    /// Change the cursor position, and print item information at the bottom.
    /// If preview is enabled, print text preview, contents of the directory or image preview on the right half of the terminal
    /// (To preview image, you must install chafa. See help).
    pub fn move_cursor(&mut self, y: u16) {
        // If preview is enabled, set the preview type, read the content (if text type) and reset the scroll.
        if self.layout.preview {
            if let Ok(item) = self.get_item_mut() {
                if item.preview_type.is_none() {
                    set_preview_type(item);
                }
                item.preview_scroll = 0;
            }
        }

        if let Ok(item) = self.get_item() {
            delete_cursor();

            //Print item information at the bottom
            self.print_footer(item);

            //Print preview if preview is on
            if self.layout.preview {
                self.layout.print_preview(item, y);
            }
        }
        move_to(1, y);
        print_pointer();
        move_left(1);

        //Store cursor position when cursor moves
        self.layout.y = y;
    }

    pub fn to_status_bar(&self) {
        move_to(1, self.layout.terminal_row);
    }

    pub fn clear_status_line(&self) {
        self.to_status_bar();
        clear_current_line();
        reset_color();
        print!(
            "{}",
            " ".repeat(self.layout.terminal_column as usize).negative(),
        );
        move_to(1, self.layout.terminal_row);
    }

    /// Print item information at the bottom of the terminal.
    fn print_footer(&self, item: &ItemInfo) {
        self.clear_status_line();

        if let Some(keyword) = &self.keyword {
            let count = self
                .list
                .iter()
                .filter(|x| x.file_name.contains(keyword))
                .count();
            let count = if count <= 1 {
                format!("{} match", count)
            } else {
                format!("{} matches", count)
            };
            print!(
                "{}",
                " ".repeat(self.layout.terminal_column as usize).negative(),
            );
            move_to(1, self.layout.terminal_row);
            print!(
                "{}{}{}{}",
                " /".negative(),
                keyword.clone().negative(),
                " - ".negative(),
                count.negative()
            );
            return;
        }

        let footer = self.make_footer(item);
        print!("{}", footer.negative());
    }

    fn make_footer(&self, item: &ItemInfo) -> String {
        match &item.file_ext {
            Some(ext) => {
                let mut footer = match item.permissions {
                    Some(permissions) => {
                        format!(
                            " {}/{} {} {} {}",
                            self.layout.nums.index + 1,
                            self.list.len(),
                            ext.clone(),
                            to_proper_size(item.file_size),
                            convert_to_permissions(permissions)
                        )
                    }
                    None => format!(
                        " {}/{} {} {}",
                        self.layout.nums.index + 1,
                        self.list.len(),
                        ext.clone(),
                        to_proper_size(item.file_size),
                    ),
                };
                if self.rust_log.is_some() {
                    let _ = write!(
                        footer,
                        " i:{} s:{} c:{} r:{}",
                        self.layout.nums.index,
                        self.layout.nums.skip,
                        self.layout.terminal_column,
                        self.layout.terminal_row
                    );
                }
                footer
                    .chars()
                    .take(self.layout.terminal_column.into())
                    .collect()
            }
            None => {
                let mut footer = match item.permissions {
                    Some(permissions) => {
                        format!(
                            " {}/{} {} {}",
                            self.layout.nums.index + 1,
                            self.list.len(),
                            to_proper_size(item.file_size),
                            convert_to_permissions(permissions)
                        )
                    }
                    None => format!(
                        " {}/{} {}",
                        self.layout.nums.index + 1,
                        self.list.len(),
                        to_proper_size(item.file_size),
                    ),
                };
                if self.rust_log.is_some() {
                    let _ = write!(
                        footer,
                        " i:{} s:{} c:{} r:{}",
                        self.layout.nums.index,
                        self.layout.nums.skip,
                        self.layout.terminal_column,
                        self.layout.terminal_row
                    );
                }
                footer
                    .chars()
                    .take(self.layout.terminal_column.into())
                    .collect()
            }
        }
    }

    pub fn scroll_down_preview(&mut self, y: u16) {
        if let Ok(item) = self.get_item_mut() {
            item.preview_scroll += 1;
            self.scroll_preview(y)
        }
    }

    pub fn scroll_up_preview(&mut self, y: u16) {
        if let Ok(item) = self.get_item_mut() {
            if item.preview_scroll != 0 {
                item.preview_scroll -= 1;
                self.scroll_preview(y)
            }
        }
    }

    fn scroll_preview(&self, y: u16) {
        if let Ok(item) = self.get_item() {
            self.layout.print_preview(item, y);
            move_to(1, y);
            print_pointer();
            move_left(1);
        }
    }

    /// Store the sort key and whether to show hidden items to session file.
    pub fn write_session(&self, session_path: PathBuf) -> Result<(), FxError> {
        let session = Session {
            sort_by: self.layout.sort_by.clone(),
            show_hidden: self.layout.show_hidden,
            preview: Some(self.layout.preview),
            split: Some(self.layout.split),
        };
        let serialized = serde_yaml::to_string(&session)?;
        fs::write(&session_path, serialized)?;
        Ok(())
    }
}

/// Read item information from `std::fs::DirEntry`.
fn read_item(entry: fs::DirEntry) -> ItemInfo {
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path);

    let name = entry
        .file_name()
        .into_string()
        .unwrap_or_else(|_| "Invalid unicode name".to_string());

    let hidden = matches!(name.chars().next(), Some('.'));

    let ext = path.extension().map(|s| {
        s.to_os_string()
            .into_string()
            .unwrap_or_default()
            .to_ascii_lowercase()
    });

    match metadata {
        Ok(metadata) => {
            let time = {
                let sometime = metadata.modified().unwrap_or(UNIX_EPOCH);
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

            #[cfg(target_family = "unix")]
            let permissions = Some(metadata.permissions().mode());
            #[cfg(not(target_family = "unix"))]
            let permissions = None;

            let size = metadata.len();
            ItemInfo {
                file_type: filetype,
                file_name: name,
                file_path: path,
                symlink_dir_path: sym_dir_path,
                file_size: size,
                file_ext: match filetype {
                    FileType::Directory => None,
                    _ => ext,
                },
                modified: time,
                selected: false,
                matches: false,
                is_hidden: hidden,
                preview_type: None,
                preview_scroll: 0,
                content: None,
                permissions,
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
            matches: false,
            is_hidden: false,
            preview_type: None,
            preview_scroll: 0,
            content: None,
            permissions: None,
        },
    }
}

/// Generate item information from trash directory, in order to use when redoing.
pub fn trash_to_info(trash_dir: &PathBuf, vec: &[PathBuf]) -> Result<Vec<ItemInfo>, FxError> {
    let total = vec.len();
    let mut count = 0;
    let mut result = Vec::new();
    for entry in fs::read_dir(trash_dir)? {
        let entry = entry?;
        if vec.contains(&entry.path()) {
            result.push(read_item(entry));
            count += 1;
            if count == total {
                break;
            }
        }
    }
    Ok(result)
}

fn check_chafa() -> bool {
    std::process::Command::new("chafa")
        .arg("--help")
        .output()
        .is_ok()
}

// Check if the terminal is Kitty or not
fn check_kitty_support() -> bool {
    if let Ok(term) = std::env::var("TERM") {
        term.contains("kitty")
    } else {
        false
    }
}

fn set_preview_content_type(item: &mut ItemInfo) {
    if item.file_size > MAX_SIZE_TO_PREVIEW {
        item.preview_type = Some(PreviewType::TooBigSize);
    } else if is_supported_ext(item) {
        item.preview_type = Some(PreviewType::Image);
    } else if let Ok(content) = &std::fs::read(&item.file_path) {
        if content_inspector::inspect(content).is_text() {
            if let Ok(content) = String::from_utf8(content.to_vec()) {
                let content = content.replace('\t', "    ");
                item.content = Some(content);
            }
            item.preview_type = Some(PreviewType::Text);
        } else {
            item.preview_type = Some(PreviewType::Binary);
        }
    } else {
        // failed to resolve item to any form of supported preview
        // it is probably not accessible due to permissions, broken symlink etc.
        item.preview_type = Some(PreviewType::NotReadable);
    }
}

/// Check preview type.
fn set_preview_type(item: &mut ItemInfo) {
    if item.file_type == FileType::Directory
        || (item.file_type == FileType::Symlink && item.symlink_dir_path.is_some())
    {
        // symlink was resolved to directory already in the ItemInfo
        item.preview_type = Some(PreviewType::Directory);
    } else {
        set_preview_content_type(item);
    }
}

fn is_supported_ext(item: &ItemInfo) -> bool {
    match &item.file_ext {
        None => false,
        Some(ext) => IMAGE_EXTENSION.contains(&ext.as_str()),
    }
}

fn set_theme(config: &Config) -> Theme {
    match &config.theme_path {
        Some(p) => match ThemeSet::get_theme(p) {
            Ok(theme) => theme,
            Err(_) => match &config.default_theme {
                Some(dt) => choose_theme(dt),
                None => ThemeSet::load_defaults().themes["base16-ocean.dark"].clone(),
            },
        },
        None => match &config.default_theme {
            Some(dt) => choose_theme(dt),
            None => ThemeSet::load_defaults().themes["base16-ocean.dark"].clone(),
        },
    }
}

fn choose_theme(dt: &DefaultTheme) -> Theme {
    let defaults = ThemeSet::load_defaults();
    match dt {
        DefaultTheme::Base16OceanDark => defaults.themes["base16-ocean.dark"].clone(),
        DefaultTheme::Base16EightiesDark => defaults.themes["base16-eighties.dark"].clone(),
        DefaultTheme::Base16MochaDark => defaults.themes["base16-mocha.dark"].clone(),
        DefaultTheme::Base16OceanLight => defaults.themes["base16-ocean.light"].clone(),
        DefaultTheme::InspiredGitHub => defaults.themes["InspiredGitHub"].clone(),
        DefaultTheme::SolarizedDark => defaults.themes["Solarized (dark)"].clone(),
        DefaultTheme::SolarizedLight => defaults.themes["Solarized (light)"].clone(),
    }
}

#[allow(dead_code)]
fn is_supported_image(item: &ItemInfo) -> bool {
    if let Ok(output) = std::process::Command::new("file")
        .args(["--mime", item.file_path.to_str().unwrap()])
        .output()
    {
        if let Ok(result) = String::from_utf8(output.stdout) {
            let v: Vec<&str> = result.split([':', ';']).collect();
            v[1].contains("image")
        } else {
            false
        }
    } else {
        false
    }
}

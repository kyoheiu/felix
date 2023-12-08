use super::config::*;
use super::errors::FxError;
use super::functions::*;
use super::help::HELP;
use super::jumplist::*;
use super::layout::*;
use super::magic_image;
use super::magic_packed;
use super::nums::*;
use super::op::*;
use super::session::*;
use super::term::*;

use chrono::prelude::*;
use crossterm::event::KeyEventKind;
use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::style::Stylize;
use log::{error, info};
use std::collections::VecDeque;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::io::Stdout;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Instant;
use std::time::UNIX_EPOCH;

#[cfg(target_family = "unix")]
use nix::sys::stat::Mode;
#[cfg(target_family = "unix")]
use nix::unistd::{Gid, Uid};
#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;

pub const BEGINNING_ROW: u16 = 3;
pub const EMPTY_WARNING: &str = "Are you sure to empty the trash directory? (if yes: y)";
const BASE32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

#[derive(Debug)]
pub struct State {
    pub list: Vec<ItemInfo>,
    pub current_dir: PathBuf,
    pub trash_dir: PathBuf,
    pub lwd_file: Option<PathBuf>,
    pub match_vim_exit_behavior: bool,
    pub has_zoxide: bool,
    pub default: String,
    pub commands: Option<BTreeMap<String, String>>,
    pub registers: Registers,
    pub operations: Operation,
    pub jumplist: JumpList,
    pub c_memo: Vec<StateMemo>,
    pub p_memo: Vec<StateMemo>,
    pub keyword: Option<String>,
    pub layout: Layout,
    pub v_start: Option<usize>,
    pub is_ro: bool,
}

#[derive(Debug)]
pub struct Registers {
    pub unnamed: Vec<ItemBuffer>,
    pub zero: Vec<ItemBuffer>,
    pub numbered: VecDeque<Vec<ItemBuffer>>,
    pub named: BTreeMap<char, Vec<ItemBuffer>>,
}

impl Registers {
    /// Append ItemBuffer to named register.
    pub fn append_item(&mut self, items: &[ItemBuffer], reg: char) -> usize {
        let v = self.named.get(&reg);
        match v {
            Some(v) => {
                let mut v = v.clone();
                v.append(&mut items.to_vec());
                self.named.insert(reg, v.to_vec());
            }
            None => {
                self.named.insert(reg, items.to_vec());
            }
        }

        items.len()
    }

    /// Register selected items to unnamed and zero registers.
    /// Also register to named when needed.
    pub fn yank_item(&mut self, items: &[ItemBuffer], reg: Option<char>, append: bool) -> usize {
        self.unnamed = items.to_vec();
        match reg {
            None => {
                self.zero = items.to_vec();
            }
            Some(c) => {
                if append {
                    self.append_item(items, c);
                } else {
                    self.named.insert(c, items.to_vec());
                }
            }
        }
        items.len()
    }

    /// Return Vec<ItemBuffer> from registers according to the KeyCode, if exists.
    pub fn check_reg(&self, code: &KeyCode) -> Option<Vec<ItemBuffer>> {
        match code {
            KeyCode::Char('"') => Some(self.unnamed.clone()),
            KeyCode::Char('0') => Some(self.zero.clone()),
            KeyCode::Char(c) => {
                if c.is_ascii_digit() {
                    self.numbered
                        .get(c.to_digit(10).unwrap() as usize - 1)
                        .cloned()
                } else if c.is_ascii_alphabetic() {
                    self.named.get(c).cloned()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Return Vec<String> that contains item names in the register.
    pub fn prepare_reg(&self, width: u16) -> Vec<String> {
        let mut s = String::new();

        //registers.unnamed
        let mut unnamed = "\"\"".to_string();
        if !self.unnamed.is_empty() {
            for b in self.unnamed.iter() {
                unnamed.push(' ');
                unnamed.push_str(&b.file_name);
            }
            unnamed.push('\n');
            s.push_str(&unnamed);
        }

        //registers.zero
        let mut zero = "\"0".to_string();
        if !self.zero.is_empty() {
            for b in self.zero.iter() {
                zero.push(' ');
                zero.push_str(&b.file_name);
            }
            zero.push('\n');
            s.push_str(&zero);
        }

        //registers.numbered
        for i in 1..=9 {
            if let Some(reg) = self.numbered.get(i - 1) {
                let mut numbered = "\"".to_string();
                numbered.push_str(&i.to_string());
                for b in reg {
                    numbered.push(' ');
                    numbered.push_str(&b.file_name);
                }
                numbered.push('\n');
                s.push_str(&numbered);
            }
        }

        //registers.named
        for (c, b) in self.named.iter() {
            let mut named = "\"".to_string();
            named.push(*c);
            for buffer in b {
                named.push(' ');
                named.push_str(&buffer.file_name);
            }
            named.push('\n');
            s.push_str(&named);
        }

        s.pop();
        split_lines_including_wide_char(&s, width.into())
    }
}

/// To avoid cost copying ItemInfo, use ItemBuffer when tinkering with register.
#[derive(Debug, Clone)]
pub struct ItemBuffer {
    pub file_type: FileType,
    pub file_name: String,
    pub file_path: std::path::PathBuf,
}

impl ItemBuffer {
    pub fn new(item: &ItemInfo) -> Self {
        ItemBuffer {
            file_type: item.file_type,
            file_name: item.file_name.clone(),
            file_path: item.file_path.clone(),
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
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
    pub is_dirty: bool,
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileType {
    Directory,
    #[default]
    File,
    Symlink,
}

impl State {
    /// Initialize the state of the app.
    pub fn new(session_path: &std::path::Path) -> Result<Self, FxError> {
        //Read config file.
        //Use default configuration if the file does not exist or cannot be read.
        let config = read_config_or_default();
        let config = match config {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Cannot read the config file properly.\nError: {}\nfelix launches with default configuration.", e);
                Config::default()
            }
        };

        let session = read_session(session_path);
        let (original_column, original_row) = terminal_size()?;

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

        let color = config.color.unwrap_or_default();

        let split = session.split.unwrap_or(Split::Vertical);

        let has_bat = check_bat();
        let has_chafa = check_chafa();
        let has_zoxide = check_zoxide();
        let is_kitty = check_kitty_support();

        Ok(State {
            list: Vec::new(),
            registers: Registers {
                unnamed: vec![],
                zero: vec![],
                numbered: VecDeque::new(),
                named: BTreeMap::new(),
            },
            operations: Operation {
                pos: 0,
                op_list: Vec::new(),
            },
            current_dir: PathBuf::new(),
            trash_dir: PathBuf::new(),
            lwd_file: None,
            match_vim_exit_behavior: config.match_vim_exit_behavior.unwrap_or_default(),
            has_zoxide,
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
                    dir_fg: color.dir_fg,
                    file_fg: color.file_fg,
                    symlink_fg: color.symlink_fg,
                    dirty_fg: color.dirty_fg,
                },
                sort_by: session.sort_by,
                show_hidden: session.show_hidden,
                side: if session.preview.unwrap_or(false) {
                    Side::Preview
                } else {
                    Side::None
                },
                split,
                preview_start: match split {
                    Split::Vertical => (0, 0),
                    Split::Horizontal => (0, 0),
                },
                preview_space: match split {
                    Split::Vertical => (0, 0),
                    Split::Horizontal => (0, 0),
                },
                has_bat,
                has_chafa,
                is_kitty,
            },
            jumplist: JumpList::default(),
            c_memo: Vec::new(),
            p_memo: Vec::new(),
            keyword: None,
            v_start: None,
            is_ro: false,
        })
    }

    /// Select item that the cursor points to.
    pub fn get_item(&self) -> Result<&ItemInfo, FxError> {
        self.list
            .get(self.layout.nums.index)
            .ok_or(FxError::GetItem)
    }

    /// Select item that the cursor points to, as mut.
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
            None => default
                .arg(path)
                .status()
                .map_err(|_| FxError::DefaultEditor),
            Some(map) => match extension {
                None => default
                    .arg(path)
                    .status()
                    .map_err(|_| FxError::DefaultEditor),
                Some(extension) => match map.get(extension) {
                    Some(command) => {
                        let command: Vec<&str> = command.split_ascii_whitespace().collect();
                        //If the key has no arguments
                        if command.len() == 1 {
                            let mut ex = Command::new(command[0]);
                            ex.arg(path)
                                .status()
                                .map_err(|e| FxError::OpenItem(e.to_string()))
                        } else {
                            let mut args: Vec<&OsStr> =
                                command[1..].iter().map(|x| x.as_ref()).collect();
                            args.push(path.as_ref());
                            let mut ex = Command::new(command[0]);
                            ex.args(args)
                                .status()
                                .map_err(|e| FxError::OpenItem(e.to_string()))
                        }
                    }
                    None => default
                        .arg(path)
                        .status()
                        .map_err(|_| FxError::DefaultEditor),
                },
            },
        }
    }

    #[cfg(any(target_os = "linux", target_os = "netbsd"))]
    /// Open the selected file in a new window, according to the config.
    pub fn open_file_in_new_window(&self) -> Result<(), FxError> {
        let item = self.get_item()?;
        let path = &item.file_path;
        let map = &self.commands;
        let extension = &item.file_ext;

        info!("OPEN(new window): {:?}", path);

        match map {
            None => Err(FxError::OpenNewWindow("No exec configuration".to_owned())),
            Some(map) => match extension {
                Some(extension) => match map.get(extension) {
                    Some(command) => match unsafe { nix::unistd::fork() } {
                        Ok(result) => match result {
                            nix::unistd::ForkResult::Parent { child } => {
                                nix::sys::wait::waitpid(Some(child), None)?;
                                Ok(())
                            }
                            nix::unistd::ForkResult::Child => {
                                nix::unistd::setsid()?;
                                let command: Vec<&str> = command.split_ascii_whitespace().collect();
                                if command.len() == 1 {
                                    let mut ex = Command::new(command[0]);
                                    ex.arg(path)
                                        .stdout(Stdio::null())
                                        .stdin(Stdio::null())
                                        .spawn()
                                        .and(Ok(()))
                                        .map_err(|e| FxError::OpenItem(e.to_string()))?;
                                    drop(ex);
                                    std::process::exit(0);
                                } else {
                                    let mut args: Vec<&OsStr> =
                                        command[1..].iter().map(|x| x.as_ref()).collect();
                                    args.push(path.as_ref());
                                    let mut ex = Command::new(command[0]);
                                    ex.args(args)
                                        .stdout(Stdio::null())
                                        .stdin(Stdio::null())
                                        .spawn()
                                        .and(Ok(()))
                                        .map_err(|e| FxError::OpenItem(e.to_string()))?;
                                    drop(ex);
                                    std::process::exit(0);
                                }
                            }
                        },
                        Err(e) => Err(FxError::Nix(e.to_string())),
                    },
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

    #[cfg(all(not(target_os = "linux"), not(target_os = "netbsd")))]
    /// Open the selected file in a new window, according to the config.
    pub fn open_file_in_new_window(&self) -> Result<(), FxError> {
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
                            .and(Ok(()))
                            .map_err(|e| (FxError::OpenItem(e.to_string())))
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

    /// Delete item.
    pub fn delete(
        &mut self,
        reg: Option<char>,
        append: bool,
        screen: &mut Stdout,
    ) -> Result<(), FxError> {
        hide_cursor();
        print_info("DELETE: Processing...", self.layout.y);
        screen.flush()?;
        let start = Instant::now();

        let target = self.get_item()?;
        let target = vec![ItemBuffer::new(target)];

        match self.move_to_trash(&target, true) {
            Err(e) => {
                return Err(e);
            }
            Ok((src, dest)) => {
                self.yank_after_delete(&src, &dest, reg, append)?;
            }
        }

        self.clear_and_show_headline();
        self.update_list()?;
        self.list_up();
        self.layout.y = if self.list.is_empty() {
            BEGINNING_ROW
        } else if self.layout.nums.index == self.list.len() {
            self.layout.nums.go_up();
            self.layout.y - 1
        } else {
            self.layout.y
        };
        let duration = duration_to_string(start.elapsed());
        print_info(format!("1 item deleted. [{}]", duration), self.layout.y);
        self.move_cursor(self.layout.y);
        Ok(())
    }

    /// Delete items in visual mode.
    pub fn delete_in_visual(
        &mut self,
        reg: Option<char>,
        append: bool,
        screen: &mut Stdout,
    ) -> Result<(), FxError> {
        print_info("DELETE: Processing...", self.layout.y);
        let start = Instant::now();
        screen.flush()?;

        let selected: Vec<ItemBuffer> = self
            .list
            .iter()
            .filter(|item| item.selected)
            .map(ItemBuffer::new)
            .collect();
        let total: usize = match self.move_to_trash(&selected, true) {
            Err(e) => {
                return Err(e);
            }
            Ok((src, dest)) => self.yank_after_delete(&src, &dest, reg, append)?,
        };

        self.update_list()?;
        let new_len = self.list.len();
        self.clear_and_show_headline();

        let duration = duration_to_string(start.elapsed());
        let delete_message: String = {
            if total == 1 {
                format!("1 item deleted [{}]", duration)
            } else {
                let mut count = total.to_string();
                let _ = write!(count, " items deleted [{}]", duration);
                count
            }
        };
        print_info(delete_message, self.layout.y);
        delete_pointer();

        self.reset_selection();
        if new_len == 0 {
            self.layout.nums.reset();
            self.list_up();
            self.move_cursor(BEGINNING_ROW);
        } else if self.is_out_of_bounds() {
            if self.layout.nums.skip as usize >= new_len {
                self.layout.nums.skip = (new_len - 1) as u16;
                self.layout.nums.index = self.list.len() - 1;
                self.list_up();
                self.move_cursor(BEGINNING_ROW);
            } else {
                self.layout.nums.index = self.list.len() - 1;
                self.list_up();
                self.move_cursor(
                    (self.list.len() as u16) - self.layout.nums.skip + BEGINNING_ROW - 1,
                );
            }
        } else {
            self.list_up();
            self.move_cursor(self.layout.y);
        }
        Ok(())
    }

    /// Move items from the current directory to trash directory.
    /// This does not actually delete items.
    /// If you'd like to delete, use `:empty` after this.
    fn move_to_trash(
        &mut self,
        src: &[ItemBuffer],
        new_op: bool,
    ) -> Result<(Vec<ItemBuffer>, Vec<ItemBuffer>), FxError> {
        if self.current_dir == self.trash_dir {
            return Err(FxError::Io(
                "Use `:empty` to delete item in the trash dir.".to_string(),
            ));
        }

        let total_selected = src.len();
        let mut dest = Vec::new();
        for (i, item) in src.iter().enumerate() {
            delete_pointer();
            to_info_line();
            clear_current_line();
            print!("{}", display_count(i, total_selected));

            match item.file_type {
                FileType::Directory => match self.remove_dir(item, new_op) {
                    Err(e) => {
                        return Err(e);
                    }
                    Ok(path) => dest.push(path),
                },
                FileType::File | FileType::Symlink => match self.remove_file(item, new_op) {
                    Err(e) => {
                        return Err(e);
                    }
                    Ok(path) => {
                        if let Some(p) = path {
                            dest.push(p);
                        }
                    }
                },
            }
        }

        Ok((src.to_vec(), dest))
    }

    /// Add dest to register, and item infomation to operation
    fn yank_after_delete(
        &mut self,
        src: &[ItemBuffer],
        dest: &[ItemBuffer],
        reg: Option<char>,
        append: bool,
    ) -> Result<usize, FxError> {
        if !dest.is_empty() {
            //save to unnamed reg
            self.registers.unnamed = dest.to_vec();
            //If numbered registers is full, pop_back first
            if self.registers.numbered.len() == 9 {
                self.registers.numbered.pop_back();
            }
            //save to "1
            self.registers.numbered.push_front(dest.to_vec());

            if let Some(reg) = reg {
                if append {
                    self.registers.append_item(dest, reg);
                } else {
                    self.registers.named.insert(reg, dest.to_vec());
                }
            }

            //Update operations value
            self.operations.branch();
            //push deleted item information to operations
            self.operations.push(OpKind::Delete(DeletedFiles {
                trash: dest.to_vec(),
                original: src.to_vec(),
                dir: self.current_dir.clone(),
            }));
        }

        Ok(dest.len())
    }

    /// Move single directory recursively to trash directory.
    fn remove_dir(&mut self, item: &ItemBuffer, new_op: bool) -> Result<ItemBuffer, FxError> {
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

                    let mut trash_name = chrono::Local::now().timestamp().to_string();
                    trash_name.push('_');
                    let file_name = entry.file_name().to_str();
                    if file_name.is_none() {
                        return Err(FxError::Encode);
                    }
                    trash_name.push_str(file_name.unwrap());
                    trash_path = self.trash_dir.join(&trash_name);
                    std::fs::create_dir(self.trash_dir.join(&trash_path))?;

                    continue;
                } else {
                    if entry.file_type().is_symlink() && !entry_path.exists() {
                        if std::fs::remove_file(entry_path).is_err() {
                            return Err(FxError::RemoveItem(entry_path.to_owned()));
                        }
                        continue;
                    }
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
        }

        //remove original
        if std::fs::remove_dir_all(&item.file_path).is_err() {
            return Err(FxError::RemoveItem(item.file_path.clone()));
        }

        Ok(ItemBuffer {
            file_type: item.file_type,
            file_name: item.file_name.clone(),
            file_path: trash_path,
        })
    }

    /// Move single file to trash directory.
    fn remove_file(
        &mut self,
        item: &ItemBuffer,
        new_op: bool,
    ) -> Result<Option<ItemBuffer>, FxError> {
        //prepare from and to for copy
        let from = &item.file_path;
        let mut to = PathBuf::new();

        if item.file_type == FileType::Symlink && !from.exists() {
            match std::fs::remove_file(from) {
                Ok(_) => Ok(None),
                Err(_) => Err(FxError::RemoveItem(from.to_owned())),
            }
        } else {
            let mut rename = Local::now().timestamp().to_string();
            rename.push('_');
            rename.push_str(&item.file_name);

            if new_op {
                to = self.trash_dir.join(&rename);

                //copy
                if std::fs::copy(from, &to).is_err() {
                    return Err(FxError::PutItem(from.to_owned()));
                }
            }

            //remove original
            if std::fs::remove_file(from).is_err() {
                return Err(FxError::RemoveItem(from.to_owned()));
            }

            Ok(Some(ItemBuffer {
                file_type: item.file_type,
                file_name: item.file_name.clone(),
                file_path: to,
            }))
        }
    }

    /// Put.
    pub fn put(&mut self, reg: Vec<ItemBuffer>, screen: &mut Stdout) -> Result<(), FxError> {
        //If read-only, putting is disabled.
        if self.is_ro {
            print_warning("Cannot put into this directory.", self.layout.y);
            return Ok(());
        }
        if reg.is_empty() {
            return Ok(());
        }
        print_info("PUT: Processing...", self.layout.y);
        screen.flush()?;
        let start = Instant::now();

        let total = self.put_item(&reg, None)?;

        self.reload(self.layout.y)?;

        let duration = duration_to_string(start.elapsed());
        let mut put_message = total.to_string();
        if total == 1 {
            let _ = write!(put_message, " item inserted. [{}]", duration);
        } else {
            let _ = write!(put_message, " items inserted. [{}]", duration);
        }
        print_info(put_message, self.layout.y);
        Ok(())
    }

    /// Put items in the register to the current directory or target directory.
    /// Return the total number of put items.
    /// Only Redo command uses target directory.
    fn put_item(
        &mut self,
        targets: &[ItemBuffer],
        target_dir: Option<PathBuf>,
    ) -> Result<usize, FxError> {
        //make HashSet<String> of file_name
        let mut name_set = BTreeSet::new();
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
            delete_pointer();
            to_info_line();
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
                put: put_v.clone(),
                dir: self.current_dir.clone(),
            }));
        }

        Ok(put_v.len())
    }

    /// Put single item to current or target directory.
    fn put_file(
        &mut self,
        item: &ItemBuffer,
        target_dir: &Option<PathBuf>,
        name_set: &mut BTreeSet<String>,
    ) -> Result<PathBuf, FxError> {
        let rename = rename_file(&item.file_name, name_set);
        let to = match target_dir {
            None => self.current_dir.join(&rename),
            Some(path) => path.join(&rename),
        };
        if std::fs::copy(&item.file_path, &to).is_err() {
            return Err(FxError::PutItem(item.file_path.clone()));
        }
        name_set.insert(rename);
        Ok(to.to_path_buf())
    }

    /// Put single directory recursively to current or target directory.
    fn put_dir(
        &mut self,
        item: &ItemBuffer,
        target_dir: &Option<PathBuf>,
        name_set: &mut BTreeSet<String>,
    ) -> Result<PathBuf, FxError> {
        let mut base: usize = 0;
        let mut target: PathBuf = PathBuf::new();
        let original_path = &item.file_path;

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

                let rename = rename_dir(&item.file_name, name_set);
                target = match &target_dir {
                    None => self.current_dir.join(&rename),
                    Some(path) => path.join(&rename),
                };
                name_set.insert(rename);
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

    /// Undo operations (put/delete/rename)
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
                self.put_item(&op.trash, Some(op.dir.clone()))?;
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
                self.put_item(&op.original, Some(op.dir.clone()))?;
                self.operations.pos -= 1;
                self.update_list()?;
                self.clear_and_show_headline();
                self.list_up();
                print_info("REDONE: PUT", BEGINNING_ROW);
            }
            OpKind::Delete(op) => {
                self.move_to_trash(&op.original, false)?;
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
    pub fn refresh(&mut self, column: u16, row: u16, mut cursor_pos: u16) -> Result<(), FxError> {
        let (time_start, name_max) = make_layout(column);

        let (original_column, original_row) = terminal_size()?;

        self.layout.terminal_row = row;
        self.layout.terminal_column = column;
        self.layout.preview_start = match self.layout.split {
            Split::Vertical => (column + 2, BEGINNING_ROW),
            Split::Horizontal => (1, row + 2),
        };
        self.layout.preview_space = if self.layout.is_preview() || self.layout.is_reg() {
            match self.layout.split {
                Split::Vertical => (original_column - column - 1, row - BEGINNING_ROW),
                Split::Horizontal => (column, original_row - row - 1),
            }
        } else {
            (0, 0)
        };
        self.layout.name_max_len = name_max;
        self.layout.time_start_pos = time_start;

        if cursor_pos > row - 1 {
            self.layout.nums.index -= (cursor_pos - row + 1) as usize;
            cursor_pos = row - 1;
        }

        self.redraw(cursor_pos);
        Ok(())
    }

    /// Clear all and show the current directory information.
    pub fn clear_and_show_headline(&mut self) {
        clear_all();
        move_to(1, 1);

        let mut header_space = (self.layout.terminal_column - 1) as usize;

        // Show current directory path.
        // crossterm's Stylize cannot be applied to PathBuf,
        // current directory does not have any text attribute for now.
        let current_dir = self.current_dir.display().to_string();
        if current_dir.bytes().len() >= header_space {
            let current_dir = shorten_str_including_wide_char(&current_dir, header_space);
            set_color_current_dir();
            print!(" {}", current_dir);
            reset_color();
            return;
        } else {
            set_color_current_dir();
            print!(" {}", current_dir);
            reset_color();
            header_space -= current_dir.len();
        }

        // If without the write permission, print [RO].
        if self.is_ro && header_space > 5 {
            set_color_read_only();
            print!(" [RO]");
            reset_color();
            header_space -= 5;
        }

        //If git repository exists, get the branch information and print it.
        if let Ok(repo) = git2::Repository::open(&self.current_dir) {
            if let Ok(head) = repo.head() {
                if let Some(branch) = head.shorthand() {
                    if branch.len() + 4 <= header_space {
                        print!(" on ",);
                        set_color_git_repo();
                        print!("{}", branch.trim().bold());
                        reset_color();
                    }
                }
            }
        }
    }

    /// Print an item in the directory.
    fn print_item(&self, item: &ItemInfo) {
        let name = if item.file_name.bytes().len() <= self.layout.name_max_len {
            item.file_name.clone()
        } else {
            let i = self.layout.name_max_len - 2;
            let mut file_name = shorten_str_including_wide_char(&item.file_name, i);
            file_name.push_str("..");
            file_name
        };
        let time = format_time(&item.modified);
        let mut color = match item.file_type {
            FileType::Directory => &self.layout.colors.dir_fg,
            FileType::File => &self.layout.colors.file_fg,
            FileType::Symlink => &self.layout.colors.symlink_fg,
        };
        if item.is_dirty {
            color = &self.layout.colors.dirty_fg;
        }

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

        // If git repository exists, get information of changed/untracked files.
        let mut dirty_paths = BTreeSet::new();
        if let Ok(repo) = git2::Repository::discover(&self.current_dir) {
            let mut opts = git2::DiffOptions::new();
            // When detecting dirty files, includes untracked files.
            opts.include_untracked(true);
            if let Ok(diff) = repo.diff_index_to_workdir(None, Some(&mut opts)) {
                // Current directory does not always point to the root (e.g. in the child dir),
                // so uses repo.path() and pop() here.
                let mut root = repo.path().to_path_buf();
                root.pop();
                diff.foreach(
                    &mut |x, _| {
                        if let Some(new_file) = x.new_file().path() {
                            let dirty_path = root.join(new_file);
                            // Changes color of ancestors to show a directory may contain
                            // dirty files.
                            for ancestor in dirty_path.ancestors() {
                                dirty_paths.insert(ancestor.to_owned());
                            }
                        }
                        true
                    },
                    None,
                    None,
                    None,
                )
                // Ignores error to continue the update_list process.
                .unwrap_or(());
            }
        }

        for entry in fs::read_dir(&self.current_dir)? {
            let e = entry?;
            let mut entry = read_item(e);
            if dirty_paths.contains(&entry.file_path) {
                entry.is_dirty = true;
            }
            match entry.file_type {
                FileType::Directory => dir_v.push(entry),
                FileType::File | FileType::Symlink => file_v.push(entry),
            }
        }

        match self.layout.sort_by {
            SortKey::Name => {
                dir_v.sort_by(|a, b| natord::compare_ignore_case(&a.file_name, &b.file_name));
                file_v.sort_by(|a, b| natord::compare_ignore_case(&a.file_name, &b.file_name));
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

    /// Change (only) the order of the list and print it.
    pub fn reorder(&mut self, y: u16) {
        self.change_order();
        self.clear_and_show_headline();
        self.list_up();
        self.move_cursor(y);
    }

    /// Change the order of the list without re-reading all the items.
    fn change_order(&mut self) {
        let mut dir_v = Vec::new();
        let mut file_v = Vec::new();
        let mut result = Vec::with_capacity(self.list.len());

        for item in self.list.iter_mut() {
            if item.file_type == FileType::Directory {
                dir_v.push(std::mem::take(item));
            } else {
                file_v.push(std::mem::take(item));
            }
        }

        match self.layout.sort_by {
            SortKey::Name => {
                dir_v.sort_by(|a, b| natord::compare_ignore_case(&a.file_name, &b.file_name));
                file_v.sort_by(|a, b| natord::compare_ignore_case(&a.file_name, &b.file_name));
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
    }

    /// Reset all item's selected state and exit the select mode.
    pub fn reset_selection(&mut self) {
        for item in self.list.iter_mut() {
            item.selected = false;
        }
        self.v_start = None;
    }

    /// Highlight matched items.
    pub fn highlight_matches(&mut self, keyword: &str) {
        for item in self.list.iter_mut() {
            item.matches = item.file_name.contains(keyword);
        }
    }

    /// Select items from the top to current position.
    pub fn select_from_top(&mut self, start_pos: usize) {
        for (i, item) in self.list.iter_mut().enumerate() {
            item.selected = i <= start_pos;
        }
    }

    /// Select items from the current position to bottom.
    pub fn select_to_bottom(&mut self, start_pos: usize) {
        for (i, item) in self.list.iter_mut().enumerate() {
            item.selected = i >= start_pos;
        }
    }

    /// Creates temp file for directory. Works like touch, but with randomized suffix
    #[allow(dead_code)]
    pub fn create_temp(&mut self, is_dir: bool) -> Result<PathBuf, FxError> {
        let mut new_name = self.current_dir.join(".tmp");
        if new_name.exists() {
            let mut nanos = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .subsec_nanos();
            let encoded: &mut [u8] = &mut [0, 0, 0, 0, 0];
            for i in 0..5 {
                let v = (nanos & 0x1f) as usize;
                encoded[4 - i] = BASE32[v];
                nanos >>= 5;
            }
            new_name = self
                .current_dir
                .join(format!(".tmp_{}", String::from_utf8(encoded.to_vec())?))
        }
        if is_dir {
            std::fs::create_dir(new_name.clone())?;
        } else {
            std::fs::File::create(new_name.clone())?;
        }
        Ok(new_name)
    }

    /// Show help
    pub fn show_help(&self, mut screen: &Stdout) -> Result<(), FxError> {
        clear_all();
        move_to(1, 1);
        screen.flush()?;
        let (width, height) = terminal_size()?;
        let help = format_txt(HELP, width, true);
        print_help(&help, 0, height);
        screen.flush()?;

        let mut skip = 0;
        loop {
            if let Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) = crossterm::event::read()?
            {
                match code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        clear_all();
                        skip += 1;
                        print_help(&help, skip, height);
                        screen.flush()?;
                        continue;
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if skip == 0 {
                            continue;
                        } else {
                            clear_all();
                            skip -= 1;
                            print_help(&help, skip, height);
                            screen.flush()?;
                            continue;
                        }
                    }
                    _ => {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// Empty the trash dir.
    pub fn empty_trash(&mut self, mut screen: &Stdout) -> Result<(), FxError> {
        print_warning(EMPTY_WARNING, self.layout.y);
        screen.flush()?;

        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = crossterm::event::read()?
        {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    print_info("EMPTY: Processing...", self.layout.y);
                    screen.flush()?;

                    //Delete trash dir.
                    if let Err(e) = std::fs::remove_dir_all(&self.trash_dir) {
                        print_warning(e, self.layout.y);
                    }
                    //Recreate the dir.
                    if let Err(e) = std::fs::create_dir(&self.trash_dir) {
                        print_warning(e, self.layout.y);
                    }
                    if self.current_dir == self.trash_dir {
                        self.reload(BEGINNING_ROW)?;
                    }
                    go_to_info_line_and_reset();
                    print_info("Trash dir emptied", self.layout.y);
                    self.move_cursor(self.layout.y);
                    screen.flush()?;
                }
                _ => {
                    go_to_info_line_and_reset();
                    self.move_cursor(self.layout.y);
                }
            }
        }
        Ok(())
    }

    /// Change directory.
    pub fn chdir(&mut self, p: &std::path::Path, mv: Move) -> Result<(), FxError> {
        std::env::set_current_dir(p)?;

        self.is_ro = match has_write_permission(p) {
            Ok(b) => !b,
            Err(_) => false,
        };
        match mv {
            Move::Up => {
                // Add the new directory path to jumplist
                self.jumplist.add(p);

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
                        self.keyword = None;
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
                                self.layout.nums.reset();
                                self.redraw(BEGINNING_ROW);
                            }
                        }
                    }
                }
            }
            Move::Down => {
                // Add the new directory path to jumplist
                self.jumplist.add(p);

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
                // Add the new directory path to jumplist
                self.jumplist.add(p);

                self.current_dir = p.to_owned();
                self.keyword = None;
                self.p_memo = Vec::new();
                self.c_memo = Vec::new();
                self.layout.nums.reset();
                self.reload(BEGINNING_ROW)?;
            }
            Move::List => {
                self.current_dir = p.to_owned();
                self.keyword = None;
                self.p_memo = Vec::new();
                self.c_memo = Vec::new();
                self.layout.nums.reset();
                self.reload(BEGINNING_ROW)?;
            }
        }
        //if zoxide is installed, add the target or increment its rank.
        if self.has_zoxide {
            if let Some(p) = p.as_os_str().to_str() {
                if std::process::Command::new("zoxide")
                    .args(["add", p])
                    .output()
                    .is_err()
                {
                    print_warning("Failed to `zoxide add`.", self.layout.y);
                }
            }
        }
        self.v_start = None;
        Ok(())
    }

    /// For subsequent use by cd in the parent shell
    pub fn export_lwd(&self) -> Result<(), ()> {
        if let Some(lwd_file) = &self.lwd_file {
            std::fs::write(lwd_file, self.current_dir.to_str().unwrap()).map_err(|_| {
                print_warning(
                    format!(
                        "Couldn't write the LWD to file {0}!",
                        lwd_file.as_path().to_string_lossy()
                    ),
                    self.layout.y,
                );
            })
        } else {
            print_warning("Shell integration may not be configured.", self.layout.y);
            Err(())
        }
    }

    /// Change the cursor position, and print item information at the bottom.
    /// If preview is enabled, print text preview, contents of the directory or image preview.
    pub fn move_cursor(&mut self, y: u16) {
        // If preview is enabled, set the preview type, read the content (if text type) and reset the scroll.
        if self.layout.is_preview() {
            if let Ok(item) = self.get_item_mut() {
                if item.preview_type.is_none() {
                    set_preview_type(item);
                }
                item.preview_scroll = 0;
            }
        }

        delete_pointer();

        if self.layout.is_reg() {
            //Print registers by :reg
            let reg = self.registers.prepare_reg(self.layout.preview_space.0);
            self.layout.print_reg(&reg);
        }

        let item = self.get_item().ok();
        //Print item information at the bottom
        self.print_footer(item);
        if self.layout.is_preview() {
            //Print preview if preview is on
            self.layout.print_preview(item, y);
        }

        move_to(1, y);
        print_pointer();

        //Store cursor position when cursor moves
        self.layout.y = y;
    }

    fn to_status_bar(&self) {
        move_to(1, self.layout.terminal_row);
    }

    /// Clear status line.
    fn clear_status_line(&self) {
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
    fn print_footer(&self, item: Option<&ItemInfo>) {
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

        if let Some(item) = item {
            let footer = self.make_footer(item);
            print!("{}", footer.negative());
        }
    }

    /// Return footer string.
    fn make_footer(&self, item: &ItemInfo) -> String {
        match &item.file_ext {
            Some(ext) => {
                let footer = match item.permissions {
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
                footer
                    .chars()
                    .take(self.layout.terminal_column.into())
                    .collect()
            }
            None => {
                let footer = match item.permissions {
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
                footer
                    .chars()
                    .take(self.layout.terminal_column.into())
                    .collect()
            }
        }
    }

    /// Scroll down previewed text.
    pub fn scroll_down_preview(&mut self, y: u16) {
        if let Ok(item) = self.get_item_mut() {
            item.preview_scroll += 1;
            self.scroll_preview(y)
        }
    }

    /// Scroll up previewed text.
    pub fn scroll_up_preview(&mut self, y: u16) {
        if let Ok(item) = self.get_item_mut() {
            if item.preview_scroll != 0 {
                item.preview_scroll -= 1;
                self.scroll_preview(y)
            }
        }
    }

    /// Scroll preview.
    fn scroll_preview(&self, y: u16) {
        self.layout.print_preview(self.get_item().ok(), y);
        move_to(1, y);
        print_pointer();
    }

    /// Save the sort key and whether to show hidden items to session file.
    pub fn write_session(&self, session_path: PathBuf) -> Result<(), FxError> {
        let session = Session {
            sort_by: self.layout.sort_by.clone(),
            show_hidden: self.layout.show_hidden,
            preview: Some(self.layout.is_preview()),
            split: Some(self.layout.split),
        };
        let serialized = serde_yaml::to_string(&session)?;
        fs::write(session_path, serialized)?;
        Ok(())
    }

    /// Unpack or unarchive a file.
    pub fn unpack(&self) -> Result<(), FxError> {
        let item = self.get_item()?;
        let p = item.file_path.clone();

        let mut name_set: BTreeSet<String> = BTreeSet::new();
        for item in self.list.iter() {
            name_set.insert(item.file_name.clone());
        }

        let dest_name = rename_dir(&item.file_name, &name_set);
        let mut dest = self.current_dir.clone();
        dest.push(dest_name);

        magic_packed::unpack(&p, &dest)?;
        Ok(())
    }

    /// Check if the cursor is out of bounds.
    pub fn is_out_of_bounds(&self) -> bool {
        let current = self.layout.nums.skip + self.layout.y - BEGINNING_ROW + 1;
        current as usize > self.list.len()
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
                is_dirty: false,
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
            is_dirty: false,
        },
    }
}

/// Generate item information from trash directory, in order to use when redoing.
// pub fn sellect_buffer(trash_dir: &PathBuf, vec: &[ItemBuffer]) -> Result<Vec<ItemBuffer>, FxError> {
//     let total = vec.len();
//     let mut count = 0;
//     let mut result = Vec::new();
//     for entry in fs::read_dir(trash_dir)? {
//         let entry = entry?;
//         if vec
//             .iter()
//             .map(|x| x.file_path.clone())
//             .collect::<Vec<PathBuf>>()
//             .contains(&entry.path())
//         {
//             result.push(ItemBuffer::new(&read_item(entry)));
//             count += 1;
//             if count == total {
//                 break;
//             }
//         }
//     }
//     Ok(result)
// }

/// Check if bat is installed.
fn check_bat() -> bool {
    std::process::Command::new("bat")
        .arg("--help")
        .output()
        .is_ok()
}

/// Check if chafa is installed.
fn check_chafa() -> bool {
    std::process::Command::new("chafa")
        .arg("--help")
        .output()
        .is_ok()
}

/// Check if zoxide is installed.
fn check_zoxide() -> bool {
    std::process::Command::new("zoxide")
        .arg("--help")
        .output()
        .is_ok()
}

/// Check if the terminal is Kitty or not
fn check_kitty_support() -> bool {
    if let Ok(term) = std::env::var("TERM") {
        term.contains("kitty")
    } else {
        false
    }
}

/// Set content type from ItemInfo.
fn set_preview_content_type(item: &mut ItemInfo) {
    if item.file_size > MAX_SIZE_TO_PREVIEW {
        item.preview_type = Some(PreviewType::TooBigSize);
    } else if is_supported_image(item) {
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

/// Check if item is supported image type.
fn is_supported_image(item: &ItemInfo) -> bool {
    magic_image::is_supported_image_type(&item.file_path)
}

// Check if the current process has the write permission to a path.
// Currently available in unix only.
// TODO: Use this function to determine if deleting items can be done in the first place?
#[cfg(target_family = "unix")]
pub fn has_write_permission(path: &std::path::Path) -> Result<bool, FxError> {
    let metadata = std::fs::metadata(path)?;
    let mode = metadata.mode();
    if mode == 0 {
        Ok(false)
    } else {
        let euid = Uid::effective();
        if euid.is_root() {
            Ok(true)
        } else {
            let uid = metadata.uid();
            let gid = metadata.gid();

            if uid == euid.as_raw() {
                Ok((mode & Mode::S_IWUSR.bits() as u32) != 0)
            } else if gid == Gid::effective().as_raw() || in_groups(gid) {
                Ok((mode & Mode::S_IWGRP.bits() as u32) != 0)
            } else {
                Ok((mode & Mode::S_IWOTH.bits() as u32) != 0)
            }
        }
    }
}

// Currently on non-unix OS, this always returns true.
#[cfg(not(target_family = "unix"))]
pub fn has_write_permission(_path: &std::path::Path) -> Result<bool, FxError> {
    Ok(true)
}

#[cfg(all(target_family = "unix", not(target_os = "macos")))]
fn in_groups(gid: u32) -> bool {
    if let Ok(groups) = nix::unistd::getgroups() {
        for group in groups {
            if group.as_raw() == gid {
                return true;
            } else {
                continue;
            }
        }
        false
    } else {
        false
    }
}

#[cfg(target_os = "macos")]
fn in_groups(_gid: u32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    use devtimer::run_benchmark;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    fn bench_update1() -> Result<(), FxError> {
        let mut dir_v = Vec::new();
        let mut file_v = Vec::new();
        for entry in fs::read_dir("src")? {
            let e = entry?;
            let entry = read_item(e);
            match entry.file_type {
                FileType::Directory => dir_v.push(entry),
                FileType::File | FileType::Symlink => file_v.push(entry),
            }
        }
        Ok(())
    }

    fn bench_update2() -> Result<(), FxError> {
        let mut dir_v = Vec::new();
        let mut file_v = Vec::new();
        let mut temp = Vec::new();
        for entry in fs::read_dir("src")? {
            let e = entry?;
            temp.push(e);
        }

        let temp: Vec<ItemInfo> = temp.into_par_iter().map(read_item).collect();

        for entry in temp {
            match entry.file_type {
                FileType::Directory => dir_v.push(entry),
                FileType::File | FileType::Symlink => file_v.push(entry),
            }
        }

        Ok(())
    }

    #[test]
    fn test_has_write_permission() {
        // chmod to 444 and check if it's read-only
        let p = std::path::PathBuf::from("./testfiles/permission_test");
        let _status = std::process::Command::new("chmod")
            .args(["444", "./testfiles/permission_test"])
            .status()
            .unwrap();
        assert!(!has_write_permission(p.as_path()).unwrap());
        let _status = std::process::Command::new("chmod")
            .args(["755", "./testfiles/permission_test"])
            .status()
            .unwrap();

        // Test the home directory, which should pass
        let home_dir = dirs::home_dir().unwrap();
        assert!(has_write_permission(&home_dir).unwrap());
    }

    #[test]
    fn bench_update_single() {
        let bench_result = run_benchmark(100, |_| {
            // Fake a long running operation
            bench_update1().unwrap();
        });
        bench_result.print_stats();
    }

    #[test]
    fn bench_update_parallel() {
        let bench_result = run_benchmark(100, |_| {
            // Fake a long running operation
            bench_update2().unwrap();
        });
        bench_result.print_stats();
    }
}

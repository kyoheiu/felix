use super::config::*;
use super::errors::FxError;
use super::functions::*;
use super::nums::*;
use super::session::{read_session, SortKey};
use super::state::{ItemInfo, BEGINNING_ROW};
use super::term::*;

use log::error;
use serde::{Deserialize, Serialize};

pub const MAX_SIZE_TO_PREVIEW: u64 = 1_000_000_000;
pub const CHAFA_WARNING: &str =
    "From v1.1.0, the image preview needs chafa (>= v1.10.0). For more details, please see help by `:h` ";

pub const PROPER_WIDTH: u16 = 28;
pub const TIME_WIDTH: u16 = 16;
const EXTRA_SPACES: u16 = 3;

#[derive(Debug, Default)]
pub struct Layout {
    pub nums: Num,
    pub y: u16,
    pub terminal_row: u16,
    pub terminal_column: u16,
    pub name_max_len: usize,
    pub time_start_pos: u16,
    pub colors: ConfigColor,
    pub sort_by: SortKey,
    pub show_hidden: bool,
    pub side: Side,
    pub split: Split,
    pub preview_start: (u16, u16),
    pub preview_space: (u16, u16),
    pub has_chafa: bool,
    pub has_bat: bool,
    pub is_kitty: bool,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum PreviewType {
    NotReadable,
    TooBigSize,
    Directory,
    Image,
    Text,
    Binary,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Default)]
pub enum Side {
    #[default]
    Preview,
    Reg,
    None,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Copy, Default)]
pub enum Split {
    #[default]
    Vertical,
    Horizontal,
}

impl Layout {
    pub fn new(session_path: &std::path::Path, config: Config) -> Result<Self, FxError> {
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

        // Prepare state fields.
        let (time_start, name_max) = make_layout(original_column);
        let session = read_session(session_path);
        let split = session.split.unwrap_or_default();
        let has_bat = check_bat();
        let has_chafa = check_chafa();
        let is_kitty = check_kitty_support();

        let colors = config.color.unwrap_or_default();

        Ok(Layout {
            nums: Num::new(),
            y: BEGINNING_ROW,
            terminal_row: original_row,
            terminal_column: original_column,
            name_max_len: name_max,
            time_start_pos: time_start,
            sort_by: session.sort_by,
            show_hidden: session.show_hidden,
            side: match session.preview.unwrap_or(false) {
                true => Side::Preview,
                false => Side::None,
            },
            split,
            preview_start: (0, 0),
            preview_space: (0, 0),
            has_bat,
            has_chafa,
            is_kitty,
            colors,
        })
    }

    pub fn is_preview(&self) -> bool {
        self.side == Side::Preview
    }

    pub fn is_reg(&self) -> bool {
        self.side == Side::Reg
    }

    pub fn show_preview(&mut self) {
        self.side = Side::Preview;
    }

    pub fn show_reg(&mut self) {
        self.side = Side::Reg;
    }

    pub fn reset_side(&mut self) {
        self.side = Side::None;
    }

    pub fn print_reg(&self, reg: &[String]) {
        match self.split {
            Split::Vertical => {
                self.clear_preview(self.preview_start.0);
            }
            Split::Horizontal => {
                self.clear_preview(self.preview_start.1);
            }
        }

        if reg.iter().all(|x| x.is_empty()) {
            print!("No registers found.");
            return;
        }

        match self.split {
            Split::Vertical => {
                for (i, line) in reg.iter().enumerate() {
                    let row = self.preview_start.1 + i as u16;
                    move_to(self.preview_start.0, row);
                    print!("{}", line);
                    if i as u16 == self.preview_space.1 - 1 {
                        break;
                    }
                }
            }
            Split::Horizontal => {
                for (i, line) in reg.iter().enumerate() {
                    let row = self.preview_start.1 + i as u16;
                    move_to(1, row);
                    print!("{}", line);
                    if row == self.terminal_row + self.preview_space.1 {
                        break;
                    }
                }
            }
        }
    }

    /// Print preview according to the preview type.
    pub fn print_preview(&self, item: Option<&ItemInfo>, y: u16) {
        if let Some(item) = item {
            match self.split {
                Split::Vertical => {
                    //At least print the item name
                    self.print_file_name(item);
                    //Clear preview space
                    self.clear_preview(self.preview_start.0);
                }
                Split::Horizontal => {
                    self.clear_preview(self.preview_start.1);
                }
            }

            match item.preview_type {
                Some(PreviewType::NotReadable) => {
                    print!("(file not readable)");
                }
                Some(PreviewType::TooBigSize) => {
                    print!("(file too big for preview)");
                }
                Some(PreviewType::Directory) => {
                    self.preview_directory(item);
                }
                Some(PreviewType::Image) => {
                    if self.has_chafa {
                        if let Err(e) = self.preview_image(item) {
                            print_warning(e, y);
                        }
                    } else {
                        let help = format_txt(CHAFA_WARNING, self.terminal_column - 1, false);
                        for (i, line) in help.iter().enumerate() {
                            move_to(self.preview_start.0, BEGINNING_ROW + i as u16);
                            print!("{}", line,);
                            if BEGINNING_ROW + i as u16 == self.terminal_row - 1 {
                                break;
                            }
                        }
                    }
                }
                Some(PreviewType::Text) => {
                    if let Err(e) = self.preview_text(item) {
                        print_warning(e, y);
                    }
                }
                Some(PreviewType::Binary) => {
                    print!("(Binary file)");
                }
                _ => {
                    print!("(Not Available)");
                }
            }
        }
    }

    /// Print item name at the top.
    fn print_file_name(&self, item: &ItemInfo) {
        move_to(self.preview_start.0 - 1, 1);
        clear_until_newline();
        move_right(1);
        let mut file_name = format!("[{}]", item.file_name);
        if file_name.bytes().len() > self.preview_space.0 as usize {
            file_name = shorten_str_including_wide_char(&file_name, self.preview_space.0 as usize);
        }
        print!("{}", file_name);
    }

    fn preview_text(&self, item: &ItemInfo) -> Result<(), FxError> {
        if let Some(content) = &item.content {
            if !self.has_bat {
                self.print_txt_in_preview_area(
                    item,
                    &format_txt(content, self.preview_space.0, false),
                );
            } else {
                let path = item.file_path.to_str().ok_or(FxError::InvalidPath)?;
                let output = std::process::Command::new("bat")
                    .args([
                        path,
                        "-fpP",
                        "--tabs",
                        "4",
                        "--wrap",
                        "character",
                        "--terminal-width",
                        &format!("{}", self.preview_space.0),
                    ])
                    .output()?
                    .stdout;
                let content = String::from_utf8(output)?;
                let content = content
                    .split('\n')
                    .map(|x| x.to_owned())
                    .collect::<Vec<String>>();
                self.print_txt_in_preview_area(item, &content);
            }
        }
        Ok(())
    }

    fn preview_directory(&self, item: &ItemInfo) {
        let contents = match &item.symlink_dir_path {
            None => list_up_contents(&item.file_path, self.preview_space.0),
            Some(p) => list_up_contents(p, self.preview_space.0),
        };
        if let Ok(contents) = contents {
            self.print_txt_in_preview_area(
                item,
                &format_txt(&contents, self.preview_space.0, false),
            );
        }
    }

    fn print_txt_in_preview_area(&self, item: &ItemInfo, content: &[String]) {
        match self.split {
            Split::Vertical => {
                for (i, line) in content.iter().enumerate() {
                    if i < item.preview_scroll {
                        continue;
                    }
                    let sum = (i - item.preview_scroll) as u16;
                    let row = self.preview_start.1 + sum;
                    move_to(self.preview_start.0, row);
                    set_color(&TermColor::ForeGround(&Colorname::LightBlack));
                    print!("{}", line);
                    if sum == self.preview_space.1 - 1 {
                        break;
                    }
                }
            }
            Split::Horizontal => {
                for (i, line) in content.iter().enumerate() {
                    if i < item.preview_scroll {
                        continue;
                    }
                    let sum = (i - item.preview_scroll) as u16;
                    let row = self.preview_start.1 + sum;
                    move_to(1, row);
                    set_color(&TermColor::ForeGround(&Colorname::LightBlack));
                    print!("{}", line);
                    if row == self.terminal_row + self.preview_space.1 {
                        break;
                    }
                }
            }
        }
        reset_color();
    }

    /// Print text preview on the right half of the terminal (Experimental).
    fn preview_image(&self, item: &ItemInfo) -> Result<(), FxError> {
        let wxh = match self.split {
            Split::Vertical => {
                format!("--size={}x{}", self.preview_space.0, self.preview_space.1)
            }
            Split::Horizontal => {
                format!(
                    "--size={}x{}",
                    self.preview_space.0,
                    self.preview_space.1 - 1
                )
            }
        };

        let file_path = item.file_path.to_str().ok_or(FxError::InvalidPath)?;
        let output = std::process::Command::new("chafa")
            .args(["--animate=false", &wxh, file_path])
            .output()?
            .stdout;
        let output = String::from_utf8(output)?;

        match self.split {
            Split::Vertical => {
                for (i, line) in output.lines().enumerate() {
                    print!("{}", line);
                    let next_line: u16 = BEGINNING_ROW + (i as u16) + 1;
                    move_to(self.preview_start.0, next_line);
                }
            }
            Split::Horizontal => {
                for (i, line) in output.lines().enumerate() {
                    print!("{}", line);
                    let next_line: u16 = self.preview_start.1 + (i as u16) + 1;
                    move_to(1, next_line);
                }
            }
        }
        Ok(())
    }

    /// Clear the preview space.
    fn clear_preview(&self, preview_start_point: u16) {
        match self.split {
            Split::Vertical => {
                for i in 0..=self.terminal_row {
                    move_to(preview_start_point, BEGINNING_ROW + i);
                    clear_until_newline();
                }
                move_to(self.preview_start.0, BEGINNING_ROW);
            }
            Split::Horizontal => {
                for i in 0..=self.terminal_row {
                    move_to(1, preview_start_point + i);
                    clear_until_newline();
                }
                move_to(1, preview_start_point);
            }
        }
    }

    pub fn update_column_and_row(&mut self) -> Result<(u16, u16), FxError> {
        if self.is_preview() || self.is_reg() {
            match self.split {
                Split::Vertical => Ok((self.terminal_column >> 1, self.terminal_row)),
                Split::Horizontal => Ok((self.terminal_column, self.terminal_row >> 1)),
            }
        } else {
            terminal_size()
        }
    }
}

/// Make app's layout according to terminal width and app's config.
pub fn make_layout(column: u16) -> (u16, usize) {
    let mut time_start: u16;
    let mut name_max: usize;

    if column < PROPER_WIDTH {
        time_start = column;
        name_max = (column - 2).into();
        (time_start, name_max)
    } else {
        time_start = column - TIME_WIDTH;
        name_max = (time_start - EXTRA_SPACES).into();
        let required = time_start + TIME_WIDTH - 1;
        if required > column {
            let diff = required - column;
            name_max -= diff as usize;
            time_start -= diff;
        }

        (time_start, name_max)
    }
}

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

/// Check if the terminal is Kitty or not
fn check_kitty_support() -> bool {
    if let Ok(term) = std::env::var("TERM") {
        term.contains("kitty")
    } else {
        false
    }
}

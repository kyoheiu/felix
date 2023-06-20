use super::config::*;
use super::errors::FxError;
use super::functions::*;
use super::nums::*;
use super::session::SortKey;
use super::state::{ItemInfo, BEGINNING_ROW};
use super::term::*;

use serde::{Deserialize, Serialize};
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, split_at, LinesWithEndings};

pub const MAX_SIZE_TO_PREVIEW: u64 = 1_000_000_000;
pub const CHAFA_WARNING: &str =
    "From v1.1.0, the image preview needs chafa (>= v1.10.0). For more details, please see help by `:h` ";

pub const PROPER_WIDTH: u16 = 28;
pub const TIME_WIDTH: u16 = 16;
const EXTRA_SPACES: u16 = 3;

#[derive(Debug)]
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
    pub syntax_highlight: bool,
    pub syntax_set: SyntaxSet,
    pub theme: Theme,
    pub has_chafa: bool,
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

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum Side {
    Preview,
    Reg,
    None,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Copy)]
pub enum Split {
    Vertical,
    Horizontal,
}

impl Layout {
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
        if item.is_none() {
            return;
        } else {
            let item = item.unwrap();
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
                        if let Err(e) = self.preview_image(item, y) {
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
                    if self.syntax_highlight {
                        match self.preview_text_with_highlight(item) {
                            Ok(_) => {}
                            Err(e) => {
                                print!("{}", e);
                            }
                        }
                    } else {
                        self.preview_text(item);
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

    fn preview_text(&self, item: &ItemInfo) {
        if let Some(content) = &item.content {
            self.print_txt_in_preview_area(
                item,
                &format_txt(content, self.preview_space.0, false),
                false,
            );
        }
    }

    /// Preview text with syntax highlighting.
    fn preview_text_with_highlight(&self, item: &ItemInfo) -> Result<(), FxError> {
        if let Ok(Some(syntax)) = self.syntax_set.find_syntax_for_file(item.file_path.clone()) {
            let mut h = HighlightLines::new(syntax, &self.theme);
            if let Some(content) = &item.content {
                move_to(self.preview_start.0, BEGINNING_ROW);
                let mut result = vec![];
                for (index, line) in LinesWithEndings::from(content).enumerate() {
                    let count = line.len() / self.preview_space.0 as usize;
                    let mut range = h.highlight_line(line, &self.syntax_set)?;
                    for _ in 0..=count + 1 {
                        let ranges = split_at(&range, (self.preview_space.0) as usize);
                        if !ranges.0.is_empty() {
                            result.push(ranges.0);
                        }
                        range = ranges.1;
                    }
                    if index > self.preview_space.1 as usize + item.preview_scroll {
                        break;
                    }
                }
                let result: Vec<String> = result
                    .iter()
                    .map(|x| as_24_bit_terminal_escaped(x, false))
                    .collect();
                self.print_txt_in_preview_area(item, &result, true);
            } else {
            }
        } else {
            self.preview_text(item);
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
                false,
            );
        }
    }

    fn print_txt_in_preview_area(
        &self,
        item: &ItemInfo,
        content: &[String],
        syntex_highlight: bool,
    ) {
        match self.split {
            Split::Vertical => {
                for (i, line) in content.iter().enumerate() {
                    if i < item.preview_scroll {
                        continue;
                    }
                    let sum = (i - item.preview_scroll) as u16;
                    let row = self.preview_start.1 + sum;
                    move_to(self.preview_start.0, row);
                    if syntex_highlight {
                        print!("{}", line);
                    } else {
                        set_color(&TermColor::ForeGround(&Colorname::LightBlack));
                        print!("{}", line);
                    }
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
                    if syntex_highlight {
                        print!("{}", line);
                    } else {
                        set_color(&TermColor::ForeGround(&Colorname::LightBlack));
                        print!("{}", line);
                    }
                    if row == self.terminal_row + self.preview_space.1 {
                        break;
                    }
                }
            }
        }
        reset_color();
    }

    /// Print text preview on the right half of the terminal (Experimental).
    fn preview_image(&self, item: &ItemInfo, y: u16) -> Result<(), FxError> {
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

        let file_path = item.file_path.to_str();
        if file_path.is_none() {
            print_warning("Cannot read the file path correctly.", y);
            return Ok(());
        }

        let output = std::process::Command::new("chafa")
            .args(["--animate=false", &wxh, file_path.unwrap()])
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

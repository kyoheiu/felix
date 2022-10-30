use super::config::*;
use super::errors::FxError;
use super::functions::*;
use super::state::{FileType, ItemInfo, BEGINNING_ROW};
use super::term::*;

use serde::{Deserialize, Serialize};
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, split_at, LinesWithEndings};

/// cf: https://docs.rs/image/latest/src/image/image.rs.html#84-112
pub const MAX_SIZE_TO_PREVIEW: u64 = 1_000_000_000;
pub const IMAGE_EXTENSION: [&str; 20] = [
    "avif", "jpg", "jpeg", "png", "gif", "webp", "tif", "tiff", "tga", "dds", "bmp", "ico", "hdr",
    "exr", "pbm", "pam", "ppm", "pgm", "ff", "farbfeld",
];
pub const CHAFA_WARNING: &str =
    "From v1.1.0, the image preview needs chafa. For more details, please see help by `:h` ";

pub const PROPER_WIDTH: u16 = 28;
pub const TIME_WIDTH: u16 = 16;
pub const DEFAULT_NAME_LENGTH: u16 = 30;

#[derive(Debug)]
pub struct Layout {
    pub y: u16,
    pub terminal_row: u16,
    pub terminal_column: u16,
    pub name_max_len: usize,
    pub time_start_pos: u16,
    pub use_full: Option<bool>,
    pub option_name_len: Option<usize>,
    pub colors: ConfigColor,
    pub preview: bool,
    pub split: Split,
    pub preview_start_column: u16,
    pub preview_start_row: u16,
    pub preview_width: u16,
    pub syntax_highlight: bool,
    pub syntax_set: SyntaxSet,
    pub theme: Theme,
    pub has_chafa: bool,
    pub is_kitty: bool,
}

pub enum PreviewType {
    NotReadable,
    TooBigSize,
    Directory,
    Image,
    Text,
    Binary,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone, Copy)]
pub enum Split {
    Vertical,
    Horizontal,
}

impl Layout {
    /// Print preview according to the preview type.
    pub fn print_preview(&self, item: &ItemInfo, y: u16) {
        match self.split {
            Split::Vertical => {
                //At least print the item name
                self.print_file_name(item);
                //Clear preview space
                self.clear_preview(self.preview_start_column);
            }
            Split::Horizontal => {
                self.clear_preview(self.preview_start_row);
            }
        }

        match check_preview_type(item) {
            PreviewType::NotReadable => {
                print!("(file not readable)");
            }
            PreviewType::TooBigSize => {
                print!("(file too big for preview)");
            }
            PreviewType::Directory => {
                self.preview_directory(item);
            }
            PreviewType::Image => {
                if self.has_chafa {
                    if let Err(e) = self.preview_image(item, y) {
                        print_warning(e, y);
                    }
                } else {
                    let help = format_txt(CHAFA_WARNING, self.terminal_column - 1, false);
                    for (i, line) in help.iter().enumerate() {
                        move_to(self.preview_start_column, BEGINNING_ROW + i as u16);
                        print!("{}", line,);
                        if BEGINNING_ROW + i as u16 == self.terminal_row - 1 {
                            break;
                        }
                    }
                }
            }
            PreviewType::Text => {
                if self.syntax_highlight {
                    self.preview_text_with_sh(item);
                } else {
                    self.preview_text(item);
                }
            }
            PreviewType::Binary => {
                print!("(Binary file)");
            }
        }
    }

    /// Print item name at the top.
    fn print_file_name(&self, item: &ItemInfo) {
        move_to(self.preview_start_column - 1, 1);
        clear_until_newline();
        move_right(1);
        let mut file_name = format!("[{}]", item.file_name);
        if file_name.len() > self.preview_width.into() {
            file_name = file_name.chars().take(self.preview_width.into()).collect();
        }
        print!("{}", file_name);
    }

    fn preview_text(&self, item: &ItemInfo) {
        let content = {
            if let Ok(content) = std::fs::read_to_string(item.file_path.clone()) {
                let content = content.replace('\t', "    ");
                format_txt(&content, self.terminal_column - 1, false)
            } else {
                vec![]
            }
        };
        match self.split {
            Split::Vertical => {
                for (i, line) in content.iter().enumerate() {
                    move_to(self.preview_start_column, BEGINNING_ROW + i as u16);
                    set_color(&TermColor::ForeGround(&Colorname::LightBlack));
                    print!("{}", line);
                    reset_color();
                    if BEGINNING_ROW + i as u16 == self.terminal_row - 1 {
                        break;
                    }
                }
            }
            Split::Horizontal => {
                for (i, line) in content.iter().enumerate() {
                    let row = self.preview_start_row + i as u16;
                    move_to(1, row);
                    set_color(&TermColor::ForeGround(&Colorname::LightBlack));
                    print!("{}", line);
                    reset_color();
                    if row == self.terminal_row - 1 {
                        break;
                    }
                }
            }
        }
    }

    /// Preview text with syntax highlighting.
    fn preview_text_with_sh(&self, item: &ItemInfo) {
        if let Ok(Some(syntax)) = self.syntax_set.find_syntax_for_file(item.file_path.clone()) {
            let mut h = HighlightLines::new(syntax, &self.theme);
            if let Ok(content) = std::fs::read_to_string(item.file_path.clone()) {
                move_to(self.preview_start_column, BEGINNING_ROW);
                let mut result = vec![];
                let max_row = match self.split {
                    Split::Vertical => self.terminal_row - BEGINNING_ROW,
                    Split::Horizontal => self.terminal_row - 1,
                };
                'outer: for line in LinesWithEndings::from(&content) {
                    let count = line.len() / self.preview_width as usize;
                    let mut range = h.highlight_line(line, &self.syntax_set).unwrap();
                    for _ in 0..=count + 1 {
                        let ranges = split_at(&range, self.preview_width.into());
                        if !ranges.0.is_empty() {
                            result.push(ranges.0);
                        }
                        if result.len() == max_row as usize {
                            break 'outer;
                        } else {
                            range = ranges.1;
                            continue;
                        }
                    }
                }
                for (i, line) in result.iter().enumerate() {
                    let escaped = as_24_bit_terminal_escaped(line, false);
                    match self.split {
                        Split::Vertical => {
                            move_to(self.preview_start_column, BEGINNING_ROW + i as u16);
                        }
                        Split::Horizontal => {
                            move_to(1, self.preview_start_row + i as u16);
                        }
                    }
                    print!("{}", escaped);
                }
                reset_color();
            }
        } else {
            self.preview_text(item);
        }
    }

    fn preview_directory(&self, item: &ItemInfo) {
        let content = {
            let contents = match &item.symlink_dir_path {
                None => list_up_contents(&item.file_path),
                Some(p) => list_up_contents(p),
            };
            if let Ok(contents) = contents {
                if let Ok(contents) = make_tree(contents) {
                    format_txt(&contents, self.preview_width, false)
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        };

        //Print preview (wrapping)
        match self.split {
            Split::Vertical => {
                for (i, line) in content.iter().enumerate() {
                    move_to(self.preview_start_column, BEGINNING_ROW + i as u16);
                    set_color(&TermColor::ForeGround(&Colorname::LightBlack));
                    print!("{}", line);
                    reset_color();
                    if BEGINNING_ROW + i as u16 == self.terminal_row - 1 {
                        break;
                    }
                }
            }
            Split::Horizontal => {
                for (i, line) in content.iter().enumerate() {
                    let row = self.preview_start_row + i as u16;
                    move_to(1, row);
                    set_color(&TermColor::ForeGround(&Colorname::LightBlack));
                    print!("{}", line);
                    reset_color();
                    if row == self.terminal_row - 1 {
                        break;
                    }
                }
            }
        }
    }
    /// Print text preview on the right half of the terminal (Experimental).
    fn preview_image(&self, item: &ItemInfo, y: u16) -> Result<(), FxError> {
        let wxh = match self.split {
            Split::Vertical => {
                format!(
                    "--size={}x{}",
                    self.preview_width,
                    self.terminal_row - BEGINNING_ROW
                )
            }
            Split::Horizontal => {
                format!("--size={}x{}", self.preview_width, self.terminal_row)
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
        let output = String::from_utf8(output).unwrap();

        match self.split {
            Split::Vertical => {
                for (i, line) in output.lines().enumerate() {
                    print!("{}", line);
                    let next_line: u16 = BEGINNING_ROW + (i as u16) + 1;
                    move_to(self.preview_start_column, next_line);
                }
            }
            Split::Horizontal => {
                for (i, line) in output.lines().enumerate() {
                    print!("{}", line);
                    let next_line: u16 = self.preview_start_row + (i as u16) + 1;
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
                for i in 0..self.terminal_row {
                    move_to(preview_start_point, BEGINNING_ROW + i as u16);
                    clear_until_newline();
                }
                move_to(self.preview_start_column, BEGINNING_ROW);
            }
            Split::Horizontal => {
                for i in 0..self.terminal_row {
                    move_to(1, preview_start_point + i as u16);
                    clear_until_newline();
                }
                move_to(1, preview_start_point);
            }
        }
    }
}

/// Make app's layout according to terminal width and app's config.
pub fn make_layout(
    column: u16,
    use_full: Option<bool>,
    name_length: Option<usize>,
) -> (u16, usize) {
    let mut time_start: u16;
    let mut name_max: usize;

    if column < PROPER_WIDTH {
        time_start = column;
        name_max = (column - 2).into();
        (time_start, name_max)
    } else {
        match use_full {
            Some(true) | None => {
                time_start = column - TIME_WIDTH;
                name_max = (time_start - SPACES).into();
            }
            Some(false) => match name_length {
                Some(option_max) => {
                    time_start = option_max as u16 + SPACES;
                    name_max = option_max;
                }
                None => {
                    time_start = if column >= DEFAULT_NAME_LENGTH + TIME_WIDTH + SPACES {
                        DEFAULT_NAME_LENGTH + SPACES
                    } else {
                        column - TIME_WIDTH
                    };
                    name_max = if column >= DEFAULT_NAME_LENGTH + TIME_WIDTH + SPACES {
                        DEFAULT_NAME_LENGTH.into()
                    } else {
                        (time_start - SPACES).into()
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

        (time_start, name_max)
    }
}

fn check_preview_content_type(item: &ItemInfo) -> PreviewType {
    if item.file_size > MAX_SIZE_TO_PREVIEW {
        PreviewType::TooBigSize
    } else if is_supported_ext(item) {
        PreviewType::Image
    } else if let Ok(content) = &std::fs::read(&item.file_path) {
        if content_inspector::inspect(content).is_text() {
            PreviewType::Text
        } else {
            PreviewType::Binary
        }
    } else {
        // failed to resolve item to any form of supported preview
        // it is probably not accessible due to permissions, broken symlink etc.
        PreviewType::NotReadable
    }
}

/// Check preview type.
fn check_preview_type(item: &ItemInfo) -> PreviewType {
    if item.file_type == FileType::Directory
        || (item.file_type == FileType::Symlink && item.symlink_dir_path.is_some())
    {
        // symlink was resolved to directory already in the ItemInfo
        PreviewType::Directory
    } else {
        check_preview_content_type(item)
    }
}

fn is_supported_ext(item: &ItemInfo) -> bool {
    match &item.file_ext {
        None => false,
        Some(ext) => IMAGE_EXTENSION.contains(&ext.as_str()),
    }
}

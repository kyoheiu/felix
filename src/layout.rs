use super::config::*;
use super::errors::FxError;
use super::functions::*;
use super::state::{FileType, ItemInfo, BEGINNING_ROW};
use super::term::*;
use crossterm::style::Color;

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

#[derive(Debug, Clone)]
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
    pub preview_start_column: u16,
    pub preview_width: u16,
    pub has_chafa: bool,
    pub is_kitty: bool,
}

pub enum PreviewType {
    TooBigSize,
    Directory,
    Image,
    Text,
    Binary,
}

impl Layout {
    /// Print preview according to the preview type.
    pub fn print_preview(&self, item: &ItemInfo, y: u16) {
        //At least print the item name
        self.print_file_name(item);
        //Clear preview space
        self.clear_preview(self.preview_start_column);

        match check_preview_type(item) {
            PreviewType::TooBigSize => {
                print!("(Too big size to preview)");
            }
            PreviewType::Directory => {
                self.preview_content(item, true);
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
                self.preview_content(item, false);
            }
            PreviewType::Binary => {
                print!("(Binary file)");
            }
        }
    }

    /// Print item name at the top.
    fn print_file_name(&self, item: &ItemInfo) {
        move_to(self.preview_start_column, 1);
        clear_until_newline();
        let mut file_name = format!("[{}]", item.file_name);
        if file_name.len() > self.preview_width.into() {
            file_name = file_name.chars().take(self.preview_width.into()).collect();
        }
        print!("{}", file_name);
    }

    /// Print text preview on the right half of the terminal.
    fn preview_content(&self, item: &ItemInfo, is_dir: bool) {
        let content = if is_dir {
            if let Ok(content) = list_up_contents(item.file_path.clone()) {
                if let Ok(content) = make_tree(content) {
                    format_txt(&content, self.preview_width, false)
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            let item = item.file_path.clone();
            let column = self.terminal_column;
            let content = std::fs::read_to_string(item);
            if let Ok(content) = content {
                let content = content.replace('\t', "    ");
                format_txt(&content, column - 1, false)
            } else {
                vec![]
            }
        };

        //Print preview (wrapping)
        for (i, line) in content.iter().enumerate() {
            move_to(self.preview_start_column, BEGINNING_ROW + i as u16);
            set_color(Some(Color::DarkGrey), None);
            print!("{}", line);
            reset_color();
            if BEGINNING_ROW + i as u16 == self.terminal_row - 1 {
                break;
            }
        }
    }

    /// Print text preview on the right half of the terminal (Experimental).
    fn preview_image(&self, item: &ItemInfo, y: u16) -> Result<(), FxError> {
        let wxh = format!(
            "--size={}x{}",
            self.preview_width,
            self.terminal_row - BEGINNING_ROW
        );

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
        for (i, line) in output.lines().enumerate() {
            let next_line: u16 = BEGINNING_ROW + (i as u16) + 1;
            print!("{}", line);
            move_to(self.preview_start_column, next_line);
        }
        Ok(())
    }

    /// Clear the preview space.
    fn clear_preview(&self, preview_start_column: u16) {
        for i in 0..self.terminal_row {
            move_to(preview_start_column, BEGINNING_ROW + i as u16);
            clear_until_newline();
        }
        move_to(self.preview_start_column, BEGINNING_ROW);
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

/// Check preview type.
fn check_preview_type(item: &ItemInfo) -> PreviewType {
    if item.file_size > MAX_SIZE_TO_PREVIEW {
        PreviewType::TooBigSize
    } else if item.file_type == FileType::Directory {
        PreviewType::Directory
    } else if is_supported_ext(item) {
        PreviewType::Image
    } else {
        let content_type = content_inspector::inspect(&std::fs::read(&item.file_path).unwrap());
        if content_type.is_text() {
            PreviewType::Text
        } else {
            PreviewType::Binary
        }
    }
}

fn is_supported_ext(item: &ItemInfo) -> bool {
    match &item.file_ext {
        None => false,
        Some(ext) => IMAGE_EXTENSION.contains(&ext.as_str()),
    }
}

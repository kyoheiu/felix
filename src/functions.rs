use super::config::Colorname;
use super::errors::FxError;
use super::term::*;

use crossterm::style::Stylize;
use log::{info, warn};
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub const PROCESS_INDICATOR_LENGTH: u16 = 7;
const KB: u64 = 1000;
const MB: u64 = 1_000_000;
const GB: u64 = 1_000_000_000;

/// Generate modified time as `String`.
pub fn format_time(time: &Option<String>) -> String {
    match time {
        Some(datetime) => format!("{} {}", &datetime[0..10], &datetime[11..16]),
        None => "".to_string(),
    }
}

/// Rename the put file, in order to avoid the name conflict.
pub fn rename_file(file_name: &str, name_set: &BTreeSet<String>) -> String {
    let mut count: usize = 1;
    let (stem, extension) = {
        let file_name = PathBuf::from(file_name);
        (
            file_name.file_stem().unwrap().to_owned(),
            file_name.extension().map(|x| x.to_owned()),
        )
    };
    let mut new_name = file_name.to_owned();

    while name_set.contains(&new_name) {
        let mut suffix = OsString::from("_");
        suffix.push({
            let count: OsString = count.to_string().into();
            count
        });
        let mut rename = stem.to_os_string();
        if let Some(ref ext) = extension {
            rename.push(suffix);
            rename.push(".");
            rename.push(ext);
        } else {
            rename.push(suffix);
        }
        new_name = rename.into_string().unwrap();
        count += 1;
    }
    new_name
}

/// Rename the put directory, in order to avoid the name conflict.
pub fn rename_dir(dir_name: &str, name_set: &BTreeSet<String>) -> String {
    let mut count: usize = 1;
    let mut new_name = dir_name.to_owned();
    while name_set.contains(&new_name) {
        let mut suffix = "_".to_string();
        suffix.push_str(&count.to_string());
        let mut rename = dir_name.to_owned();
        rename.push_str(&suffix);
        new_name = rename;
        count += 1;
    }
    new_name
}

/// Print the result of operation, such as put/delete/redo/undo.
pub fn print_info<T: std::fmt::Display>(message: T, then: u16) {
    delete_pointer();
    go_to_info_line_and_reset();
    info!("{}", message);

    let (width, _) = terminal_size().unwrap();
    let trimmed = shorten_str_including_wide_char(&message.to_string(), (width - 1).into());
    print!("{}", trimmed);

    hide_cursor();
    move_to(1, then);
    print_pointer();
}

/// When something goes wrong or does not work, print information about it.
pub fn print_warning<T: std::fmt::Display>(message: T, then: u16) {
    delete_pointer();
    go_to_info_line_and_reset();
    warn!("{}", message);

    let (width, _) = terminal_size().unwrap();
    let trimmed = shorten_str_including_wide_char(&message.to_string(), (width - 1).into());
    set_color(&TermColor::ForeGround(&Colorname::White));
    set_color(&TermColor::BackGround(&Colorname::LightRed));
    print!("{}", trimmed);
    reset_color();

    hide_cursor();
    move_to(1, then);
    print_pointer();
}

/// Print process of put/delete.
pub fn print_process<T: std::fmt::Display>(message: T) {
    print!("{}", message);
    move_left(PROCESS_INDICATOR_LENGTH);
}

/// Print the number of process (put/delete).
pub fn display_count(i: usize, all: usize) -> String {
    let mut result = String::new();
    result.push_str(&(i + 1).to_string());
    result.push('/');
    result.push_str(&all.to_string());
    result
}

/// Convert extension setting in the config to BTreeMap.
pub fn to_extension_map(
    config: &Option<BTreeMap<String, Vec<String>>>,
) -> Option<BTreeMap<String, String>> {
    let mut new_map = BTreeMap::new();
    match config {
        Some(config) => {
            for (command, extensions) in config.iter() {
                for ext in extensions.iter() {
                    new_map.insert(ext.to_lowercase(), command.clone());
                }
            }
        }
        None => return None,
    }
    Some(new_map)
}

/// Create the duration as String. Used after print_process(put/delete).
pub fn duration_to_string(duration: Duration) -> String {
    let s = duration.as_secs_f32();
    let mut result: String = s.to_string().chars().take(4).collect();
    result.push('s');
    result
}

/// Get the size format of item.
pub fn to_proper_size(byte: u64) -> String {
    let mut result: String;
    if byte < KB {
        result = byte.to_string();
        result.push('B');
    } else if byte < MB {
        result = (byte / KB).to_string();
        result.push_str("KB");
    } else if byte < GB {
        result = (byte / MB).to_string();
        result.push_str("MB");
    } else {
        result = (byte / GB).to_string();
        result.push_str("GB");
    }
    result
}

/// Generate the contents of the directory to preview.
pub fn list_up_contents(path: &Path, width: u16) -> Result<String, FxError> {
    let mut file_v = Vec::new();
    let mut dir_v = Vec::new();
    let mut v = Vec::new();
    for item in std::fs::read_dir(path)? {
        let item = item?;
        if item.file_type()?.is_dir() {
            dir_v.push(item.file_name().into_string().unwrap_or_default());
        } else {
            file_v.push(item.file_name().into_string().unwrap_or_default());
        }
    }
    dir_v.sort_by(|a, b| natord::compare(a, b));
    file_v.sort_by(|a, b| natord::compare(a, b));
    v.append(&mut dir_v);
    v.append(&mut file_v);

    let mut result = String::new();
    let len = v.len();
    for (i, item) in v.iter().enumerate() {
        if i == len - 1 {
            let mut line = "└ ".to_string();
            line.push_str(item);
            line = shorten_str_including_wide_char(&line, width.into());
            result.push_str(&line);
        } else {
            let mut line = "├ ".to_string();
            line.push_str(item);
            line.push('\n');
            result.push_str(&line);
        }
    }
    Ok(result)
}

/// Format texts to print. Used when printing help or text preview.
pub fn format_txt(txt: &str, width: u16, is_help: bool) -> Vec<String> {
    let mut v = split_lines_including_wide_char(txt, width.into());
    if is_help {
        v.push("Press Enter to go back.".to_owned());
    }
    v
}

/// Print help text.
pub fn print_help(v: &[String], skip_number: usize, row: u16) {
    let mut row_count = 0;
    for (i, line) in v.iter().enumerate() {
        if i < skip_number {
            continue;
        }

        move_to(1, (i + 1 - skip_number) as u16);
        if row_count == row - 1 {
            print!("{}", "...".negative());
            break;
        }
        print!("{}", line);
        row_count += 1;
    }
}

/// Initialize the log if `-l` option is added.
pub fn init_log(data_local_path: &Path) -> Result<(), FxError> {
    let mut log_name = chrono::Local::now().format("%F-%H-%M-%S").to_string();
    log_name.push_str(".log");
    let config = ConfigBuilder::new()
        .set_time_offset_to_local()
        .unwrap()
        .build();
    let log_path = {
        let mut path = data_local_path.to_path_buf();
        path.push("log");
        path
    };
    if !log_path.exists() {
        std::fs::create_dir(&log_path)?;
    }
    let log_path = log_path.join(log_name);
    WriteLogger::init(LevelFilter::Info, config, std::fs::File::create(log_path)?)?;
    info!("===START===");

    Ok(())
}

/// linux-specific: Convert u32 to permission-ish string.
pub fn convert_to_permissions(permissions: u32) -> String {
    let permissions = format!("{permissions:o}");
    let permissions: String = permissions.chars().rev().take(3).collect();
    permissions.chars().rev().collect()
}

/// Shorten &str to specific width. With unicode_width, even if the string includes wide chars, it's properly split, using full width of the terminal.
pub fn shorten_str_including_wide_char(s: &str, i: usize) -> String {
    let mut result = "".to_owned();
    for c in s.chars() {
        let result_length = UnicodeWidthStr::width(result.as_str());
        if let Some(c_width) = UnicodeWidthChar::width(c) {
            if result_length + c_width > i {
                return result;
            }
            result.push(c);
            continue;
        }
    }
    result
}

pub fn split_lines_including_wide_char(s: &str, width: usize) -> Vec<String> {
    let mut result = vec![];
    let mut new_line = "".to_owned();
    for c in s.chars() {
        let new_line_length = UnicodeWidthStr::width(new_line.as_str());
        if c == '\n' {
            result.push(new_line);
            new_line = "".to_owned();
        }
        if let Some(c_width) = UnicodeWidthChar::width(c) {
            if new_line_length + c_width > width {
                result.push(new_line);
                new_line = "".to_owned();
            }
            new_line.push(c);
        }
    }
    result.push(new_line);

    result
}

//cargo test -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time() {
        let time1 = Some("1970-01-01 00:00:00".to_string());
        let time2 = None;
        assert_eq!(format_time(&time1), "1970-01-01 00:00".to_string());
        assert_eq!(format_time(&time2), "".to_string());
    }

    #[test]
    fn test_display_count() {
        assert_eq!(display_count(1, 4), "2/4".to_string());
    }

    #[test]
    fn test_proper_size() {
        assert_eq!(to_proper_size(50), "50B".to_string());
        assert_eq!(to_proper_size(2000), "2KB".to_string());
        assert_eq!(to_proper_size(3000000), "3MB".to_string());
        assert_eq!(to_proper_size(6000000000), "6GB".to_string());
    }

    #[test]
    fn test_duration_to_string() {
        assert_eq!(
            duration_to_string(Duration::from_millis(5432)),
            "5.43s".to_string()
        );
    }

    #[test]
    fn test_list_up_contents() {
        let p = PathBuf::from("./testfiles");
        let tree = list_up_contents(&p, 20).unwrap();
        assert_eq!(tree, "├ archives\n├ images\n└ permission_test".to_string());
    }

    #[test]
    fn test_convert_to_permissions() {
        let file = 33188;
        let dir = 16877;
        assert_eq!(&convert_to_permissions(file), "644");
        assert_eq!(&convert_to_permissions(dir), "755");
    }

    #[test]
    fn test_split_str_including_wide_char() {
        let teststr = "Ｈｅｌｌｏ, ｗｏｒｌｄ!";
        assert_eq!(
            "Ｈｅｌｌｏ, ｗｏｒｌ".to_owned(),
            shorten_str_including_wide_char(teststr, 20)
        );
        assert_eq!(
            "Ｈｅｌｌｏ".to_owned(),
            shorten_str_including_wide_char(teststr, 10)
        );
        assert_eq!(
            "Ｈｅｌｌｏ, ｗ".to_owned(),
            shorten_str_including_wide_char(teststr, 15)
        );
    }
}

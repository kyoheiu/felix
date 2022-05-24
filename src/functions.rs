use super::state::*;
use crate::errors::FxError;
use log::{info, warn};
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use termion::{clear, color, cursor, style};

pub const PROPER_WIDTH: u16 = 28;
pub const TIME_WIDTH: u16 = 16;
pub const DEFAULT_NAME_LENGTH: u16 = 30;
pub const SPACES: u16 = 3;

/// Generate modified time as `String`.
pub fn format_time(time: &Option<String>) -> String {
    match time {
        Some(datetime) => format!("{} {}", &datetime[0..10], &datetime[11..16]),
        None => "".to_string(),
    }
}

/// Clear all and show the current directory information.
pub fn clear_and_show(dir: &Path) {
    print!("{}{}", clear::All, cursor::Goto(1, 1));

    //Show current directory path
    print!(
        " {}{}{}{}{}",
        style::Bold,
        color::Fg(color::Cyan),
        dir.display(),
        style::Reset,
        color::Fg(color::Reset),
    );

    //If .git directory exists, get the branch information and print it.
    let git = dir.join(".git");
    if git.exists() {
        let head = git.join("HEAD");
        if let Ok(head) = std::fs::read(head) {
            let branch: Vec<u8> = head.into_iter().skip(16).collect();
            if let Ok(branch) = std::str::from_utf8(&branch) {
                print!(
                    " on {}{}{}{}{}",
                    style::Bold,
                    color::Fg(color::Magenta),
                    branch,
                    style::Reset,
                    color::Fg(color::Reset)
                );
            }
        }
    }
    //Show arrow
    reset_info_line();
}

/// Rename file when put, in order to avoid the name conflict.
pub fn rename_file(file_name: &str, name_set: &HashSet<String>) -> String {
    if name_set.contains(file_name) {
        let rename = PathBuf::from(file_name);
        let extension = rename.extension();

        let mut rename = rename.file_stem().unwrap().to_os_string();
        if let Some(ext) = extension {
            rename.push("_copied.");
            rename.push(ext);
        } else {
            rename.push("_copied");
        }

        let rename = rename
            .into_string()
            .unwrap_or_else(|_| panic!("cannot rename item."));

        rename_file(&rename, name_set)
    } else {
        file_name.to_string()
    }
}

/// Rename directory when put, in order to avoid the name conflict.
pub fn rename_dir(dir_name: &str, name_set: &HashSet<String>) -> String {
    if name_set.contains(dir_name) {
        let mut rename = dir_name.to_string();
        rename.push_str("_copied");
        rename_dir(&rename, name_set)
    } else {
        dir_name.to_string()
    }
}

pub fn reset_info_line() {
    print!("{}{}{}", cursor::Goto(2, 2), clear::CurrentLine, DOWN_ARROW);
}

pub fn delete_cursor() {
    print!(" {}", cursor::Left(1));
}

/// Print the result of operation, such as put/delete/redo/undo.
pub fn print_info<T: std::fmt::Display>(message: T, then: u16) {
    delete_cursor();
    print!("{}{}{}", cursor::Goto(2, 2), clear::CurrentLine, message,);

    print!(
        "{}{}>{}",
        cursor::Hide,
        cursor::Goto(1, then),
        cursor::Left(1)
    );
}

/// When something goes wrong or does not work, print information about it.
pub fn print_warning<T: std::fmt::Display>(message: T, then: u16) {
    delete_cursor();
    warn!("{}", message);
    print!(
        "{}{}{}{}{}{}{}",
        cursor::Goto(2, 2),
        clear::CurrentLine,
        color::Fg(color::LightWhite),
        color::Bg(color::Red),
        message,
        color::Fg(color::Reset),
        color::Bg(color::Reset),
    );

    print!(
        "{}{}>{}",
        cursor::Hide,
        cursor::Goto(1, then),
        cursor::Left(1)
    );
}

/// Print process of put/delete.
pub fn print_process<T: std::fmt::Display>(message: T) {
    print!("{}{}", message, cursor::Left(10));
}

/// Print the number of process (put/delete).
pub fn display_count(i: usize, all: usize) -> String {
    let mut result = String::new();
    result.push_str(&(i + 1).to_string());
    result.push('/');
    result.push_str(&all.to_string());
    result
}

/// Convert extension setting in the config to HashMap.
pub fn to_extension_map(config: &HashMap<String, Vec<String>>) -> HashMap<String, String> {
    let mut new_map = HashMap::new();
    for (command, extensions) in config.iter() {
        for ext in extensions.iter() {
            new_map.insert(ext.to_lowercase(), command.clone());
        }
    }
    new_map
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
    if byte < 1000 {
        result = byte.to_string();
        result.push('B');
    } else if byte < 1_000_000 {
        result = (byte / 1_000).to_string();
        result.push_str("KB");
    } else if byte < 1_000_000_000 {
        result = (byte / 1_000_000).to_string();
        result.push_str("MB");
    } else {
        result = (byte / 1_000_000_000).to_string();
        result.push_str("GB");
    }
    result
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
            Some(true) => {
                time_start = column - TIME_WIDTH;
                name_max = (time_start - SPACES).into();
            }
            Some(false) | None => match name_length {
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

/// Format a line of the text preview to fit the width.
pub fn format_preview_line(line: &str, preview_column: usize) -> String {
    line.chars().take(preview_column).collect()
}

/// Generate the contents of item to show as a preview.
pub fn list_up_contents(path: PathBuf) -> Result<Vec<String>, FxError> {
    let mut file_v = Vec::new();
    let mut dir_v = Vec::new();
    let mut result = Vec::new();
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
    result.append(&mut dir_v);
    result.append(&mut file_v);
    Ok(result)
}

/// Generate the contents tree.
pub fn make_tree(v: Vec<String>) -> Result<String, FxError> {
    let len = v.len();
    let mut result = String::new();
    for (i, path) in v.iter().enumerate() {
        if i == len - 1 {
            let mut line = "└ ".to_string();
            line.push_str(path);
            result.push_str(&line);
        } else {
            let mut line = "├ ".to_string();
            line.push_str(path);
            line.push('\n');
            result.push_str(&line);
        }
    }
    Ok(result)
}

/// Format texts to print. Used when printing help or text preview.
pub fn format_txt(txt: &str, column: u16, is_help: bool) -> Vec<String> {
    let mut v = Vec::new();
    let mut column_count = 0;
    let mut line = String::new();
    for c in txt.chars() {
        if c == '\n' {
            v.push(line.clone());
            line = String::new();
            column_count = 0;
            continue;
        }
        line.push(c);
        column_count += 1;
        if column_count == column {
            v.push(line.clone());
            line = String::new();
            column_count = 0;
            continue;
        }
    }
    if is_help {
        v.push("Enter 'q' to go back.".to_string());
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

        print!("{}", cursor::Goto(1, (i + 1 - skip_number) as u16));

        if row_count == row - 1 {
            print!("{}...{}", termion::style::Invert, termion::style::Reset);
            break;
        }
        print!("{}", line);
        row_count += 1;
    }
}

pub fn is_editable(s: &str) -> bool {
    s.is_ascii()
}

pub fn init_log(config_dir_path: &Path) -> Result<(), FxError> {
    let mut log_name = chrono::Local::now().format("%F-%H-%M-%S").to_string();
    log_name.push_str(".log");
    let config = ConfigBuilder::new()
        .set_time_offset_to_local()
        .unwrap()
        .build();
    let log_path = config_dir_path.join("log");
    if !log_path.exists() {
        std::fs::create_dir(&log_path)?;
    }
    let log_path = log_path.join(log_name);
    WriteLogger::init(LevelFilter::Info, config, std::fs::File::create(log_path)?).unwrap();
    info!("===START===");

    Ok(())
}

pub fn check_version() -> Result<(), FxError> {
    let output = std::process::Command::new("cargo")
        .args(["search", "felix", "--limit", "1"])
        .output()?
        .stdout;
    if !output.is_empty() {
        if let Ok(ver) = std::str::from_utf8(&output) {
            let latest: String = ver.chars().skip(9).take_while(|x| *x != '\"').collect();
            let current = env!("CARGO_PKG_VERSION");
            if latest != current {
                println!("felix v{}: Latest version is {}.", current, latest);
            } else {
                println!("felix v{}: Up to date.", current);
            }
        } else {
            println!("Cannot read the version.");
        }
    } else {
        println!("Cannot fetch the latest version: Check your internet connection.");
    }
    Ok(())
}

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
    fn test_format_preview_line() {
        assert_eq!(
            format_preview_line("The quick brown fox jumps over the lazy dog", 20),
            "The quick brown fox ".to_string()
        );
    }

    #[test]
    fn test_make_tree() {
        let v = vec![
            "data".to_string(),
            "01.txt".to_string(),
            "2.txt".to_string(),
            "a.txt".to_string(),
            "b.txt".to_string(),
        ];
        assert_eq!(
            make_tree(v.clone()).unwrap(),
            ("├ data\n├ 01.txt\n├ 2.txt\n├ a.txt\n└ b.txt").to_string()
        );
        println!("{}", make_tree(v).unwrap());
    }

    #[test]
    fn test_is_editable() {
        let s1 = "Hello, world!";
        let s2 = "image.jpg";
        let s3 = "a̐éö̲\r\n";
        let s4 = "日本の首都は東京です";
        assert!(is_editable(s1));
        assert!(is_editable(s2));
        assert!(!is_editable(s3));
        assert!(!is_editable(s4));
    }
}

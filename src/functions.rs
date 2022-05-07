use super::state::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use termion::{clear, color, cursor, style};

pub const TIME_WIDTH: u16 = 16;
pub const DEFAULT_NAME_LENGTH: u16 = 30;
pub const SPACES: u16 = 3;

pub fn format_time(time: &Option<String>) -> String {
    match time {
        Some(datetime) => format!("{} {}", &datetime[0..10], &datetime[11..16]),
        None => "".to_string(),
    }
}

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

    let git = dir.join(".git");
    if git.exists() {
        let head = git.join("HEAD");
        if let Ok(head) = std::fs::read(head) {
            let branch: Vec<u8> = head.into_iter().skip(16).collect();
            let branch = std::str::from_utf8(&branch).unwrap();
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
    //Show arrow
    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
}

pub fn rename_file(item: &ItemInfo, name_set: &HashSet<String>) -> String {
    let file_name = &item.file_name;
    if name_set.contains(file_name) {
        let rename = PathBuf::from(&(item).file_name);
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

        let mut renamed_item = item.clone();
        renamed_item.file_name = rename;
        rename_file(&renamed_item, name_set)
    } else {
        file_name.clone()
    }
}

pub fn rename_dir(item: &ItemInfo, name_set: &HashSet<String>) -> String {
    let dir_name = &item.file_name;
    if name_set.contains(dir_name) {
        let mut rename = dir_name.clone();
        rename.push_str("_copied");
        let mut renamed_item = item.clone();
        renamed_item.file_name = rename;
        rename_dir(&renamed_item, name_set)
    } else {
        dir_name.clone()
    }
}

pub fn print_warning<T: std::fmt::Display>(message: T, then: u16) {
    print!(
        " {}{}{}{}{}{}{}",
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

pub fn print_info<T: std::fmt::Display>(message: T, then: u16) {
    print!(" {}{}{}", cursor::Goto(2, 2), clear::CurrentLine, message,);

    print!(
        "{}{}>{}",
        cursor::Hide,
        cursor::Goto(1, then),
        cursor::Left(1)
    );
}

pub fn print_process<T: std::fmt::Display>(message: T) {
    print!("{}{}", message, cursor::Left(10));
}

pub fn display_count(i: usize, all: usize) -> String {
    let mut result = String::new();
    result.push_str(&(i + 1).to_string());
    result.push('/');
    result.push_str(&all.to_string());
    result
}

pub fn to_extension_map(config: &HashMap<String, Vec<String>>) -> HashMap<String, String> {
    let mut new_map = HashMap::new();
    for (command, extensions) in config.iter() {
        for ext in extensions.iter() {
            new_map.insert(ext.to_lowercase(), command.clone());
        }
    }
    new_map
}

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

pub fn duration_to_string(duration: Duration) -> String {
    let s = duration.as_secs_f32();
    let mut result: String = s.to_string().chars().take(4).collect();
    result.push('s');
    result
}

pub fn make_layout(
    column: u16,
    use_full: Option<bool>,
    name_length: Option<usize>,
) -> (u16, usize) {
    let mut time_start: u16;
    let mut name_max: usize;
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

pub fn format_preview_line(line: &str, preview_column: usize) -> String {
    line.replace('\t', "    ")
        .chars()
        .take(preview_column)
        .collect()
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
            format_preview_line("The\tquick brown fox jumps over the lazy dog", 20),
            "The    quick brown f".to_string()
        );
    }
}

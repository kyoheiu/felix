use super::config::CONFIG_EXAMPLE;
use super::errors::MyError;
use super::session::*;
use super::state::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use termion::{clear, color, cursor, style};

pub fn make_config(config_file: &Path, trash_dir: &Path) -> Result<(), MyError> {
    if !trash_dir.exists() {
        fs::create_dir_all(trash_dir)?;
    }

    if !config_file.exists() {
        fs::write(&config_file, CONFIG_EXAMPLE)
            .unwrap_or_else(|_| panic!("cannot write new config file."));
    }

    Ok(())
}

pub fn make_session(session_file: &Path) -> Result<(), MyError> {
    if !session_file.exists() {
        fs::write(&session_file, SESSION_EXAMPLE)
            .unwrap_or_else(|_| panic!("cannot write new session file."));
    }

    Ok(())
}

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
        if let Ok(head) = std::fs::read(".git/HEAD") {
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
    print!("{}{}", message, cursor::Left(7));
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

#[allow(dead_code)]
pub fn get_contents_r(path: PathBuf, vec: &mut Vec<PathBuf>) -> Result<Vec<PathBuf>, MyError> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let dir_path = entry.path();
            vec.push(entry.path());
            let childs = get_contents_r(dir_path, vec)?;
            for child in childs {
                vec.push(child.to_path_buf());
            }
        } else {
            vec.push(entry.path());
        }
    }
    Ok(vec.clone())
}

use super::config::CONFIG_EXAMPLE;
use super::state::*;
use log::debug;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use termion::{clear, color, cursor, style};
use super::session::*;

pub fn make_config(config_file: &Path, trash_dir: &Path) -> std::io::Result<()> {
    if !trash_dir.exists() {
        fs::create_dir_all(trash_dir)?;
    }

    if !config_file.exists() {
        fs::write(&config_file, CONFIG_EXAMPLE)
            .unwrap_or_else(|_| panic!("cannot write new config file."));
    }

    Ok(())
}

pub fn make_session(session_file: &Path) -> std::io::Result<()> {
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
    debug!("clear::All finished.");
    //Show current directory path
    print!(
        " {}{}{}{}{}",
        style::Bold,
        color::Fg(color::Cyan),
        dir.display(),
        style::Reset,
        color::Fg(color::Reset),
    );
    debug!("current_dir displayed.");

    let git = dir.join(".git");
    if git.exists() {
        let head = std::fs::read(".git/HEAD").unwrap();
        let branch: Vec<u8> = head.into_iter().skip(16).collect();
        let branch = std::str::from_utf8(&branch).unwrap();
        debug!("caught current_branch.");
        debug!("current branch to String finished.");
        print!(
            " on {}{}{}{}{}",
            style::Bold,
            color::Fg(color::Magenta),
            branch,
            style::Reset,
            color::Fg(color::Reset)
        );
        debug!("branch name appeared.");
    }
    //Show arrow
    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
    debug!("arrow appeared.");
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
    print!(
        " {}{}{}{}{}",
        cursor::Goto(2, 2),
        color::Fg(color::White),
        clear::CurrentLine,
        message,
        color::Fg(color::Reset)
    );

    print!(
        "{}{}>{}",
        cursor::Hide,
        cursor::Goto(1, then),
        cursor::Left(1)
    );
}

pub fn to_extension_map(config: &HashMap<String, Vec<String>>) -> HashMap<String, String> {
    let mut new_map = HashMap::new();
    for (command, extensions) in config.iter() {
        for ext in extensions.iter() {
            new_map.insert(ext.clone(), command.clone());
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

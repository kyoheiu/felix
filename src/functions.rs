use super::config::CONFIG_EXAMPLE;
use super::state::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use termion::{clear, color, cursor, style};

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

pub fn format_time(time: &Option<String>) -> String {
    match time {
        Some(datetime) => format!(" {} {}", &datetime[0..10], &datetime[11..16]),
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
        let current_branch = std::process::Command::new("git")
            .args(["branch", "--show-current"])
            .output();
        if let Ok(result) = current_branch {
            let branch = String::from_utf8(result.stdout).unwrap_or_default();
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

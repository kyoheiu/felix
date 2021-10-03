use super::entry::*;
use std::fs;
use std::path::PathBuf;
use termion::{clear, cursor};

pub fn make_config(config_file: &PathBuf, trash_dir: &PathBuf) -> std::io::Result<()> {
    if !config_file.exists() {
        fs::File::create(config_file)?;
    }

    if !trash_dir.exists() {
        fs::create_dir_all(trash_dir)?;
    }

    Ok(())
}

pub fn format_time(time: &Option<String>) -> String {
    match time {
        Some(datetime) => format!("{} {}", &datetime[0..10], &datetime[11..16]),
        None => "".to_string(),
    }
}

pub fn clear_all() {
    print!("{}{}", clear::All, cursor::Goto(1, 1));
}

pub fn rename_file(mut name: String, entry_v: &Vec<EntryInfo>) -> String {
    name.push_str("_copied");
    if entry_v
        .iter()
        .any(|entry| entry.file_path.to_str().unwrap() == &name)
    {
        return rename_file(name, entry_v);
    } else {
        return name;
    }
}

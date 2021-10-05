use super::entry::*;
use std::fs;
use std::path::PathBuf;
use termion::{clear, color, cursor, style};

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

pub fn clear_and_show(dir: &PathBuf) {
    print!("{}{}", clear::All, cursor::Goto(1, 1));
    //Show current directory path
    println!(
        " {}{}{}{}{}{}{}",
        style::Bold,
        color::Bg(color::Cyan),
        color::Fg(color::Black),
        dir.display(),
        style::Reset,
        color::Bg(color::Reset),
        color::Fg(color::Reset)
    );
}

pub fn rename_file(
    path_buffer: Option<(PathBuf, String)>,
    entry_v: &Vec<ItemInfo>,
) -> Option<PathBuf> {
    if let Some((mut path, mut name)) = path_buffer {
        if entry_v.iter().any(|entry| entry.file_name == name) {
            name.push_str("_copied");
            path.set_file_name(name);
            return Some(path);
        } else {
            return None;
        }
    } else {
        None
    }
}

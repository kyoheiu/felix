use super::config::CONFIG_EXAMPLE;
use super::items::*;
use std::fs;
use std::path::PathBuf;
use termion::{clear, color, cursor, style};

pub fn make_config(config_file: &PathBuf, trash_dir: &PathBuf) -> std::io::Result<()> {
    if !config_file.exists() {
        fs::write(&config_file, CONFIG_EXAMPLE)
            .unwrap_or_else(|_| panic!("cannot write new confi file."));
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
    //Show arrow
    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
}

pub fn rename_file(item: &ItemInfo, items: &Items) -> String {
    if items.list.iter().any(|x| x.file_name == item.file_name) {
        let rename = PathBuf::from(item.file_name.clone());
        let extension = rename.extension();

        let mut rename = rename.file_stem().unwrap().to_os_string();
        rename.push("_copied.");
        if let Some(ext) = extension {
            rename.push(ext);
        }

        let rename = rename
            .into_string()
            .unwrap_or_else(|_| panic!("cannot paste item."));

        let mut renamed_item = item.clone();
        renamed_item.file_name = rename;
        return rename_file(&renamed_item, items);
    } else {
        item.file_name.clone()
    }
}

pub fn rename_dir(item: &ItemInfo, items: &Items) -> String {
    if items.list.iter().any(|x| x.file_name == item.file_name) {
        let mut rename = item.file_name.clone();
        rename.push_str("_copied");
        let mut renamed_item = item.clone();
        renamed_item.file_name = rename;
        return rename_dir(&renamed_item, items);
    } else {
        item.file_name.clone()
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

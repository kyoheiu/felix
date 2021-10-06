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

pub fn rename_file(file_name: &String, items: &Items) -> String {
    if items
        .list
        .iter()
        .any(|x| x.file_name == file_name.to_string())
    {
        let rename = PathBuf::from(file_name);
        let extension = rename.extension();
        let mut rename = rename.file_stem().unwrap().to_os_string();
        rename.push("_copied.");
        if let Some(ext) = extension {
            rename.push(ext);
        }
        let rename = rename
            .into_string()
            .unwrap_or_else(|_| panic!("cannot paste item."));
        return rename_file(&rename, items);
    } else {
        file_name.to_string()
    }
}

// pub fn rename_dir(file_name: &String, items: &Items) -> String {
//     if items
//         .list
//         .iter()
//         .any(|x| x.file_name == file_name.to_string())
//     {
//         let mut rename = file_name.clone();
//         rename.push_str("_copied");
//         return rename_file(&rename, items);
//     } else {
//         file_name.to_string()
//     }
// }

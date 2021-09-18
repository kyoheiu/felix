use std::env::current_dir;
use std::fs;
use std::io::{stdin, stdout, Error, Write};
use std::process;
use termion::cursor::DetectCursorPos;
use termion::{clear, color, cursor, event::Key, input::TermRead, raw::IntoRawMode};

#[derive(Clone)]
struct EntryInfo {
    line_number: usize,
    file_path: std::path::PathBuf,
    file_name: String,
}

fn make_parent_dir(p: std::path::PathBuf) -> EntryInfo {
    return EntryInfo {
        line_number: 0,
        file_path: p.to_path_buf(),
        file_name: "../".to_string(),
    };
}

fn make_entry(i: usize, dir: std::fs::DirEntry) -> EntryInfo {
    return EntryInfo {
        line_number: i,
        file_path: dir.path(),
        file_name: dir.file_name().into_string().unwrap(),
    };
}

fn push_entries(p: &std::path::PathBuf) -> Result<Vec<EntryInfo>, Error> {
    let mut v = vec![];
    let mut i = 1;

    match p.parent() {
        Some(parent_p) => {
            let parent_dir = make_parent_dir(parent_p.to_path_buf());
            v.push(parent_dir);
        }
        None => {}
    }
    for entry in fs::read_dir(p)? {
        let entry = entry?;
        v.push(make_entry(i, entry));
        i = i + 1;
    }
    Ok(v)
}

fn main() {
    println!("{}", clear::All);
    println!("{}", cursor::Goto(1, 1));

    let path_buf = current_dir().unwrap();

    println!(
        "{red}{}{reset}",
        path_buf.display(),
        red = color::Bg(color::Magenta),
        reset = color::Bg(color::Reset)
    );

    println!("{}", cursor::Goto(1, 3));

    let entry_v = push_entries(&path_buf).unwrap();

    entry_v
        .iter()
        .for_each(|entry| println!("{}", entry.file_name));

    println!("{}", cursor::Goto(1, 4));

    let mut stdout = stdout().into_raw_mode().unwrap();
    let stdin = stdin();

    loop {
        let (x, y) = stdout.cursor_pos().unwrap();

        let ch = stdin.keys();
        match ch {
            Key::Char('j') => print!("{}", cursor::Goto(x, y + 1)),
            Key::Char('k') => print!("{}", cursor::Goto(x, y - 1)),
            _ => {
                println!("{}{}", cursor::Goto(1, 1), clear::All);
                break;
            }
        }
    }
}

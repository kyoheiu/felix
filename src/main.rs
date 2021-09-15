use pancurses::{endwin, initscr, noecho, Attributes, Input, Window};
use std::fs;
use std::io;

struct EntryInfo {
    line_number: u32,
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

fn make_entry(i: u32, dir: std::fs::DirEntry) -> EntryInfo {
    return EntryInfo {
        line_number: i,
        file_path: dir.path(),
        file_name: dir.file_name().into_string().unwrap(),
    };
}

fn push_entries(
    p: &std::path::PathBuf,
    mut v: Vec<EntryInfo>,
) -> Result<Vec<EntryInfo>, io::Error> {
    let mut i = 1;
    for entry in fs::read_dir(p)? {
        let entry = entry?;
        v.push(make_entry(i, entry));
        i = i + 1;
    }
    Ok(v)
}

fn cursor_down(w: &Window) {
    let (y, x) = w.get_cur_yx();
    w.mv(y + 1, x);
}

fn cursor_up(w: &Window) {
    let (y, x) = w.get_cur_yx();
    w.mv(y - 1, x);
}

fn main() {
    let w = initscr();
    w.keypad(true);
    noecho();

    w.refresh();

    let current_directory = std::env::current_dir();

    let mut entry_v = vec![];

    if let Ok(p) = current_directory {
        match p.parent() {
            Some(parent_p) => {
                let parent_dir = make_parent_dir(parent_p.to_path_buf());
                entry_v.push(parent_dir);
            }
            None => {}
        }

        if let Ok(v) = push_entries(&p, entry_v) {
            for entry in v {
                w.addstr(entry.file_name);
                w.addstr("\n");
            }
        }
        w.mv(1, 0);
        w.refresh();

        loop {
            let ch = w.getch().unwrap_or_else(|| panic!("Invalid input."));

            match ch {
                Input::Character('j') => {
                    cursor_down(&w);
                    w.refresh();
                }
                Input::Character('k') => {
                    cursor_up(&w);
                    w.refresh();
                }
                _ => break,
            }
        }

        endwin();
    }
}

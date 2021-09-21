use std::convert::TryInto;
use std::env::current_dir;
use std::fs;
use std::io::{stdin, stdout, Error, Write};
use std::process::Command;
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::screen;
use termion::{clear, color, cursor, input, raw::IntoRawMode};

enum FileType {
    File,
    Directory,
}

struct EntryInfo {
    line_number: usize,
    file_path: std::path::PathBuf,
    file_name: String,
    file_type: FileType,
}

impl EntryInfo {
    fn open_file(&self) {
        let mut exec = Command::new("nvim");
        let path = &self.file_name;
        exec.arg(path).status().expect("failed");
    }

    fn open_directory(&self) {
        let path = &self.file_path;
        Command::new("cd").arg(path).status().expect("failed");
    }
}

fn make_parent_dir(p: std::path::PathBuf) -> EntryInfo {
    return EntryInfo {
        line_number: 0,
        file_path: p.to_path_buf(),
        file_name: "../".to_string(),
        file_type: FileType::Directory,
    };
}

fn make_entry(i: usize, dir: std::fs::DirEntry) -> EntryInfo {
    return EntryInfo {
        line_number: i,
        file_path: dir.path(),
        file_name: dir.file_name().into_string().unwrap(),
        file_type: if dir.path().is_file() {
            FileType::File
        } else {
            FileType::Directory
        },
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

fn list_up(p: &std::path::PathBuf, v: &std::vec::Vec<EntryInfo>) {
    println!(
        " {red}{}{reset}",
        p.display(),
        red = color::Bg(color::Magenta),
        reset = color::Bg(color::Reset)
    );

    for (i, entry) in v.iter().enumerate() {
        print!("{}", cursor::Goto(3, (i + 3).try_into().unwrap()));
        println!("{}", entry.file_name);
    }
}

pub fn start() {
    let mut screen = screen::AlternateScreen::from(std::io::stdout().into_raw_mode().unwrap());
    let mut stdin = stdin().keys();

    print!("{}", clear::All);
    print!("{}", cursor::Goto(1, 1));

    let mut path_buf = current_dir().unwrap();

    let mut entry_v = push_entries(&path_buf).unwrap();

    list_up(&path_buf, &entry_v);

    print!("{}{}>{}", cursor::Hide, cursor::Goto(1, 4), cursor::Left(1));
    screen.flush().unwrap();

    loop {
        let (_, y) = screen.cursor_pos().unwrap();
        let input = stdin.next();
        let mut len = &entry_v.len();

        if let Some(Ok(key)) = input {
            match key {
                Key::Char('j') | Key::Char('\n') => {
                    if y > *len as u16 + 1 {
                        continue;
                    };
                    print!(" {}\n>{}", cursor::Left(1), cursor::Left(1));
                }

                Key::Char('k') => {
                    if y == 3 {
                        continue;
                    };
                    print!(" {}{}>{}", cursor::Up(1), cursor::Left(1), cursor::Left(1));
                }

                Key::Char('g') => {
                    print!(" {}>{}", cursor::Goto(1, 3), cursor::Left(1));
                }

                Key::Char('G') => {
                    print!(" {}>{}", cursor::Goto(1, *len as u16 + 2), cursor::Left(1));
                }

                Key::Char('l') => {
                    let target = &entry_v.get((y - 3) as usize);

                    if let Some(entry) = target {
                        match entry.file_type {
                            FileType::File => {
                                print!("{}", screen::ToAlternateScreen);
                                entry.open_file();
                                print!("{}", screen::ToAlternateScreen);
                                print!("{}{}", clear::All, cursor::Goto(1, 1));
                                list_up(&path_buf, &entry_v);
                                print!(
                                    "{}{}>{}",
                                    cursor::Hide,
                                    cursor::Goto(1, y),
                                    cursor::Left(1)
                                );
                            }
                            FileType::Directory => {
                                path_buf = entry.file_path.to_path_buf();
                                entry_v = push_entries(&path_buf).unwrap();
                                print!("{}{}", clear::All, cursor::Goto(1, 1));
                                list_up(&path_buf, &entry_v);
                                print!(
                                    "{}{}>{}",
                                    cursor::Hide,
                                    cursor::Goto(1, 4),
                                    cursor::Left(1)
                                );
                            }
                        }
                    }
                }

                _ => {
                    print!("{}", cursor::Show);
                    break;
                }
            }
        }
        screen.flush().unwrap();
    }
}

use std::env::current_dir;
use std::fs;
use std::io::{stdin, stdout, Error, Write};
use std::process::Command;
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::screen;
use termion::scroll;
use termion::{clear, color, cursor, raw::IntoRawMode};

const STARTING_POINT: u16 = 4;

#[derive(PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
enum FileType {
    Directory,
    File,
}

struct EntryInfo {
    file_path: std::path::PathBuf,
    file_name: String,
    file_type: FileType,
}

impl EntryInfo {
    fn open_file(&self) {
        let mut exec = Command::new("nvim");
        let path = &self.file_path;
        exec.arg(path).status().expect("failed");
    }
}

fn make_parent_dir(p: std::path::PathBuf) -> EntryInfo {
    return EntryInfo {
        file_path: p.to_path_buf(),
        file_name: String::from("../"),
        file_type: FileType::Directory,
    };
}

fn make_entry(dir: std::fs::DirEntry) -> EntryInfo {
    return EntryInfo {
        file_path: dir.path(),
        file_name: dir
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        file_type: if dir.path().is_file() {
            FileType::File
        } else {
            FileType::Directory
        },
    };
}

fn push_entries(p: &std::path::PathBuf) -> Result<Vec<EntryInfo>, Error> {
    let mut dir_v = vec![];
    let mut file_v = vec![];

    match p.parent() {
        Some(parent_p) => {
            let parent_dir = make_parent_dir(parent_p.to_path_buf());
            dir_v.push(parent_dir);
        }
        None => {}
    }
    for entry in fs::read_dir(p)? {
        let e = entry?;
        let entry = make_entry(e);
        if entry.file_type == FileType::File {
            file_v.push(entry);
        } else {
            dir_v.push(entry);
        }
    }
    dir_v.sort_by_key(|entry| entry.file_name.clone());
    file_v.sort_by_key(|entry| entry.file_name.clone());
    dir_v.append(&mut file_v);
    Ok(dir_v)
}

fn list_up(p: &std::path::PathBuf, v: &std::vec::Vec<EntryInfo>) {
    println!(
        " {red}{}{reset}",
        p.display(),
        red = color::Bg(color::Magenta),
        reset = color::Bg(color::Reset)
    );

    println!("{}__________", cursor::Goto(2, 2));

    for (i, entry) in v.iter().enumerate() {
        let i = i as u16;
        print!("{}", cursor::Goto(3, i + STARTING_POINT));

        if entry.file_type == FileType::File {
            println!(
                "{}{}{}",
                color::Fg(color::LightWhite),
                entry.file_name,
                color::Fg(color::Reset)
            );
        } else {
            println!(
                "{}{}{}",
                color::Fg(color::Green),
                entry.file_name,
                color::Fg(color::Reset)
            );
        }
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

    print!(
        "{}{}>{}",
        cursor::Hide,
        cursor::Goto(1, STARTING_POINT + 1),
        cursor::Left(1)
    );

    let mut i = 1;

    screen.flush().unwrap();

    loop {
        let (_, y) = screen.cursor_pos().unwrap();
        let input = stdin.next();
        let len = &entry_v.len();

        if let Some(Ok(key)) = input {
            match key {
                Key::Char('j') | Key::Char('\n') => {
                    if i == len - 1 {
                        continue;
                    };
                    print!(" {}\n>{}", cursor::Left(1), cursor::Left(1));
                    i += 1;
                }

                Key::Char('k') => {
                    if y == STARTING_POINT {
                        continue;
                    };
                    print!(" {}{}>{}", cursor::Up(1), cursor::Left(1), cursor::Left(1));
                    i -= 1;
                }

                Key::Char('g') => {
                    print!(" {}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                }

                Key::Char('G') => {
                    print!(
                        " {}>{}",
                        cursor::Goto(1, *len as u16 + STARTING_POINT - 1),
                        cursor::Left(1)
                    );
                }

                Key::Char('l') => {
                    let target = &entry_v.get((y - STARTING_POINT) as usize);

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
                                    cursor::Goto(1, STARTING_POINT + 1),
                                    cursor::Left(1)
                                );
                                i = 1;
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

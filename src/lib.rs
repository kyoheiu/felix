use std::env::current_dir;
use std::fs;
use std::io::{stdin, stdout, Error, Write};
use std::process::Command;
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::screen;
use termion::{clear, color, cursor, raw::IntoRawMode};

const STARTING_POINT: u16 = 3;
const SEARCH_EMOJI: char = '\u{1F50D}';

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

fn list_up(p: &std::path::PathBuf, v: &std::vec::Vec<EntryInfo>, skip_number: u16) {
    println!(
        " {red}{}{reset}",
        p.display(),
        red = color::Bg(color::Magenta),
        reset = color::Bg(color::Reset)
    );

    print!("{}{}", cursor::Goto(2, 2), SEARCH_EMOJI);

    let (_, row) = termion::terminal_size().unwrap();

    let mut row_count = 0;

    if row > STARTING_POINT - 1 && v.len() > (row - STARTING_POINT) as usize - 1 {
        for (i, entry) in v.iter().enumerate() {
            let i = i as u16;

            if i < skip_number {
                continue;
            }

            print!("{}", cursor::Goto(3, i + STARTING_POINT - skip_number));

            if row_count == row - STARTING_POINT {
                print!(
                    "{}{}{}lines {}-{}{}{}",
                    cursor::Left(2),
                    color::Bg(color::LightWhite),
                    color::Fg(color::Black),
                    skip_number,
                    row - STARTING_POINT + skip_number,
                    color::Bg(color::Reset),
                    color::Fg(color::Reset)
                );
                break;
            }

            if entry.file_type == FileType::File {
                print!(
                    "{}{}{}",
                    color::Fg(color::LightWhite),
                    entry.file_name,
                    color::Fg(color::Reset)
                );
            } else {
                print!(
                    "{}{}{}",
                    color::Fg(color::Green),
                    entry.file_name,
                    color::Fg(color::Reset)
                );
            }

            row_count += 1;
        }
    } else {
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
}

pub fn start() {
    let mut screen = screen::AlternateScreen::from(std::io::stdout().into_raw_mode().unwrap());
    let mut stdin = stdin().keys();

    print!("{}", clear::All);
    print!("{}", cursor::Goto(1, 1));

    let mut path_buf = current_dir().unwrap();

    let mut entry_v = push_entries(&path_buf).unwrap();
    list_up(&path_buf, &entry_v, 0);

    print!(
        "{}{}>{}",
        cursor::Hide,
        cursor::Goto(1, STARTING_POINT + 1),
        cursor::Left(1)
    );

    let mut i = 1;
    let mut skip_number = 0;
    let (_, row) = termion::terminal_size().unwrap();

    screen.flush().unwrap();

    loop {
        let (_, y) = screen.cursor_pos().unwrap();
        let input = stdin.next();
        let len = &entry_v.len();

        if let Some(Ok(key)) = input {
            match key {
                Key::Char('j') | Key::Down => {
                    if i == len - 1 {
                        continue;
                    };

                    if y == row - 1 && *len > (row - STARTING_POINT) as usize - 1 {
                        skip_number += 1;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&path_buf, &entry_v, skip_number);
                        print!("{}>{}", cursor::Goto(1, row - 1), cursor::Left(1));
                        i += 1;
                        continue;
                    }

                    print!(" {}\n>{}", cursor::Left(1), cursor::Left(1));
                    i += 1;
                }

                Key::Char('k') | Key::Up => {
                    if y == STARTING_POINT {
                        if skip_number == 0 {
                            continue;
                        } else {
                            skip_number -= 1;
                            print!("{}{}", clear::All, cursor::Goto(1, 1));
                            list_up(&path_buf, &entry_v, skip_number);
                            print!("{}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                            i -= 1;
                            continue;
                        }
                    };
                    print!(" {}{}>{}", cursor::Up(1), cursor::Left(1), cursor::Left(1));
                    i -= 1;
                }

                Key::Char('g') => {
                    if i == 0 {
                        continue;
                    }
                    if skip_number != 0 {
                        skip_number = 0;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&path_buf, &entry_v, skip_number);
                    }
                    print!("{}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                    i = 0;
                }

                Key::Char('G') => {
                    if *len > (row - STARTING_POINT) as usize {
                        skip_number = (*len as u16) - row + STARTING_POINT;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&path_buf, &entry_v, skip_number);
                        print!("{}>{}", cursor::Goto(1, row - 1), cursor::Left(1));
                        i = len - 1;
                        continue;
                    }
                    print!(
                        " {}>{}",
                        cursor::Goto(1, *len as u16 + STARTING_POINT - 1),
                        cursor::Left(1)
                    );
                    i = len - 1;
                }

                Key::Char('l') | Key::Char('\n') | Key::Right => {
                    let target = &entry_v.get(i);

                    if let Some(entry) = target {
                        match entry.file_type {
                            FileType::File => {
                                print!("{}", screen::ToAlternateScreen);
                                entry.open_file();
                                print!("{}", screen::ToAlternateScreen);
                                print!("{}{}", clear::All, cursor::Goto(1, 1));
                                list_up(&path_buf, &entry_v, 0);
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
                                list_up(&path_buf, &entry_v, 0);
                                print!(
                                    "{}>{}",
                                    cursor::Goto(1, STARTING_POINT + 1),
                                    cursor::Left(1)
                                );
                                skip_number = 0;
                                i = 1;
                            }
                        }
                    }
                }

                Key::Char('h') | Key::Left => match path_buf.parent() {
                    Some(parent_p) => {
                        path_buf = parent_p.to_path_buf();
                        entry_v = push_entries(&path_buf).unwrap();
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&path_buf, &entry_v, 0);
                        print!(
                            "{}>{}",
                            cursor::Goto(1, STARTING_POINT + 1),
                            cursor::Left(1)
                        );
                        skip_number = 0;
                        i = 1;
                    }
                    None => {
                        continue;
                    }
                },

                Key::Char('/') => {
                    print!(" ");
                    print!("{}>{}", cursor::Goto(1, 2), cursor::Right(2));
                    screen.flush().unwrap();
                    let mut prefix = String::from("");
                    loop {
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Char(c) => {
                                    print!("{}", c);
                                    prefix.push(c);
                                    screen.flush().unwrap();
                                }
                                Key::Esc => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), SEARCH_EMOJI);
                                    screen.flush().unwrap();
                                    print!("{}>{}", cursor::Goto(1, 4), cursor::Left(1));
                                    i = 1;
                                    break;
                                }
                                _ => continue,
                            }
                        }
                    }
                }

                Key::Esc => break,

                _ => {
                    continue;
                }
            }
        }
        screen.flush().unwrap();
    }
    print!("{}", cursor::Show);
}

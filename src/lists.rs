use super::config::{read_config, Config};
use super::entry::*;
use dirs;
use std::env::current_dir;
use std::fs;
use std::io::{stdin, Error, Write};
use std::path::PathBuf;
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor, screen, style};

const STARTING_POINT: u16 = 3;
const SEARCH_EMOJI: char = '\u{1F50D}';
const CONFIG_FILE: &str = "fm/config.toml";
const TRUSH: &str = "fm/trash";

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
        //todo: I have no idea what I'm doing
        file_name: dir
            .path()
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap(),
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

fn make_config() -> std::io::Result<()> {
    let config_dir = dirs::config_dir().unwrap();
    let config_file = config_dir.join(PathBuf::from(CONFIG_FILE));
    let trush_dir = config_dir.join(PathBuf::from(TRUSH));

    if !trush_dir.exists() {
        fs::create_dir_all(trush_dir)?;
    }

    if !config_file.exists() {
        fs::File::create(config_file)?;
    }

    Ok(())
}

fn list_up(
    config: &Config,
    p: &std::path::PathBuf,
    v: &std::vec::Vec<EntryInfo>,
    skip_number: u16,
) {
    //Show current directory path
    println!(
        " {}{}{}{}{}{}{}",
        style::Bold,
        color::Bg(color::Cyan),
        color::Fg(color::Black),
        p.display(),
        style::Reset,
        color::Bg(color::Reset),
        color::Fg(color::Reset)
    );

    //Show filter emoji and space
    print!("{}{}", cursor::Goto(2, 2), SEARCH_EMOJI);

    let (_, row) = termion::terminal_size().unwrap();
    let len = v.len();

    //if lists exceeds max-row
    if row > STARTING_POINT - 1 && v.len() > (row - STARTING_POINT) as usize - 1 {
        let mut row_count = 0;
        for (i, entry) in v.iter().enumerate() {
            let i = i as u16;

            if i < skip_number {
                continue;
            }

            print!("{}", cursor::Goto(3, i + STARTING_POINT - skip_number));

            if row_count == row - STARTING_POINT {
                print!(
                    "  {}{}{}lines {}-{}({}){}{}",
                    cursor::Left(2),
                    color::Bg(color::LightWhite),
                    color::Fg(color::Black),
                    skip_number,
                    row - STARTING_POINT + skip_number,
                    len,
                    color::Bg(color::Reset),
                    color::Fg(color::Reset)
                );
                break;
            } else {
                entry.print(config);
                row_count += 1;
            }
        }
    } else {
        for (i, entry) in v.iter().enumerate() {
            let i = i as u16;
            print!("{}", cursor::Goto(3, i + STARTING_POINT));
            entry.print(config);
        }
    }
}

pub fn start() {
    let _ = make_config();
    let config = read_config().unwrap();

    let (_, row) = termion::terminal_size().unwrap();

    let mut screen = screen::AlternateScreen::from(std::io::stdout().into_raw_mode().unwrap());

    print!("{}", clear::All);
    print!("{}", cursor::Goto(1, 1));

    let mut path_buf = current_dir().unwrap();
    let mut entry_v = push_entries(&path_buf).unwrap();
    list_up(&config, &path_buf, &entry_v, 0);

    print!(
        "{}{}>{}",
        cursor::Hide,
        cursor::Goto(1, STARTING_POINT + 1),
        cursor::Left(1)
    );
    screen.flush().unwrap();

    let mut index = 1;
    let mut skip_number = 0;
    let mut stdin = stdin().keys();

    loop {
        let (_, y) = screen.cursor_pos().unwrap();
        let input = stdin.next();
        let len = &entry_v.len();

        if let Some(Ok(key)) = input {
            match key {
                //Go up. If lists exceeds max-row, lists "scrolls" before the top of the list.
                Key::Char('j') | Key::Down => {
                    if index == len - 1 {
                        continue;
                    } else if y == row - 4 && *len > (row - STARTING_POINT) as usize - 1 {
                        skip_number += 1;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &path_buf, &entry_v, skip_number);
                        print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                        index += 1;
                        continue;
                    } else {
                        print!(" {}\n>{}", cursor::Left(1), cursor::Left(1));
                        index += 1;
                    }
                }

                //Go down. If lists exceeds max-row, lists "scrolls" before the bottom of the list.
                Key::Char('k') | Key::Up => {
                    if y == STARTING_POINT {
                    } else if y == STARTING_POINT + 3 && skip_number != 0 {
                        skip_number -= 1;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &path_buf, &entry_v, skip_number);
                        print!(
                            "{}>{}",
                            cursor::Goto(1, STARTING_POINT + 3),
                            cursor::Left(1)
                        );
                        index -= 1;
                    } else {
                        print!(" {}{}>{}", cursor::Up(1), cursor::Left(1), cursor::Left(1));
                        index -= 1;
                    }
                }

                //Go to first line of lists
                Key::Char('g') => {
                    if index == 0 {
                        continue;
                    } else if skip_number != 0 {
                        skip_number = 0;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &path_buf, &entry_v, skip_number);
                    } else {
                        print!(" {}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                        index = 0;
                    }
                }

                //Go to end line of lists
                Key::Char('G') => {
                    if *len > (row - STARTING_POINT) as usize {
                        skip_number = (*len as u16) - row + STARTING_POINT;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &path_buf, &entry_v, skip_number);
                        print!("{}>{}", cursor::Goto(1, row - 1), cursor::Left(1));
                        index = len - 1;
                    } else {
                        print!(
                            " {}>{}",
                            cursor::Goto(1, *len as u16 + STARTING_POINT - 1),
                            cursor::Left(1)
                        );
                        index = len - 1;
                    }
                }

                //Choose file(exec in any way fo now) or directory(change lists as if `cd`)
                Key::Char('l') | Key::Char('\n') | Key::Right => {
                    let target = &entry_v.get(index);

                    if let Some(entry) = target {
                        match entry.file_type {
                            FileType::File => {
                                print!("{}", screen::ToAlternateScreen);
                                entry.open_file(&config);
                                print!("{}", screen::ToAlternateScreen);
                                print!("{}{}", clear::All, cursor::Goto(1, 1));
                                list_up(&config, &path_buf, &entry_v, skip_number);
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
                                list_up(&config, &path_buf, &entry_v, 0);
                                print!(
                                    "{}>{}",
                                    cursor::Goto(1, STARTING_POINT + 1),
                                    cursor::Left(1)
                                );
                                skip_number = 0;
                                index = 1;
                            }
                        }
                    }
                }

                //Go to parent directory if exists
                Key::Char('h') | Key::Left => match path_buf.parent() {
                    Some(parent_p) => {
                        path_buf = parent_p.to_path_buf();
                        entry_v = push_entries(&path_buf).unwrap();
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &path_buf, &entry_v, 0);
                        print!(
                            "{}>{}",
                            cursor::Goto(1, STARTING_POINT + 1),
                            cursor::Left(1)
                        );
                        skip_number = 0;
                        index = 1;
                    }
                    None => {
                        continue;
                    }
                },

                //Enter the filter mode
                Key::Char('/') => {
                    print!(" ");
                    print!("{}>{}", cursor::Goto(1, 2), cursor::Right(4));
                    screen.flush().unwrap();
                    let mut word = String::from("");
                    loop {
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                //Go to filtered lists
                                Key::Char('\n') => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), SEARCH_EMOJI);
                                    screen.flush().unwrap();

                                    print!("{}>{}", cursor::Goto(1, 3), cursor::Left(1));
                                    index = 0;
                                    break;
                                }

                                //Quit filter mode and return to original lists
                                Key::Esc => {
                                    print!("{}", clear::All);
                                    print!("{}", cursor::Goto(1, 1));

                                    entry_v = push_entries(&path_buf).unwrap();
                                    list_up(&config, &path_buf, &entry_v, 0);

                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, STARTING_POINT + 1),
                                        cursor::Left(1)
                                    );

                                    index = 1;
                                    skip_number = 0;

                                    break;
                                }

                                //Enter word for case-sensitive filter
                                Key::Char(c) => {
                                    word.push(c);

                                    entry_v = entry_v
                                        .into_iter()
                                        .filter(|entry| entry.file_name.contains(&word))
                                        .collect();

                                    skip_number = 0;
                                    print!("{}{}", clear::All, cursor::Goto(1, 1));
                                    list_up(&config, &path_buf, &entry_v, skip_number);

                                    print!("{}>{}{}", cursor::Goto(1, 2), word, cursor::Right(2));

                                    screen.flush().unwrap();
                                }

                                Key::Backspace => {
                                    word.pop();

                                    entry_v = push_entries(&path_buf).unwrap();
                                    entry_v = entry_v
                                        .into_iter()
                                        .filter(|entry| entry.file_name.contains(&word))
                                        .collect();

                                    skip_number = 0;
                                    print!("{}{}", clear::All, cursor::Goto(1, 1));
                                    list_up(&config, &path_buf, &entry_v, skip_number);

                                    print!("{}>{}{}", cursor::Goto(1, 2), word, cursor::Right(2));

                                    screen.flush().unwrap();
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

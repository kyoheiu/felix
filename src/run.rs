use super::config::read_config;
use super::entry::*;
use std::env::current_dir;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor, screen};

pub fn run() {
    let config_dir = dirs::config_dir().unwrap();
    let config_file = config_dir.join(PathBuf::from(CONFIG_FILE));
    let trash_dir = config_dir.join(PathBuf::from(TRASH));
    let mut path_buffer: Option<PathBuf> = None;

    let _ = make_config(&config_file, &trash_dir);
    let config = read_config().unwrap();

    let (column, row) = termion::terminal_size().unwrap();
    if column < NAME_MAX_LEN as u16 + TIME_START_POS - 3 {
        panic!("Too small terminal size.")
    };

    let mut screen = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());

    print!("{}", clear::All);
    print!("{}", cursor::Goto(1, 1));

    let mut current_dir = current_dir().unwrap();
    let mut entry_v = push_entries(&current_dir).unwrap();
    list_up(&config, &current_dir, &entry_v, 0);

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
        let len = &entry_v.len();
        let (_, y) = screen.cursor_pos().unwrap();
        let input = stdin.next();

        if let Some(Ok(key)) = input {
            match key {
                //Go up. If lists exceeds max-row, lists "scrolls" before the top of the list
                Key::Char('j') | Key::Down => {
                    if index == len - 1 {
                        continue;
                    } else if y == row - 4 && *len > (row - STARTING_POINT) as usize - 1 {
                        skip_number += 1;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &current_dir, &entry_v, skip_number);
                        print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                        index += 1;
                        continue;
                    } else {
                        print!(" {}\n>{}", cursor::Left(1), cursor::Left(1));
                        index += 1;
                    }
                }

                //Go down. If lists exceeds max-row, lists "scrolls" before the bottom of the list
                Key::Char('k') | Key::Up => {
                    if y == STARTING_POINT {
                    } else if y == STARTING_POINT + 3 && skip_number != 0 {
                        skip_number -= 1;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &current_dir, &entry_v, skip_number);
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

                //Go to first line of the list
                Key::Char('g') => {
                    if index == 0 {
                        continue;
                    } else if skip_number != 0 {
                        skip_number = 0;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &current_dir, &entry_v, skip_number);
                    } else {
                        print!(" {}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                        index = 0;
                    }
                }

                //Go to end line of the list
                Key::Char('G') => {
                    if *len > (row - STARTING_POINT) as usize {
                        skip_number = (*len as u16) - row + STARTING_POINT;
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &current_dir, &entry_v, skip_number);
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

                //Open file(exec in any way fo now) or change directory(change lists as if `cd`)
                Key::Char('l') | Key::Char('\n') | Key::Right => {
                    let target = &entry_v.get(index);

                    if let Some(entry) = target {
                        match entry.file_type {
                            FileType::File => {
                                print!("{}", screen::ToAlternateScreen);
                                entry.open_file(&config);
                                print!("{}", screen::ToAlternateScreen);
                                print!("{}{}", clear::All, cursor::Goto(1, 1));
                                list_up(&config, &current_dir, &entry_v, skip_number);
                                print!(
                                    "{}{}>{}",
                                    cursor::Hide,
                                    cursor::Goto(1, y),
                                    cursor::Left(1)
                                );
                            }
                            FileType::Directory => {
                                current_dir = entry.file_path.to_path_buf();
                                entry_v = push_entries(&current_dir).unwrap();
                                print!("{}{}", clear::All, cursor::Goto(1, 1));
                                list_up(&config, &current_dir, &entry_v, 0);
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
                Key::Char('h') | Key::Left => match current_dir.parent() {
                    Some(parent_p) => {
                        current_dir = parent_p.to_path_buf();
                        entry_v = push_entries(&current_dir).unwrap();
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &current_dir, &entry_v, 0);
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

                Key::Char('D') => {
                    let target = &entry_v.get(index);

                    if let Some(entry) = target {
                        let _ = entry.remove(&trash_dir);

                        entry_v = push_entries(&current_dir).unwrap();
                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                        list_up(&config, &current_dir, &entry_v, skip_number);
                        if index == len - 1 {
                            print!("{}>{}", cursor::Goto(1, y - 1), cursor::Left(1));
                            index -= 1;
                        } else {
                            print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                        }
                        screen.flush().unwrap();
                    }
                }

                Key::Char('y') => {
                    let target = entry_v.get(index).unwrap();
                    let path = target.file_path.clone();
                    path_buffer = Some(path);
                }

                Key::Char('E') => {
                    print!(" ");
                    print!(
                        "{}{}{}{}{}{}",
                        cursor::Goto(2, 2),
                        color::Fg(color::LightWhite),
                        color::Bg(color::Red),
                        CONFIRMATION,
                        color::Fg(color::Reset),
                        color::Bg(color::Reset),
                    );
                    screen.flush().unwrap();

                    loop {
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Char('y') | Key::Char('Y') => {
                                    let _ = fs_extra::dir::create(&trash_dir, true);
                                    print!(
                                        "{}{}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        DOWN_ARROW
                                    );
                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, STARTING_POINT + 1),
                                        cursor::Left(1)
                                    );
                                    index = 1;
                                    break;
                                }
                                _ => {
                                    print!(
                                        "{}{}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        DOWN_ARROW
                                    );
                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, STARTING_POINT + 1),
                                        cursor::Left(1)
                                    );
                                    index = 1;
                                    break;
                                }
                            }
                        }
                    }
                }

                //Enter the filter mode
                Key::Char('/') => {
                    print!(" ");
                    print!("{}{}{}", cursor::Goto(2, 2), RIGHT_ARROW, cursor::Right(4));
                    screen.flush().unwrap();
                    let mut word = String::from("");
                    loop {
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                //Go to filtered lists
                                Key::Char('\n') => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    screen.flush().unwrap();

                                    print!("{}>{}", cursor::Goto(1, 3), cursor::Left(1));
                                    index = 0;
                                    break;
                                }

                                //Quit filter mode and return to original lists
                                Key::Esc => {
                                    print!("{}", clear::All);
                                    print!("{}", cursor::Goto(1, 1));

                                    entry_v = push_entries(&current_dir).unwrap();
                                    list_up(&config, &current_dir, &entry_v, 0);

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

                                //Input char(case-sensitive)
                                Key::Char(c) => {
                                    word.push(c);

                                    entry_v = entry_v
                                        .into_iter()
                                        .filter(|entry| entry.file_name.contains(&word))
                                        .collect();

                                    skip_number = 0;
                                    print!("{}{}", clear::All, cursor::Goto(1, 1));
                                    list_up(&config, &current_dir, &entry_v, skip_number);

                                    print!(
                                        "{}{} {}{}",
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        word,
                                        cursor::Right(2)
                                    );

                                    screen.flush().unwrap();
                                }

                                Key::Backspace => {
                                    word.pop();

                                    entry_v = push_entries(&current_dir).unwrap();
                                    entry_v = entry_v
                                        .into_iter()
                                        .filter(|entry| entry.file_name.contains(&word))
                                        .collect();

                                    skip_number = 0;
                                    print!("{}{}", clear::All, cursor::Goto(1, 1));
                                    list_up(&config, &current_dir, &entry_v, skip_number);

                                    print!(
                                        "{}{} {}{}",
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        word,
                                        cursor::Right(2)
                                    );

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
    //When finishes, restore the cursor
    print!("{}", cursor::Show);
}

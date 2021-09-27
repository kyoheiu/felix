use super::config::read_config;
use super::entry::*;
use std::env::current_dir;
use std::io::{stdin, stdout, Write};
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, cursor, screen};

pub fn start() {
    let _ = make_config();
    let config = read_config().unwrap();
    let (_, row) = termion::terminal_size().unwrap();

    let mut screen = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());

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
                //Go up. If lists exceeds max-row, lists "scrolls" before the top of the list
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

                //Go down. If lists exceeds max-row, lists "scrolls" before the bottom of the list
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

                //Go to first line of the list
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

                //Go to end line of the list
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

                                //Input char(case-sensitive)
                                Key::Char(c) => {
                                    word.push(c);

                                    entry_v = entry_v
                                        .into_iter()
                                        .filter(|entry| entry.file_name.contains(&word))
                                        .collect();

                                    skip_number = 0;
                                    print!("{}{}", clear::All, cursor::Goto(1, 1));
                                    list_up(&config, &path_buf, &entry_v, skip_number);

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

                                    entry_v = push_entries(&path_buf).unwrap();
                                    entry_v = entry_v
                                        .into_iter()
                                        .filter(|entry| entry.file_name.contains(&word))
                                        .collect();

                                    skip_number = 0;
                                    print!("{}{}", clear::All, cursor::Goto(1, 1));
                                    list_up(&config, &path_buf, &entry_v, skip_number);

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

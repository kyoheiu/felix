use super::config::read_config;
use super::entry::*;
use super::functions::*;
use super::state::*;
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

    clear_all();

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

    let mut nums = Num::new();
    let mut memo_v: Vec<CursorMemo> = Vec::new();
    let mut stdin = stdin().keys();

    loop {
        let len = &entry_v.len();
        let (_, y) = screen.cursor_pos().unwrap();
        let input = stdin.next();

        if let Some(Ok(key)) = input {
            match key {
                //Go up. If lists exceeds max-row, lists "scrolls" before the top of the list
                Key::Char('j') | Key::Down => {
                    if nums.index == len - 1 {
                        continue;
                    } else if y == row - 4 && *len > (row - STARTING_POINT) as usize - 1 {
                        nums.inc_skip();
                        clear_all();
                        list_up(&config, &current_dir, &entry_v, nums.skip);
                        print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                        nums.go_down();
                    } else {
                        print!(" {}\n>{}", cursor::Left(1), cursor::Left(1));
                        nums.go_down();
                    }
                }

                //Go down. If lists exceeds max-row, lists "scrolls" before the bottom of the list
                Key::Char('k') | Key::Up => {
                    if y == STARTING_POINT {
                    } else if y == STARTING_POINT + 3 && nums.skip != 0 {
                        nums.dec_skip();
                        clear_all();
                        list_up(&config, &current_dir, &entry_v, nums.skip);
                        print!(
                            "{}>{}",
                            cursor::Goto(1, STARTING_POINT + 3),
                            cursor::Left(1)
                        );
                        nums.go_up();
                    } else {
                        print!(" {}{}>{}", cursor::Up(1), cursor::Left(1), cursor::Left(1));
                        nums.go_up();
                    }
                }

                //Go to first line of the list
                Key::Char('g') => {
                    if nums.index == 0 {
                        continue;
                    } else if nums.skip != 0 {
                        nums.reset();
                        clear_all();
                        list_up(&config, &current_dir, &entry_v, nums.skip);
                        print!(" {}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                        nums.go_top();
                    } else {
                        print!(" {}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                        nums.go_top();
                    }
                }

                //Go to end line of the list
                Key::Char('G') => {
                    if *len > (row - STARTING_POINT) as usize {
                        nums.skip = (*len as u16) - row + STARTING_POINT;
                        clear_all();
                        list_up(&config, &current_dir, &entry_v, nums.skip);
                        print!("{}>{}", cursor::Goto(1, row - 1), cursor::Left(1));
                        nums.go_bottom(len - 1);
                    } else {
                        print!(
                            " {}>{}",
                            cursor::Goto(1, *len as u16 + STARTING_POINT - 1),
                            cursor::Left(1)
                        );
                        nums.go_bottom(len - 1);
                    }
                }

                //Open file(exec in any way fo now) or change directory(change lists as if `cd`)
                Key::Char('l') | Key::Char('\n') | Key::Right => {
                    let target = &entry_v.get(nums.index);

                    if let Some(entry) = target {
                        match entry.file_type {
                            FileType::File => {
                                print!("{}", screen::ToAlternateScreen);
                                entry.open_file(&config);
                                print!("{}", screen::ToAlternateScreen);
                                clear_all();
                                list_up(&config, &current_dir, &entry_v, nums.skip);
                                print!(
                                    "{}{}>{}",
                                    cursor::Hide,
                                    cursor::Goto(1, y),
                                    cursor::Left(1)
                                );
                            }
                            FileType::Directory => {
                                let cursor_memo = CursorMemo {
                                    num: nums.clone(),
                                    cursor_pos: y,
                                };
                                memo_v.push(cursor_memo);

                                current_dir = entry.file_path.to_path_buf();
                                entry_v = push_entries(&current_dir).unwrap();
                                clear_all();
                                list_up(&config, &current_dir, &entry_v, 0);
                                print!(
                                    "{}>{}",
                                    cursor::Goto(1, STARTING_POINT + 1),
                                    cursor::Left(1)
                                );
                                nums.reset();
                            }
                        }
                    }
                }

                //Go to parent directory if exists
                Key::Char('h') | Key::Left => match current_dir.parent() {
                    Some(parent_p) => {
                        current_dir = parent_p.to_path_buf();
                        entry_v = push_entries(&current_dir).unwrap();
                        clear_all();
                        list_up(&config, &current_dir, &entry_v, 0);

                        match memo_v.pop() {
                            Some(memo) => {
                                nums = memo.num;
                                print!("{}>{}", cursor::Goto(1, memo.cursor_pos), cursor::Left(1));
                            }
                            None => {
                                nums.reset();
                                print!(
                                    "{}>{}",
                                    cursor::Goto(1, STARTING_POINT + 1),
                                    cursor::Left(1)
                                );
                            }
                        }
                    }
                    None => {
                        continue;
                    }
                },

                Key::Char('D') => {
                    let target = &entry_v.get(nums.index);

                    if let Some(entry) = target {
                        let name = entry.file_path.file_name().unwrap();
                        let path = &trash_dir.join(name);
                        path_buffer = Some(path.clone());

                        let _ = entry.remove(&trash_dir);

                        entry_v = push_entries(&current_dir).unwrap();
                        clear_all();
                        list_up(&config, &current_dir, &entry_v, nums.skip);
                        if nums.index == len - 1 {
                            print!("{}>{}", cursor::Goto(1, y - 1), cursor::Left(1));
                            nums.go_up();
                        } else {
                            print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                        }
                        screen.flush().unwrap();
                    }
                }

                Key::Char('y') => {
                    let target = entry_v.get(nums.index).unwrap();
                    let path = target.file_path.clone();
                    path_buffer = Some(path);
                }

                Key::Char('p') => {}

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
                                    nums.starting_point();
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
                                    nums.starting_point();
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
                                    nums.go_top();
                                    break;
                                }

                                //Quit filter mode and return to original lists
                                Key::Esc => {
                                    clear_all();

                                    entry_v = push_entries(&current_dir).unwrap();
                                    list_up(&config, &current_dir, &entry_v, 0);

                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, STARTING_POINT + 1),
                                        cursor::Left(1)
                                    );

                                    nums.reset();

                                    break;
                                }

                                //Input char(case-sensitive)
                                Key::Char(c) => {
                                    word.push(c);

                                    entry_v = entry_v
                                        .into_iter()
                                        .filter(|entry| entry.file_name.contains(&word))
                                        .collect();

                                    nums.reset_skip();
                                    clear_all();
                                    list_up(&config, &current_dir, &entry_v, nums.skip);

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

                                    nums.reset_skip();
                                    clear_all();
                                    list_up(&config, &current_dir, &entry_v, nums.skip);

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

use super::entry::*;
use super::functions::*;
use super::state::*;
use std::env::current_dir;
use std::io::{stdin, stdout, Write};
use std::path::{Path, PathBuf};
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, color, cursor, screen};

pub fn run() {
    let mut config_dir = dirs::config_dir().unwrap_or_else(|| panic!("cannot read config dir."));
    config_dir.push(CONFIG_DIR);
    let config_file = config_dir.join(PathBuf::from(CONFIG_FILE));
    let trash_dir = config_dir.join(PathBuf::from(TRASH));
    make_config(&config_file, &trash_dir)
        .unwrap_or_else(|_| panic!("cannot make config file or trash dir."));

    let mut items = Items::new();
    let mut current_dir = current_dir().unwrap_or_else(|_| panic!("cannot read current dir."));
    items.update_list(&current_dir);
    items.trash_dir = trash_dir;

    let mut nums = Num::new();

    let (column, row) = termion::terminal_size().unwrap();
    if column < NAME_MAX_LEN as u16 + TIME_START_POS - 3 {
        panic!("too small terminal size.")
    };

    let mut screen = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());

    clear_and_show(&current_dir);
    print!("{}", cursor::Hide);

    items.list_up(nums.skip);

    print!(
        "{}>{}",
        cursor::Goto(1, STARTING_POINT + 1),
        cursor::Left(1)
    );
    screen.flush().unwrap();

    let mut memo_v: Vec<CursorMemo> = Vec::new();
    let mut stdin = stdin().keys();

    loop {
        let len = items.list.len();
        let (_, y) = screen.cursor_pos().unwrap();
        let input = stdin.next();

        if let Some(Ok(key)) = input {
            match key {
                //Go up. If lists exceeds max-row, lists "scrolls" before the top of the list
                Key::Char('j') | Key::Down => {
                    if nums.index == len - 1 {
                        continue;
                    } else if y == row - 4 && len > (row - STARTING_POINT) as usize - 1 {
                        nums.inc_skip();
                        clear_and_show(&current_dir);
                        items.list_up(nums.skip);
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
                        clear_and_show(&current_dir);
                        items.list_up(nums.skip);
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
                        clear_and_show(&current_dir);
                        items.list_up(nums.skip);
                        print!(" {}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                        nums.go_top();
                    } else {
                        print!(" {}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                        nums.go_top();
                    }
                }

                //Go to end line of the list
                Key::Char('G') => {
                    if len > (row - STARTING_POINT) as usize {
                        nums.skip = (len as u16) - row + STARTING_POINT;
                        clear_and_show(&current_dir);
                        items.list_up(nums.skip);
                        print!("{}>{}", cursor::Goto(1, row - 1), cursor::Left(1));
                        nums.go_bottom(len - 1);
                    } else {
                        print!(
                            " {}>{}",
                            cursor::Goto(1, len as u16 + STARTING_POINT - 1),
                            cursor::Left(1)
                        );
                        nums.go_bottom(len - 1);
                    }
                }

                //Open file(exec in any way fo now) or change directory(change lists as if `cd`)
                Key::Char('l') | Key::Char('\n') | Key::Right => {
                    //todo: avoid .clone()
                    let item = items.get_item(nums.index).clone();
                    match item.file_type {
                        FileType::File => {
                            print!("{}", screen::ToAlternateScreen);
                            items.open_file(nums.index);
                            print!("{}", screen::ToAlternateScreen);
                            clear_and_show(&current_dir);
                            items.list_up(nums.skip);
                            print!("{}{}>{}", cursor::Hide, cursor::Goto(1, y), cursor::Left(1));
                        }
                        FileType::Directory => {
                            //store the last cursor position and skip number.
                            let cursor_memo = CursorMemo {
                                num: nums.clone(),
                                cursor_pos: y,
                            };
                            memo_v.push(cursor_memo);

                            current_dir = item.file_path;
                            items.update_list(&current_dir);
                            clear_and_show(&current_dir);
                            items.list_up(0);
                            print!(
                                "{}>{}",
                                cursor::Goto(1, STARTING_POINT + 1),
                                cursor::Left(1)
                            );
                            nums.reset();
                        }
                    }
                }

                //Go to parent directory if exists
                Key::Char('h') | Key::Left => match current_dir.parent() {
                    Some(parent_p) => {
                        current_dir = parent_p.to_path_buf();
                        items.update_list(&current_dir);
                        clear_and_show(&current_dir);
                        items.list_up(0);

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
                    if nums.index == 0 {
                        continue;
                    }
                    match items.get_item(nums.index).file_type {
                        FileType::Directory => items.remove_dir(nums.index),
                        FileType::File => items.remove_file(nums.index),
                    }

                    clear_and_show(&current_dir);
                    items.list_up(nums.skip);
                    if nums.index == len - 1 {
                        print!("{}>{}", cursor::Goto(1, y - 1), cursor::Left(1));
                        nums.go_up();
                    } else {
                        print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                    }
                }

                Key::Char('y') => {
                    if nums.index == 0 {
                        continue;
                    }
                    let item = items.get_item(nums.index);
                    items.item_buf = Some(item.clone());
                }

                //todo: paste item of path_buffer
                Key::Char('p') => {
                    let item = items.item_buf.clone();
                    if item == None {
                        continue;
                    } else {
                        match item.unwrap().file_type {
                            FileType::Directory => items.paste_dir(&current_dir),
                            FileType::File => items.paste_file(&current_dir),
                        }
                        clear_and_show(&current_dir);
                        items.list_up(nums.skip);
                        print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                    }
                }

                Key::Char('c') => {
                    print!("{}{}", cursor::Show, cursor::BlinkingBlock);
                    let item = items.get_item(nums.index);

                    let mut rename = item.file_name.clone().chars().collect::<Vec<char>>();
                    print!(
                        "{}{} {}",
                        cursor::Goto(2, 2),
                        RIGHT_ARROW,
                        &rename.iter().collect::<String>(),
                    );
                    screen.flush().unwrap();

                    loop {
                        let eow = rename.len() + 3;
                        let (x, _) = screen.cursor_pos().unwrap();
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                //rename item
                                Key::Char('\n') => {
                                    let rename = rename.iter().collect::<String>();
                                    let mut to = current_dir.clone();
                                    to.push(rename);
                                    std::fs::rename(Path::new(&item.file_path), Path::new(&to))
                                        .unwrap_or_else(|_| panic!("rename failed"));

                                    clear_and_show(&current_dir);
                                    items.update_list(&current_dir);
                                    items.list_up(nums.skip);

                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, y),
                                        cursor::Left(1)
                                    );
                                    break;
                                }

                                //Quit rename mode and return to original lists
                                Key::Esc => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    screen.flush().unwrap();

                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, y),
                                        cursor::Left(1)
                                    );
                                    break;
                                }

                                Key::Left => {
                                    if x == 4 {
                                        continue;
                                    };
                                    print!("{}", cursor::Left(1));
                                    screen.flush().unwrap();
                                }

                                Key::Right => {
                                    if x as usize == eow + 1 {
                                        continue;
                                    };
                                    print!("{}", cursor::Right(1));
                                    screen.flush().unwrap();
                                }

                                //Input char(case-sensitive)
                                Key::Char(c) => {
                                    let memo_x = x;
                                    rename.insert((x - 4).into(), c);

                                    print!(
                                        "{}{}{} {}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        &rename.iter().collect::<String>(),
                                        cursor::Goto(memo_x + 1, 2)
                                    );

                                    screen.flush().unwrap();
                                }

                                Key::Backspace => {
                                    let memo_x = x;
                                    if x == 4 {
                                        continue;
                                    };
                                    rename.remove((x - 5).into());

                                    print!(
                                        "{}{}{} {}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        &rename.iter().collect::<String>(),
                                        cursor::Goto(memo_x - 1, 2)
                                    );

                                    screen.flush().unwrap();
                                }

                                _ => continue,
                            }
                        }
                    }
                }

                Key::Char('E') => {
                    print!(
                        " {}{}{}{}{}{}",
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
                                    fs_extra::dir::create(&items.trash_dir, true)
                                        .unwrap_or_else(|_| panic!("cannot empty the trash dir."));
                                    break;
                                }
                                _ => {
                                    break;
                                }
                            }
                        }
                    }
                    print!("{}{}{}", clear::CurrentLine, cursor::Goto(2, 2), DOWN_ARROW);
                    print!(
                        "{}>{}",
                        cursor::Goto(1, STARTING_POINT + 1),
                        cursor::Left(1)
                    );
                    nums.starting_point();
                }

                //Enter the filter mode
                Key::Char('/') => {
                    print!(" {}{} ", cursor::Goto(2, 2), RIGHT_ARROW);
                    print!("{}{}", cursor::Show, cursor::BlinkingBlock);
                    screen.flush().unwrap();

                    let original_list = items.list.clone();

                    let mut keyword: Vec<char> = Vec::new();
                    loop {
                        let (x, _) = screen.cursor_pos().unwrap();
                        let keyword_len = keyword.len();

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
                                    clear_and_show(&current_dir);

                                    items.list = original_list;
                                    items.list_up(0);

                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, STARTING_POINT + 1),
                                        cursor::Left(1)
                                    );

                                    nums.reset();
                                    break;
                                }

                                Key::Left => {
                                    if x == 4 {
                                        continue;
                                    }
                                    print!("{}", cursor::Left(1));
                                    screen.flush().unwrap();
                                }

                                Key::Right => {
                                    if x as usize == keyword_len + 4 {
                                        continue;
                                    }
                                    print!("{}", cursor::Right(1));
                                    screen.flush().unwrap();
                                }

                                //Input char(case-sensitive)
                                Key::Char(c) => {
                                    let memo_x = x;
                                    keyword.insert((x - 4).into(), c);

                                    items.list = original_list
                                        .clone()
                                        .into_iter()
                                        .filter(|entry| {
                                            entry
                                                .file_name
                                                .contains(&keyword.iter().collect::<String>())
                                        })
                                        .collect();

                                    nums.reset_skip();
                                    clear_and_show(&current_dir);
                                    items.list_up(nums.skip);

                                    print!(
                                        "{}{} {}{}",
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        &keyword.iter().collect::<String>(),
                                        cursor::Goto(memo_x + 1, 2)
                                    );

                                    screen.flush().unwrap();
                                }

                                Key::Backspace => {
                                    let memo_x = x;
                                    if x == 4 {
                                        continue;
                                    };
                                    keyword.remove((x - 5).into());

                                    items.list = original_list
                                        .clone()
                                        .into_iter()
                                        .filter(|entry| {
                                            entry
                                                .file_name
                                                .contains(&keyword.iter().collect::<String>())
                                        })
                                        .collect();

                                    nums.reset_skip();
                                    clear_and_show(&current_dir);
                                    items.list_up(nums.skip);

                                    print!(
                                        "{}{} {}{}",
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        &keyword.iter().collect::<String>(),
                                        cursor::Goto(memo_x - 1, 2)
                                    );

                                    screen.flush().unwrap();
                                }

                                _ => continue,
                            }
                        }
                    }
                    print!("{}", cursor::Hide);
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

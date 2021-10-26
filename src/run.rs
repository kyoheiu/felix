use super::config::SortKey;
use super::functions::*;
use super::help::HELP;
use super::nums::*;
use super::state::*;
use std::io::{stdin, stdout, Write};
use std::path::{Path, PathBuf};
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, cursor, screen};

pub fn run(arg: PathBuf) {
    let mut config_dir = dirs::config_dir().unwrap_or_else(|| panic!("cannot read config dir."));
    config_dir.push(FM_CONFIG_DIR);
    let config_file = config_dir.join(PathBuf::from(CONFIG_FILE));
    let trash_dir = config_dir.join(PathBuf::from(TRASH));
    make_config(&config_file, &trash_dir)
        .unwrap_or_else(|_| panic!("cannot make config file or trash dir."));

    if !&arg.exists() {
        println!("Invalid path: {}", &arg.display());
        return;
    }

    let mut state = State::new();
    let mut current_dir = arg.to_path_buf();
    state.update_list(&current_dir);
    state.trash_dir = trash_dir;

    let mut nums = Num::new();

    let (column, row) = termion::terminal_size().unwrap();
    if column < NAME_MAX_LEN as u16 + TIME_START_POS - 3 {
        panic!("too small terminal size.")
    };

    let mut screen = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());

    print!("{}", cursor::Hide);

    clear_and_show(&current_dir);
    state.list_up(nums.skip);

    print!(
        "{}>{}",
        cursor::Goto(1, empty_or_not(state.list.len())),
        cursor::Left(1)
    );
    screen.flush().unwrap();

    let mut memo_v: Vec<CursorMemo> = Vec::new();
    let mut stdin = stdin().keys();

    loop {
        let len = state.list.len();
        let (_, y) = screen.cursor_pos().unwrap();
        let input = stdin.next();

        if let Some(Ok(key)) = input {
            match key {
                //Go up. If lists exceed max-row, lists "scrolls" before the top of the list
                Key::Char('j') | Key::Down => {
                    if nums.index == len - 1 {
                        continue;
                    } else if y == row - 4 && len > (row - STARTING_POINT) as usize - 1 {
                        nums.inc_skip();
                        clear_and_show(&current_dir);
                        state.list_up(nums.skip);
                        print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                        nums.go_down();
                    } else {
                        print!(" {}\n>{}", cursor::Left(1), cursor::Left(1));
                        nums.go_down();
                    }
                }

                //Go down. If lists exceed max-row, lists "scrolls" before the bottom of the list
                Key::Char('k') | Key::Up => {
                    if y == STARTING_POINT {
                    } else if y == STARTING_POINT + 3 && nums.skip != 0 {
                        nums.dec_skip();
                        clear_and_show(&current_dir);
                        state.list_up(nums.skip);
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

                //Go to top
                Key::Char('g') => {
                    if nums.index == 0 {
                        continue;
                    } else if nums.skip != 0 {
                        nums.reset();
                        clear_and_show(&current_dir);
                        state.list_up(nums.skip);
                        print!(" {}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                        nums.go_top();
                    } else {
                        print!(" {}>{}", cursor::Goto(1, STARTING_POINT), cursor::Left(1));
                        nums.go_top();
                    }
                }

                //Go to bottom
                Key::Char('G') => {
                    if len > (row - STARTING_POINT) as usize {
                        nums.skip = (len as u16) - row + STARTING_POINT;
                        clear_and_show(&current_dir);
                        state.list_up(nums.skip);
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

                //Open file or change directory
                Key::Char('l') | Key::Char('\n') | Key::Right => {
                    let item = if len == 1 {
                        state.get_item(0)
                    } else {
                        state.get_item(nums.index)
                    };
                    match item.file_type {
                        FileType::File | FileType::Symlink => {
                            print!("{}", screen::ToAlternateScreen);
                            if let Err(e) = state.open_file(nums.index) {
                                print_warning(e, y);
                            }
                            print!("{}", screen::ToAlternateScreen);
                            clear_and_show(&current_dir);
                            state.list_up(nums.skip);
                            print!("{}{}>{}", cursor::Hide, cursor::Goto(1, y), cursor::Left(1));
                        }
                        FileType::Directory => {
                            match std::fs::File::open(&item.file_path) {
                                Err(e) => {
                                    print_warning(e, y);
                                    continue;
                                }
                                Ok(_) => {
                                    //store the last cursor position and skip number
                                    let cursor_memo = CursorMemo {
                                        num: nums.clone(),
                                        cursor_pos: y,
                                    };
                                    memo_v.push(cursor_memo);

                                    current_dir = item.file_path.clone();
                                    state.update_list(&current_dir);
                                    clear_and_show(&current_dir);
                                    state.list_up(0);
                                    print!(
                                        "{}>{}",
                                        cursor::Goto(1, empty_or_not(state.list.len())),
                                        cursor::Left(1)
                                    );
                                    nums.reset();
                                }
                            }
                        }
                    }
                }

                //Go to parent directory if exists
                Key::Char('h') | Key::Left => match current_dir.parent() {
                    Some(parent_p) => {
                        current_dir = parent_p.to_path_buf();
                        state.update_list(&current_dir);
                        clear_and_show(&current_dir);
                        state.list_up(0);

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

                Key::Char('t') => {
                    match state.sort_by {
                        SortKey::Name => {
                            state.sort_by = SortKey::Time;
                        }
                        SortKey::Time => {
                            state.sort_by = SortKey::Name;
                        }
                    }
                    state.update_list(&current_dir);
                    clear_and_show(&current_dir);
                    state.list_up(0);
                    print!(
                        "{}>{}",
                        cursor::Goto(1, empty_or_not(state.list.len())),
                        cursor::Left(1)
                    );
                    nums.reset();
                }

                Key::Char('D') => {
                    if nums.index == 0 {
                        continue;
                    } else {
                        match &state.warning {
                            true => {
                                print_warning(WHEN_DELETE, y);
                                screen.flush().unwrap();

                                loop {
                                    let input = stdin.next();
                                    if let Some(Ok(key)) = input {
                                        match key {
                                            Key::Char('y') | Key::Char('Y') => {
                                                match state.get_item(nums.index).file_type {
                                                    FileType::Directory => {
                                                        if let Err(e) = state.remove_dir(nums.index)
                                                        {
                                                            print_warning(e, y);
                                                        }
                                                    }
                                                    FileType::File | FileType::Symlink => {
                                                        if let Err(e) =
                                                            state.remove_file(nums.index)
                                                        {
                                                            print_warning(e, y);
                                                        }
                                                    }
                                                }

                                                clear_and_show(&current_dir);
                                                state.list_up(nums.skip);
                                                if nums.index == len - 1 {
                                                    print!(
                                                        "{}>{}",
                                                        cursor::Goto(1, y - 1),
                                                        cursor::Left(1)
                                                    );
                                                    nums.go_up();
                                                } else {
                                                    print!(
                                                        "{}>{}",
                                                        cursor::Goto(1, y),
                                                        cursor::Left(1)
                                                    );
                                                }
                                                break;
                                            }
                                            _ => {
                                                print!(
                                                    "{}{}{}",
                                                    cursor::Goto(2, 2),
                                                    clear::CurrentLine,
                                                    DOWN_ARROW
                                                );
                                                screen.flush().unwrap();

                                                print!(
                                                    "{}{}>{}",
                                                    cursor::Hide,
                                                    cursor::Goto(1, y),
                                                    cursor::Left(1)
                                                );
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            false => {
                                match state.get_item(nums.index).file_type {
                                    FileType::Directory => {
                                        if let Err(e) = state.remove_dir(nums.index) {
                                            print_warning(e, y);
                                        }
                                    }
                                    FileType::File | FileType::Symlink => {
                                        if let Err(e) = state.remove_file(nums.index) {
                                            print_warning(e, y);
                                        }
                                    }
                                }

                                clear_and_show(&current_dir);
                                state.list_up(nums.skip);
                                if nums.index == len - 1 {
                                    print!("{}>{}", cursor::Goto(1, y - 1), cursor::Left(1));
                                    nums.go_up();
                                } else {
                                    print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                                }
                            }
                        }
                    }
                }

                Key::Char('y') => {
                    if nums.index == 0 {
                        continue;
                    }
                    let item = state.get_item(nums.index);
                    state.item_buf = Some(item.clone());
                }

                //todo: paste item of path_buffer
                Key::Char('p') => {
                    let item = state.item_buf.clone();
                    if item == None {
                        continue;
                    } else {
                        match item.unwrap().file_type {
                            FileType::Directory => {
                                if let Err(e) = state.paste_dir(&current_dir) {
                                    print_warning(e, y);
                                }
                            }
                            FileType::File | FileType::Symlink => {
                                if let Err(e) = state.paste_file(&current_dir) {
                                    print_warning(e, y);
                                }
                            }
                        }
                        clear_and_show(&current_dir);
                        state.list_up(nums.skip);
                        print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                    }
                }

                Key::Char('c') => {
                    print!("{}{}", cursor::Show, cursor::BlinkingBlock);
                    let item = state.get_item(nums.index);

                    let mut rename = item.file_name.chars().collect::<Vec<char>>();
                    print!(
                        "{}{}{} {}",
                        cursor::Goto(2, 2),
                        clear::CurrentLine,
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
                                    if let Err(e) =
                                        std::fs::rename(Path::new(&item.file_path), Path::new(&to))
                                    {
                                        print_warning(e, y);
                                        break;
                                    }

                                    clear_and_show(&current_dir);
                                    state.update_list(&current_dir);
                                    state.list_up(nums.skip);

                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, y),
                                        cursor::Left(1)
                                    );
                                    break;
                                }

                                //Exit rename mode and return to original lists
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

                Key::Char('m') => {
                    print!("{}{}", cursor::Show, cursor::BlinkingBlock);

                    let mut new_dir_name: Vec<char> = Vec::new();
                    print!(
                        "{}{}{} ",
                        cursor::Goto(2, 2),
                        clear::CurrentLine,
                        RIGHT_ARROW
                    );
                    screen.flush().unwrap();

                    loop {
                        let eow = new_dir_name.len() + 3;
                        let (x, _) = screen.cursor_pos().unwrap();
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Char('\n') => {
                                    let new_name = new_dir_name.iter().collect::<String>();
                                    let new_name = &current_dir.join(new_name);
                                    if let Err(e) = std::fs::create_dir(&new_name) {
                                        print_warning(e, y);
                                        break;
                                    }

                                    clear_and_show(&current_dir);
                                    state.update_list(&current_dir);
                                    state.list_up(nums.skip);

                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, y),
                                        cursor::Left(1)
                                    );
                                    break;
                                }

                                Key::Esc => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
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
                                    new_dir_name.insert((x - 4).into(), c);

                                    print!(
                                        "{}{}{} {}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        &new_dir_name.iter().collect::<String>(),
                                        cursor::Goto(memo_x + 1, 2)
                                    );

                                    screen.flush().unwrap();
                                }

                                Key::Backspace => {
                                    let memo_x = x;
                                    if x == 4 {
                                        continue;
                                    };
                                    new_dir_name.remove((x - 5).into());

                                    print!(
                                        "{}{}{} {}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        &new_dir_name.iter().collect::<String>(),
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
                    print_warning(WHEN_EMPTY, y);
                    screen.flush().unwrap();

                    loop {
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Char('y') | Key::Char('Y') => {
                                    if let Err(e) = std::fs::remove_dir_all(&state.trash_dir) {
                                        print_warning(e, y);
                                    }
                                    if let Err(e) = std::fs::create_dir(&state.trash_dir) {
                                        print_warning(e, y);
                                    }
                                    break;
                                }
                                _ => {
                                    break;
                                }
                            }
                        }
                    }
                    print!("{}{}{}", cursor::Goto(2, 2), clear::CurrentLine, DOWN_ARROW);
                    print!("{}>{}", cursor::Goto(1, y), cursor::Left(1));
                }

                Key::Char('/') => {
                    print!(
                        " {}{}{} ",
                        cursor::Goto(2, 2),
                        clear::CurrentLine,
                        RIGHT_ARROW
                    );
                    print!("{}{}", cursor::Show, cursor::BlinkingBlock);
                    screen.flush().unwrap();

                    let original_list = state.list.clone();

                    let mut keyword: Vec<char> = Vec::new();
                    loop {
                        let (x, _) = screen.cursor_pos().unwrap();
                        let keyword_len = keyword.len();

                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Char('\n') => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    screen.flush().unwrap();

                                    print!("{}>{}", cursor::Goto(1, 3), cursor::Left(1));
                                    nums.go_top();
                                    break;
                                }

                                Key::Esc => {
                                    clear_and_show(&current_dir);
                                    state.list = original_list;
                                    state.list_up(0);

                                    print!(
                                        "{}{}>{}",
                                        cursor::Hide,
                                        cursor::Goto(1, empty_or_not(state.list.len())),
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

                                    state.list = original_list
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
                                    state.list_up(nums.skip);

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

                                    state.list = original_list
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
                                    state.list_up(nums.skip);

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

                Key::Char('H') => {
                    print!("{}{}", clear::All, cursor::Goto(1, 1));
                    let mut i = 2;
                    for line in HELP.lines() {
                        println!("{}{}", line, cursor::Goto(1, i));
                        i += 1;
                    }
                    println!("\nInput any key to go back.");
                    let _ = stdin.next();
                    clear_and_show(&current_dir);
                    state.list_up(nums.skip);
                    print!("{}{}>{}", cursor::Hide, cursor::Goto(1, y), cursor::Left(1));
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

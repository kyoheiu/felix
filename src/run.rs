use super::config::SortKey;
use super::functions::*;
use super::help::HELP;
use super::nums::*;
use super::state::*;
use std::ffi::OsStr;
// use clipboard::{ClipboardContext, ClipboardProvider};
use std::io::{stdin, stdout, Write};
use std::path::{Path, PathBuf};
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, cursor, screen};

pub fn run(arg: PathBuf) {
    let mut config_dir = dirs::config_dir().unwrap_or_else(|| panic!("cannot read config dir."));
    config_dir.push(FX_CONFIG_DIR);
    let config_file = config_dir.join(PathBuf::from(CONFIG_FILE));
    let trash_dir = config_dir.join(PathBuf::from(TRASH));
    make_config(&config_file, &trash_dir)
        .unwrap_or_else(|_| panic!("cannot make config file or trash dir."));

    if !&arg.exists() {
        println!("Invalid path: {}", &arg.display());
        return;
    }

    let (column, row) = termion::terminal_size().unwrap();
    if column < 21 {
        panic!("too small terminal size.")
    };

    let mut state = State::new();

    let time_start = if column >= 49 { 31 } else { column - 17 };
    let name_max: usize = if column >= 49 {
        29
    } else {
        (time_start - 2).into()
    };
    state.layout = Layout {
        terminal_row: row,
        terminal_column: column,
        name_max_len: name_max,
        time_start_pos: time_start,
    };
    state.current_dir = arg.canonicalize().unwrap();
    state.update_list();
    state.trash_dir = trash_dir;

    let mut filtered = false;

    let mut nums = Num::new();

    let mut screen = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());

    print!("{}", cursor::Hide);

    clear_and_show(&state.current_dir);
    state.list_up(nums.skip);

    state.move_cursor(&nums, STARTING_POINT);
    screen.flush().unwrap();

    let mut p_memo_v: Vec<CursorMemo> = Vec::new();
    let mut c_memo_v: Vec<ChildMemo> = Vec::new();
    let mut stdin = stdin().keys();

    'main: loop {
        let len = state.list.len();
        let (_, y) = screen.cursor_pos().unwrap();
        let input = stdin.next();

        if let Some(Ok(key)) = input {
            match key {
                //Go up. If lists exceed max-row, lists "scrolls" before the top of the list
                Key::Char('j') | Key::Down => {
                    if len == 0 || nums.index == len - 1 {
                        continue;
                    } else if y == state.layout.terminal_row - 4
                        && len > (state.layout.terminal_row - STARTING_POINT) as usize - 1
                    {
                        nums.go_down();
                        nums.inc_skip();
                        clear_and_show(&state.current_dir);
                        state.list_up(nums.skip);
                        state.move_cursor(&nums, y);
                    } else {
                        nums.go_down();
                        print!(" ");
                        state.move_cursor(&nums, y + 1);
                    }
                }

                //Go down. If lists exceed max-row, lists "scrolls" before the bottom of the list
                Key::Char('k') | Key::Up => {
                    if y == STARTING_POINT {
                        continue;
                    } else if y == STARTING_POINT + 3 && nums.skip != 0 {
                        nums.go_up();
                        nums.dec_skip();
                        clear_and_show(&state.current_dir);
                        state.list_up(nums.skip);
                        state.move_cursor(&nums, STARTING_POINT + 3);
                    } else {
                        nums.go_up();
                        print!(" ");
                        state.move_cursor(&nums, y - 1);
                    }
                }

                //Go to top
                Key::Char('g') => {
                    if nums.index == 0 {
                        continue;
                    } else {
                        print!("{}{}g", cursor::Goto(2, 2), clear::CurrentLine,);
                        print!("{}{}", cursor::Show, cursor::BlinkingBar);

                        screen.flush().unwrap();

                        'top: loop {
                            let input = stdin.next();
                            if let Some(Ok(key)) = input {
                                match key {
                                    Key::Char('g') => {
                                        print!("{}", cursor::Hide);
                                        nums.reset();
                                        clear_and_show(&state.current_dir);
                                        state.list_up(0);
                                        print!(" ");
                                        state.move_cursor(&nums, STARTING_POINT);
                                        break 'top;
                                    }

                                    _ => {
                                        print!("{}", clear::CurrentLine);
                                        print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                        print!("{}", cursor::Hide);
                                        state.move_cursor(&nums, y);
                                        break 'top;
                                    }
                                }
                            }
                        }
                    }
                }

                //Go to bottom
                Key::Char('G') => {
                    if len == 0 {
                        continue;
                    }
                    nums.go_bottom(len - 1);
                    if len > (state.layout.terminal_row - STARTING_POINT) as usize {
                        nums.skip = (len as u16) + STARTING_POINT - state.layout.terminal_row;
                        clear_and_show(&state.current_dir);
                        state.list_up(nums.skip);
                        state.move_cursor(&nums, state.layout.terminal_row - 1);
                    } else {
                        print!(" ");
                        state.move_cursor(&nums, len as u16 + STARTING_POINT - 1);
                    }
                }

                //Open file or change directory
                Key::Char('l') | Key::Char('\n') | Key::Right => {
                    if let Ok(item) = state.get_item(nums.index) {
                        match item.file_type {
                            FileType::File | FileType::Symlink => {
                                print!("{}", screen::ToAlternateScreen);
                                if state.open_file(nums.index).is_err() {
                                    print_warning("Cannot open file. Check your config!", y);
                                    continue;
                                }
                                print!("{}", screen::ToAlternateScreen);
                                clear_and_show(&state.current_dir);
                                state.list_up(nums.skip);
                                print!("{}", cursor::Hide);
                                state.move_cursor(&nums, y);
                            }
                            FileType::Directory => {
                                match std::fs::File::open(&item.file_path) {
                                    Err(e) => {
                                        print_warning(e, y);
                                        continue;
                                    }
                                    Ok(_) => {
                                        //store the last cursor position and skip number
                                        let cursor_memo = if !filtered {
                                            CursorMemo {
                                                num: nums.clone(),
                                                cursor_pos: y,
                                            }
                                        } else {
                                            CursorMemo {
                                                num: Num::new(),
                                                cursor_pos: STARTING_POINT,
                                            }
                                        };
                                        p_memo_v.push(cursor_memo);
                                        filtered = false;

                                        state.current_dir = item.file_path.clone();
                                        if let Err(e) =
                                            std::env::set_current_dir(&state.current_dir)
                                        {
                                            print_warning(e, y);
                                            continue;
                                        }
                                        state.update_list();

                                        match c_memo_v.pop() {
                                            Some(memo) => {
                                                if state.current_dir == memo.dir_path {
                                                    nums = memo.cursor_memo.num;
                                                    clear_and_show(&state.current_dir);
                                                    state.list_up(nums.skip);
                                                    state.move_cursor(
                                                        &nums,
                                                        memo.cursor_memo.cursor_pos,
                                                    );
                                                } else {
                                                    clear_and_show(&state.current_dir);
                                                    state.list_up(0);
                                                    nums.reset();
                                                    state.move_cursor(&nums, STARTING_POINT);
                                                }
                                            }
                                            None => {
                                                clear_and_show(&state.current_dir);
                                                state.list_up(0);
                                                nums.reset();
                                                state.move_cursor(&nums, STARTING_POINT);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                //Go to parent directory if exists
                Key::Char('h') | Key::Left => {
                    let pre = state.current_dir.clone();

                    match state.current_dir.parent() {
                        Some(parent_p) => {
                            let cursor_memo = if !filtered {
                                ChildMemo {
                                    dir_path: pre.clone(),
                                    cursor_memo: CursorMemo {
                                        num: nums.clone(),
                                        cursor_pos: y,
                                    },
                                }
                            } else {
                                ChildMemo {
                                    dir_path: PathBuf::from(""),
                                    cursor_memo: CursorMemo {
                                        num: Num::new(),
                                        cursor_pos: STARTING_POINT,
                                    },
                                }
                            };
                            c_memo_v.push(cursor_memo);
                            filtered = false;

                            state.current_dir = parent_p.to_path_buf();
                            std::env::set_current_dir(&state.current_dir)
                                .unwrap_or_else(|e| print_warning(e, y));
                            state.update_list();

                            match p_memo_v.pop() {
                                Some(memo) => {
                                    nums = memo.num;
                                    clear_and_show(&state.current_dir);
                                    state.list_up(nums.skip);
                                    state.move_cursor(&nums, memo.cursor_pos);
                                }
                                None => match pre.file_name() {
                                    Some(name) => {
                                        let mut new_pos = 0;
                                        for (i, item) in state.list.iter().enumerate() {
                                            let name_as_os_str: &OsStr = item.file_name.as_ref();
                                            if name_as_os_str == name {
                                                new_pos = i;
                                            }
                                        }
                                        nums.index = new_pos;

                                        if nums.index
                                            >= (state.layout.terminal_row - (STARTING_POINT + 3))
                                                .into()
                                        {
                                            nums.skip = (nums.index - 3) as u16;
                                            clear_and_show(&state.current_dir);
                                            state.list_up(nums.skip);
                                            state.move_cursor(&nums, STARTING_POINT + 3);
                                        } else {
                                            nums.skip = 0;
                                            clear_and_show(&state.current_dir);
                                            state.list_up(0);
                                            state.move_cursor(&nums, (nums.index + 3) as u16);
                                        }
                                    }
                                    None => {
                                        nums.reset();
                                        clear_and_show(&state.current_dir);
                                        state.list_up(0);
                                        state.move_cursor(&nums, STARTING_POINT);
                                    }
                                },
                            }
                        }
                        None => {
                            continue;
                        }
                    }
                }

                Key::Char('V') => {
                    if len == 0 {
                        continue;
                    }
                    let mut item = state.list.get_mut(nums.index).unwrap();
                    item.selected = true;

                    clear_and_show(&state.current_dir);
                    state.list_up(nums.skip);
                    state.move_cursor(&nums, y);
                    screen.flush().unwrap();

                    let start_pos = nums.index;

                    loop {
                        let (_, y) = screen.cursor_pos().unwrap();
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Char('j') | Key::Down => {
                                    if nums.index == len - 1 {
                                        continue;
                                    } else if y == state.layout.terminal_row - 4
                                        && len
                                            > (state.layout.terminal_row - STARTING_POINT) as usize
                                                - 1
                                    {
                                        nums.inc_skip();
                                        nums.go_down();

                                        if nums.index > start_pos {
                                            let mut item = state.list.get_mut(nums.index).unwrap();
                                            item.selected = true;
                                        } else if nums.index < start_pos {
                                            let mut item =
                                                state.list.get_mut(nums.index - 1).unwrap();
                                            item.selected = false;
                                        }

                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        state.move_cursor(&nums, y);
                                        screen.flush().unwrap();
                                    } else {
                                        nums.go_down();

                                        if nums.index > start_pos {
                                            let mut item = state.list.get_mut(nums.index).unwrap();
                                            item.selected = true;
                                        } else if nums.index <= start_pos {
                                            let mut item =
                                                state.list.get_mut(nums.index - 1).unwrap();
                                            item.selected = false;
                                        }

                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        state.move_cursor(&nums, y + 1);
                                        screen.flush().unwrap();
                                    }
                                }

                                Key::Char('k') | Key::Up => {
                                    if y == STARTING_POINT {
                                        continue;
                                    } else if y == STARTING_POINT + 3 && nums.skip != 0 {
                                        nums.dec_skip();
                                        nums.go_up();

                                        if nums.index >= start_pos {
                                            let mut item =
                                                state.list.get_mut(nums.index + 1).unwrap();
                                            item.selected = false;
                                        } else if nums.index < start_pos {
                                            let mut item = state.list.get_mut(nums.index).unwrap();
                                            item.selected = true;
                                        }

                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        state.move_cursor(&nums, STARTING_POINT + 3);
                                        screen.flush().unwrap();
                                    } else {
                                        nums.go_up();

                                        if nums.index >= start_pos {
                                            let mut item =
                                                state.list.get_mut(nums.index + 1).unwrap();
                                            item.selected = false;
                                        } else if nums.index < start_pos {
                                            let mut item = state.list.get_mut(nums.index).unwrap();
                                            item.selected = true;
                                        }

                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        state.move_cursor(&nums, y - 1);
                                        screen.flush().unwrap();
                                    }
                                }

                                Key::Char('g') => {
                                    if nums.index == 0 {
                                        continue;
                                    } else {
                                        print!("{}{}g", cursor::Goto(2, 2), clear::CurrentLine,);
                                        print!("{}{}", cursor::Show, cursor::BlinkingBar);

                                        screen.flush().unwrap();

                                        'top_select: loop {
                                            let input = stdin.next();
                                            if let Some(Ok(key)) = input {
                                                match key {
                                                    Key::Char('g') => {
                                                        print!("{}", cursor::Hide);
                                                        nums.reset();
                                                        state.select_from_top(start_pos);
                                                        clear_and_show(&state.current_dir);
                                                        state.list_up(0);
                                                        print!(
                                                            " {}>{}",
                                                            cursor::Goto(1, STARTING_POINT),
                                                            cursor::Left(1)
                                                        );
                                                        break 'top_select;
                                                    }

                                                    _ => {
                                                        print!("{}", clear::CurrentLine);
                                                        print!(
                                                            "{}{}",
                                                            cursor::Goto(2, 2),
                                                            DOWN_ARROW
                                                        );
                                                        print!("{}", cursor::Hide);
                                                        state.move_cursor(&nums, y);
                                                        break 'top_select;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                Key::Char('G') => {
                                    if len > (state.layout.terminal_row - STARTING_POINT) as usize {
                                        nums.skip = (len as u16) + STARTING_POINT
                                            - state.layout.terminal_row;
                                        nums.go_bottom(len - 1);
                                        state.select_to_bottom(start_pos);
                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        state.move_cursor(&nums, state.layout.terminal_row - 1);
                                    } else {
                                        nums.go_bottom(len - 1);
                                        state.select_to_bottom(start_pos);
                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        print!(" ");
                                        state.move_cursor(&nums, len as u16 + STARTING_POINT - 1);
                                    }
                                }

                                Key::Char('d') => {
                                    print_info("Processing...", y);
                                    screen.flush().unwrap();

                                    state.registered.clear();
                                    let iter = state.list.clone().into_iter();
                                    let mut i = 0;
                                    for item in iter {
                                        if item.selected {
                                            match item.file_type {
                                                FileType::Directory => {
                                                    if let Err(e) = state.remove_and_yank_dir(item)
                                                    {
                                                        print_warning(e, y);
                                                        break;
                                                    }
                                                }
                                                FileType::File | FileType::Symlink => {
                                                    if let Err(e) = state.remove_and_yank_file(item)
                                                    {
                                                        print_warning(e, y);
                                                        break;
                                                    }
                                                }
                                            }
                                            i += 1;
                                        }
                                    }
                                    clear_and_show(&state.current_dir);
                                    state.update_list();
                                    state.list_up(nums.skip);

                                    let mut delete_message: String = i.to_string();
                                    delete_message.push_str(" items deleted");
                                    print_info(delete_message, y);
                                    print!(" ");

                                    let new_len = state.list.len();
                                    if new_len == 0 {
                                        nums.reset();
                                        state.move_cursor(&nums, STARTING_POINT);
                                    } else if nums.index > new_len - 1 {
                                        let new_y = y - (nums.index - (new_len - 1)) as u16;
                                        nums.index = new_len - 1;
                                        state.move_cursor(&nums, new_y)
                                    } else {
                                        state.move_cursor(&nums, y);
                                    }
                                    break;
                                }

                                Key::Char('y') => {
                                    state.yank_item(nums.index, true);
                                    state.reset_selection();
                                    clear_and_show(&state.current_dir);
                                    state.list_up(nums.skip);

                                    let mut yank_message: String =
                                        state.registered.len().to_string();
                                    yank_message.push_str(" items yanked");
                                    print_info(yank_message, y);

                                    state.move_cursor(&nums, y);
                                    break;
                                }

                                Key::Esc => {
                                    state.reset_selection();
                                    clear_and_show(&state.current_dir);
                                    state.list_up(nums.skip);
                                    state.move_cursor(&nums, y);
                                    break;
                                }

                                _ => {
                                    continue;
                                }
                            }
                        }
                        screen.flush().unwrap();
                    }
                }

                Key::Char('t') => {
                    match state.sort_by {
                        SortKey::Name => {
                            state.sort_by = SortKey::Time;
                        }
                        SortKey::Time => {
                            state.sort_by = SortKey::Name;
                        }
                    }
                    state.update_list();
                    clear_and_show(&state.current_dir);
                    state.list_up(0);
                    nums.reset();
                    state.move_cursor(&nums, STARTING_POINT);
                }

                Key::Char('d') => {
                    if len == 0 {
                        continue;
                    } else {
                        print!("{}{}d", cursor::Goto(2, 2), clear::CurrentLine,);
                        print!("{}{}", cursor::Show, cursor::BlinkingBar);

                        screen.flush().unwrap();

                        'delete: loop {
                            let input = stdin.next();
                            if let Some(Ok(key)) = input {
                                match key {
                                    Key::Char('d') => {
                                        print_info("Processing...", y);
                                        screen.flush().unwrap();

                                        state.registered.clear();
                                        let item = state.get_item(nums.index).unwrap().clone();
                                        match item.file_type {
                                            FileType::Directory => {
                                                if let Err(e) = state.remove_and_yank_dir(item) {
                                                    print!("{}", cursor::Hide);
                                                    print_warning(e, y);
                                                    state.move_cursor(&nums, y);
                                                    break 'delete;
                                                }
                                            }
                                            FileType::File | FileType::Symlink => {
                                                if let Err(e) = state.remove_and_yank_file(item) {
                                                    clear_and_show(&state.current_dir);
                                                    print!("{}", cursor::Hide);
                                                    print_warning(e, y);
                                                    state.move_cursor(&nums, y);
                                                    break 'delete;
                                                }
                                            }
                                        }

                                        clear_and_show(&state.current_dir);
                                        print!("{}", cursor::Hide);
                                        state.update_list();
                                        state.list_up(nums.skip);
                                        let cursor_pos = if state.list.is_empty() {
                                            STARTING_POINT
                                        } else if nums.index == len - 1 {
                                            nums.go_up();
                                            y - 1
                                        } else {
                                            y
                                        };
                                        print_info("1 item deleted", cursor_pos);
                                        state.move_cursor(&nums, cursor_pos);
                                        break 'delete;
                                    }
                                    _ => {
                                        print!("{}", clear::CurrentLine);
                                        print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                        print!("{}", cursor::Hide);
                                        state.move_cursor(&nums, y);
                                        break 'delete;
                                    }
                                }
                            }
                        }
                    }
                }

                Key::Char('y') => {
                    if len == 0 {
                        continue;
                    }
                    print!("{}{}y", cursor::Goto(2, 2), clear::CurrentLine,);
                    print!("{}{}", cursor::Show, cursor::BlinkingBar);

                    screen.flush().unwrap();

                    'yank: loop {
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Char('y') => {
                                    state.yank_item(nums.index, false);
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                    print_info("1 item yanked", y);
                                    break 'yank;
                                }

                                _ => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                    break 'yank;
                                }
                            }
                        }
                    }
                }

                Key::Char('p') => {
                    if state.registered.is_empty() {
                        continue;
                    }
                    print_info("Processing...", y);
                    screen.flush().unwrap();

                    if let Err(e) = state.put_items() {
                        print_warning(e, y);
                        continue;
                    }

                    clear_and_show(&state.current_dir);
                    state.update_list();
                    state.list_up(nums.skip);

                    let mut put_message: String = state.registered.len().to_string();
                    put_message.push_str(" items inserted");
                    print_info(put_message, y);
                    state.move_cursor(&nums, y);
                }

                Key::Char('c') => {
                    if len == 0 {
                        continue;
                    }
                    print!("{}{}", cursor::Show, cursor::BlinkingBar);
                    let item = state.get_item(nums.index).unwrap();

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
                                    let mut to = state.current_dir.clone();
                                    to.push(rename);
                                    if let Err(e) =
                                        std::fs::rename(Path::new(&item.file_path), Path::new(&to))
                                    {
                                        print!("{}", cursor::Hide);
                                        print_warning(e, y);
                                        break;
                                    }

                                    clear_and_show(&state.current_dir);
                                    state.update_list();
                                    state.list_up(nums.skip);

                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                    break;
                                }

                                Key::Esc => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    screen.flush().unwrap();

                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
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

                Key::Char('/') => {
                    if len == 0 {
                        continue;
                    }
                    print!(
                        " {}{}{} ",
                        cursor::Goto(2, 2),
                        clear::CurrentLine,
                        RIGHT_ARROW
                    );
                    print!("{}{}", cursor::Show, cursor::BlinkingBar);
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
                                    filtered = true;
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    screen.flush().unwrap();

                                    nums.reset();
                                    state.move_cursor(&nums, STARTING_POINT);
                                    break;
                                }

                                Key::Esc => {
                                    clear_and_show(&state.current_dir);
                                    state.list = original_list;
                                    state.list_up(nums.skip);

                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);

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

                                    clear_and_show(&state.current_dir);
                                    state.list_up(0);

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
                                    clear_and_show(&state.current_dir);
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

                Key::Char(':') => {
                    print!(" {}{}:", cursor::Goto(2, 2), clear::CurrentLine,);
                    print!("{}{}", cursor::Show, cursor::BlinkingBar);

                    let mut command: Vec<char> = Vec::new();
                    screen.flush().unwrap();

                    'command: loop {
                        let eow = command.len() + 2;
                        let (x, _) = screen.cursor_pos().unwrap();
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Char('\n') => {
                                    if command.is_empty() {
                                        print!("{}", clear::CurrentLine);
                                        print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                        print!("{}", cursor::Hide);
                                        state.move_cursor(&nums, y);
                                        break;
                                    }

                                    if command == vec!['q'] {
                                        break 'main;
                                    } else if command == vec!['e'] {
                                        state.update_list();
                                        clear_and_show(&state.current_dir);
                                        state.list_up(0);
                                        print!("{}", cursor::Hide);
                                        nums.reset();
                                        state.move_cursor(&nums, STARTING_POINT);
                                        break 'command;
                                    } else if command == vec!['h'] {
                                        print!("{}", cursor::Hide);
                                        print!("{}{}", clear::All, cursor::Goto(1, 1));
                                        let mut i = 2;
                                        for line in HELP.lines() {
                                            println!("{}{}", line, cursor::Goto(1, i));
                                            i += 1;
                                        }
                                        println!("\nInput any key to go back.");
                                        let _ = stdin.next();
                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        state.move_cursor(&nums, y);
                                        break 'command;
                                    }

                                    let commands: String = command.iter().collect();

                                    let commands = commands.split_ascii_whitespace();

                                    let mut c = "";
                                    let mut args = Vec::new();
                                    let mut i = 0;
                                    for s in commands {
                                        if i == 0 {
                                            c = s;
                                            i += 1;
                                        } else {
                                            args.push(s);
                                        }
                                    }

                                    if c == "empty" && args.is_empty() {
                                        print_warning(WHEN_EMPTY, y);
                                        screen.flush().unwrap();

                                        'empty: loop {
                                            let input = stdin.next();
                                            if let Some(Ok(key)) = input {
                                                match key {
                                                    Key::Char('y') | Key::Char('Y') => {
                                                        print_info("Processing...", y);
                                                        screen.flush().unwrap();

                                                        if let Err(e) = std::fs::remove_dir_all(
                                                            &state.trash_dir,
                                                        ) {
                                                            print!("{}", cursor::Hide);
                                                            print_warning(e, y);
                                                            continue 'main;
                                                        }
                                                        if let Err(e) =
                                                            std::fs::create_dir(&state.trash_dir)
                                                        {
                                                            print!("{}", cursor::Hide);
                                                            print_warning(e, y);
                                                            continue 'main;
                                                        }
                                                        break 'empty;
                                                    }
                                                    _ => {
                                                        break 'empty;
                                                    }
                                                }
                                            }
                                        }
                                        print!(
                                            "{}{}{}{}",
                                            cursor::Hide,
                                            cursor::Goto(2, 2),
                                            clear::CurrentLine,
                                            DOWN_ARROW
                                        );
                                        if state.current_dir == state.trash_dir {
                                            clear_and_show(&state.current_dir);
                                            state.update_list();
                                            state.list_up(nums.skip);
                                            state.move_cursor(&nums, STARTING_POINT);
                                        } else {
                                            state.move_cursor(&nums, y);
                                        }
                                        break 'command;
                                    }

                                    print!("{}", screen::ToAlternateScreen);
                                    if std::env::set_current_dir(&state.current_dir).is_err() {
                                        print!("{}", cursor::Hide,);
                                        print_warning("cannot execute command", y);
                                        break 'command;
                                    }
                                    if std::process::Command::new(c).args(args).status().is_err() {
                                        print!("{}", screen::ToAlternateScreen);

                                        clear_and_show(&state.current_dir);
                                        state.update_list();
                                        state.list_up(nums.skip);

                                        print!("{}", cursor::Hide,);
                                        print_warning("cannot execute command", y);
                                        break 'command;
                                    }
                                    print!("{}", screen::ToAlternateScreen);

                                    clear_and_show(&state.current_dir);
                                    state.update_list();
                                    state.list_up(nums.skip);

                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                    break 'command;
                                }

                                Key::Esc => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                    break 'command;
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

                                Key::Char(c) => {
                                    command.insert((x - 3).into(), c);

                                    print!(
                                        "{}{}:{}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        &command.iter().collect::<String>(),
                                        cursor::Goto(x + 1, 2)
                                    );

                                    screen.flush().unwrap();
                                }

                                Key::Backspace => {
                                    if x == 3 {
                                        continue;
                                    };
                                    command.remove((x - 4).into());

                                    print!(
                                        "{}{}:{}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        &command.iter().collect::<String>(),
                                        cursor::Goto(x - 1, 2)
                                    );

                                    screen.flush().unwrap();
                                }

                                _ => continue,
                            }
                        }
                    }
                }

                Key::Char('Z') => {
                    print!(" {}{}Z", cursor::Goto(2, 2), clear::CurrentLine,);
                    print!("{}{}", cursor::Show, cursor::BlinkingBar);

                    let mut command: Vec<char> = vec!['Z'];
                    screen.flush().unwrap();

                    'quit: loop {
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Esc => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                    break 'quit;
                                }

                                Key::Char(c) => {
                                    command.push(c);

                                    if command == vec!['Z', 'Z'] {
                                        break 'main;
                                    } else {
                                        print!("{}", clear::CurrentLine);
                                        print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                        print!("{}", cursor::Hide);
                                        state.move_cursor(&nums, y);
                                        break 'quit;
                                    }
                                }

                                _ => continue,
                            }
                        }
                    }
                }

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

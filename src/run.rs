use super::errors::MyError;
use super::functions::*;
use super::help::HELP;
use super::nums::*;
use super::state::*;
use crate::session::*;
use std::ffi::OsStr;
// use clipboard::{ClipboardContext, ClipboardProvider};
use log::debug;
use std::io::{stdin, stdout, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, cursor, screen};

pub fn run(arg: PathBuf) -> Result<(), MyError> {
    debug!("Initial setup starts.");

    let mut config_dir_path =
        dirs::config_dir().unwrap_or_else(|| panic!("Cannot read config dir."));
    config_dir_path.push(FX_CONFIG_DIR);
    let config_file_path = config_dir_path.join(PathBuf::from(CONFIG_FILE));
    let trash_dir_path = config_dir_path.join(PathBuf::from(TRASH));
    make_config(&config_file_path, &trash_dir_path)
        .unwrap_or_else(|_| panic!("Cannot make config file or trash dir."));
    let session_file_path = config_dir_path.join(PathBuf::from(SESSION_FILE));
    make_session(&session_file_path).unwrap_or_else(|_| panic!("Cannot make session file."));

    if !&arg.exists() {
        println!(
            "Invalid path or argument: {}\n`fx -h` shows help.",
            &arg.display()
        );
        return Ok(());
    }

    let mut state = State::new();
    state.trash_dir = trash_dir_path;
    state.current_dir = arg.canonicalize()?;

    let mut filtered = false;

    let mut nums = Num::new();

    let mut screen = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());

    print!("{}", cursor::Hide);

    state.update_list()?;
    clear_and_show(&state.current_dir);
    state.list_up(nums.skip);

    state.move_cursor(&nums, STARTING_POINT);
    screen.flush()?;

    let mut p_memo_v: Vec<ParentMemo> = Vec::new();
    let mut c_memo_v: Vec<ChildMemo> = Vec::new();
    let mut stdin = stdin().keys();

    'main: loop {
        let len = state.list.len();
        let (_, y) = screen.cursor_pos()?;
        let input = stdin.next();

        if let Some(Ok(key)) = input {
            match key {
                //Go up. If lists exceed max-row, lists "scrolls" before the top of the list
                Key::Char('j') | Key::Down => {
                    if len == 0 || nums.index == len - 1 {
                        continue;
                    } else if y >= state.layout.terminal_row - 4
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
                        print!("{}", cursor::Show);

                        screen.flush()?;

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
                    if len > (state.layout.terminal_row - STARTING_POINT) as usize {
                        nums.skip = (len as u16) + STARTING_POINT - state.layout.terminal_row;
                        nums.go_bottom(len - 1);
                        clear_and_show(&state.current_dir);
                        state.list_up(nums.skip);
                        state.move_cursor(&nums, state.layout.terminal_row - 1);
                    } else {
                        nums.reset();
                        nums.go_bottom(len - 1);
                        clear_and_show(&state.current_dir);
                        state.list_up(nums.skip);
                        state.move_cursor(&nums, len as u16 + STARTING_POINT - 1);
                    }
                }

                //Open file or change directory
                Key::Char('l') | Key::Char('\n') | Key::Right => {
                    if let Ok(item) = state.get_item(nums.index) {
                        match item.file_type {
                            FileType::File => {
                                print!("{}", screen::ToAlternateScreen);
                                if let Err(e) = state.open_file(nums.index) {
                                    print_warning(e, y);
                                    continue;
                                }
                                print!("{}", screen::ToAlternateScreen);
                                clear_and_show(&state.current_dir);
                                state.update_list()?;
                                state.list_up(nums.skip);
                                print!("{}", cursor::Hide);
                                state.move_cursor(&nums, y);
                            }
                            FileType::Symlink => match &item.symlink_dir_path {
                                Some(true_path) => match std::fs::File::open(true_path) {
                                    Err(e) => {
                                        print_warning(e, y);
                                        continue;
                                    }
                                    Ok(_) => {
                                        let cursor_memo = if !filtered {
                                            ParentMemo {
                                                to_sym_dir: Some(state.current_dir.clone()),
                                                num: nums.clone(),
                                                cursor_pos: y,
                                            }
                                        } else {
                                            ParentMemo {
                                                to_sym_dir: Some(state.current_dir.clone()),
                                                num: Num::new(),
                                                cursor_pos: STARTING_POINT,
                                            }
                                        };
                                        p_memo_v.push(cursor_memo);
                                        filtered = false;

                                        state.current_dir = true_path.clone();
                                        if let Err(e) =
                                            std::env::set_current_dir(&state.current_dir)
                                        {
                                            print_warning(e, y);
                                            continue;
                                        }
                                        state.update_list()?;
                                        clear_and_show(&state.current_dir);
                                        nums.reset();
                                        state.list_up(nums.skip);
                                        state.move_cursor(&nums, STARTING_POINT);
                                    }
                                },
                                None => {
                                    print!("{}", screen::ToAlternateScreen);
                                    if let Err(e) = state.open_file(nums.index) {
                                        print_warning(e, y);
                                        continue;
                                    }
                                    print!("{}", screen::ToAlternateScreen);
                                    clear_and_show(&state.current_dir);
                                    state.list_up(nums.skip);
                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                }
                            },
                            FileType::Directory => {
                                match std::fs::File::open(&item.file_path) {
                                    Err(e) => {
                                        print_warning(e, y);
                                        continue;
                                    }
                                    Ok(_) => {
                                        //store the last cursor position and skip number
                                        let cursor_memo = if !filtered {
                                            ParentMemo {
                                                to_sym_dir: None,
                                                num: nums.clone(),
                                                cursor_pos: y,
                                            }
                                        } else {
                                            ParentMemo {
                                                to_sym_dir: None,
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
                                        state.update_list()?;

                                        match c_memo_v.pop() {
                                            Some(memo) => {
                                                if state.current_dir == memo.dir_path {
                                                    nums = memo.num;
                                                    clear_and_show(&state.current_dir);
                                                    state.list_up(nums.skip);
                                                    state.move_cursor(&nums, memo.cursor_pos);
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
                                    num: nums.clone(),
                                    cursor_pos: y,
                                }
                            } else {
                                ChildMemo {
                                    dir_path: PathBuf::from(""),
                                    num: Num::new(),
                                    cursor_pos: STARTING_POINT,
                                }
                            };
                            c_memo_v.push(cursor_memo);
                            filtered = false;

                            match p_memo_v.pop() {
                                Some(memo) => {
                                    match memo.to_sym_dir {
                                        Some(true_path) => {
                                            state.current_dir = true_path;
                                        }
                                        None => {
                                            state.current_dir = parent_p.to_path_buf();
                                        }
                                    }
                                    if let Err(e) = std::env::set_current_dir(&state.current_dir) {
                                        print_warning(e, y);
                                        continue;
                                    }
                                    state.update_list()?;
                                    nums = memo.num;
                                    clear_and_show(&state.current_dir);
                                    state.list_up(nums.skip);
                                    state.move_cursor(&nums, memo.cursor_pos);
                                }
                                None => {
                                    state.current_dir = parent_p.to_path_buf();
                                    if let Err(e) = std::env::set_current_dir(&state.current_dir) {
                                        print_warning(e, y);
                                        continue;
                                    }
                                    state.update_list()?;
                                    match pre.file_name() {
                                        Some(name) => {
                                            let mut new_pos = 0;
                                            for (i, item) in state.list.iter().enumerate() {
                                                let name_as_os_str: &OsStr =
                                                    item.file_name.as_ref();
                                                if name_as_os_str == name {
                                                    new_pos = i;
                                                }
                                            }
                                            nums.index = new_pos;

                                            if nums.index
                                                >= (state.layout.terminal_row
                                                    - (STARTING_POINT + 3))
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
                                    }
                                }
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
                    screen.flush()?;

                    let start_pos = nums.index;

                    loop {
                        let (_, y) = screen.cursor_pos()?;
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
                                        screen.flush()?;
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
                                        screen.flush()?;
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
                                        screen.flush()?;
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
                                        print!("{}", cursor::Show);

                                        screen.flush()?;

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
                                    let start = Instant::now();
                                    screen.flush()?;

                                    state.registered.clear();
                                    let mut count: usize = 0;
                                    let clone = state.list.clone();
                                    let iter = clone.iter().filter(|item| item.selected);
                                    let total_selected = iter.clone().count();
                                    for (i, item) in iter.enumerate() {
                                        print!(
                                            " {}{}{}",
                                            cursor::Goto(2, 2),
                                            clear::CurrentLine,
                                            display_count(i, total_selected)
                                        );
                                        match item.file_type {
                                            FileType::Directory => {
                                                if let Err(e) =
                                                    state.remove_and_yank_dir(item.clone())
                                                {
                                                    print_warning(e, y);
                                                    break;
                                                }
                                            }
                                            FileType::File | FileType::Symlink => {
                                                if let Err(e) =
                                                    state.remove_and_yank_file(item.clone())
                                                {
                                                    print_warning(e, y);
                                                    break;
                                                }
                                            }
                                        }
                                        count += 1;
                                    }
                                    clear_and_show(&state.current_dir);
                                    state.update_list()?;
                                    state.list_up(nums.skip);

                                    let duration = duration_to_string(start.elapsed());
                                    let delete_message: String = {
                                        if count == 1 {
                                            format!("1 item deleted [{}]", duration)
                                        } else {
                                            let mut count = count.to_string();
                                            count.push_str(&format!(
                                                " items deleted [{}]",
                                                duration
                                            ));
                                            count
                                        }
                                    };
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
                        screen.flush()?;
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
                    state.update_list()?;
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
                        print!("{}", cursor::Show);

                        screen.flush()?;

                        'delete: loop {
                            let input = stdin.next();
                            if let Some(Ok(key)) = input {
                                match key {
                                    Key::Char('d') => {
                                        print!("{}", cursor::Hide);
                                        print_info("Processing...", y);
                                        let start = Instant::now();
                                        screen.flush()?;

                                        state.registered.clear();
                                        let item = state.get_item(nums.index)?.clone();
                                        match item.file_type {
                                            FileType::Directory => {
                                                print!(
                                                    " {}{}1/1",
                                                    cursor::Goto(2, 2),
                                                    clear::CurrentLine
                                                );
                                                if let Err(e) = state.remove_and_yank_dir(item) {
                                                    print_warning(e, y);
                                                    state.move_cursor(&nums, y);
                                                    break 'delete;
                                                }
                                            }
                                            FileType::File | FileType::Symlink => {
                                                if let Err(e) = state.remove_and_yank_file(item) {
                                                    clear_and_show(&state.current_dir);
                                                    print_warning(e, y);
                                                    state.move_cursor(&nums, y);
                                                    break 'delete;
                                                }
                                            }
                                        }

                                        clear_and_show(&state.current_dir);
                                        state.update_list()?;
                                        state.list_up(nums.skip);
                                        let cursor_pos = if state.list.is_empty() {
                                            STARTING_POINT
                                        } else if nums.index == len - 1 {
                                            nums.go_up();
                                            y - 1
                                        } else {
                                            y
                                        };
                                        let duration = duration_to_string(start.elapsed());
                                        print_info(
                                            format!("1 item deleted [{}]", duration),
                                            cursor_pos,
                                        );
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
                    print!("{}", cursor::Show);

                    screen.flush()?;

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
                    let start = Instant::now();
                    screen.flush()?;

                    if let Err(e) = state.put_items() {
                        print_warning(e, y);
                        continue;
                    }

                    clear_and_show(&state.current_dir);
                    state.update_list()?;
                    state.list_up(nums.skip);

                    let duration = duration_to_string(start.elapsed());
                    let registered_len = state.registered.len();
                    let mut put_message: String = registered_len.to_string();
                    if registered_len == 1 {
                        put_message.push_str(&format!(" item inserted [{}]", duration));
                    } else {
                        put_message.push_str(&format!(" items inserted [{}]", duration));
                    }
                    print_info(put_message, y);
                    state.move_cursor(&nums, y);
                }

                Key::Char('c') => {
                    if len == 0 {
                        continue;
                    }
                    print!("{}", cursor::Show);
                    let item = state.get_item(nums.index).unwrap();

                    let mut rename = item.file_name.chars().collect::<Vec<char>>();
                    print!(
                        "{}{}{} {}",
                        cursor::Goto(2, 2),
                        clear::CurrentLine,
                        RIGHT_ARROW,
                        &rename.iter().collect::<String>(),
                    );
                    screen.flush()?;

                    loop {
                        let eow = rename.len() + 3;
                        let (x, _) = screen.cursor_pos()?;
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
                                    state.update_list()?;
                                    state.list_up(nums.skip);

                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                    break;
                                }

                                Key::Esc => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);

                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                    break;
                                }

                                Key::Left => {
                                    if x == 4 {
                                        continue;
                                    };
                                    print!("{}", cursor::Left(1));
                                }

                                Key::Right => {
                                    if x as usize == eow + 1 {
                                        continue;
                                    };
                                    print!("{}", cursor::Right(1));
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
                                }

                                _ => continue,
                            }
                            screen.flush()?;
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
                    print!("{}", cursor::Show);
                    screen.flush()?;

                    let original_list = state.list.clone();

                    let mut keyword: Vec<char> = Vec::new();
                    loop {
                        let (x, _) = screen.cursor_pos()?;
                        let keyword_len = keyword.len();

                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Char('\n') => {
                                    filtered = true;
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    screen.flush()?;

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
                                }

                                Key::Right => {
                                    if x as usize == keyword_len + 4 {
                                        continue;
                                    }
                                    print!("{}", cursor::Right(1));
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
                                }

                                _ => continue,
                            }
                            screen.flush()?;
                        }
                    }
                    print!("{}", cursor::Hide);
                }

                Key::Char(':') => {
                    print!(" {}{}:", cursor::Goto(2, 2), clear::CurrentLine,);
                    print!("{}", cursor::Show);

                    let mut command: Vec<char> = Vec::new();
                    screen.flush()?;

                    'command: loop {
                        let eow = command.len() + 2;
                        let (x, _) = screen.cursor_pos()?;
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
                                    } else if command == vec!['c', 'd'] || command == vec!['z'] {
                                        p_memo_v = Vec::new();
                                        c_memo_v = Vec::new();
                                        state.current_dir = dirs::home_dir().unwrap();
                                        nums.reset();
                                        if let Err(e) = state.update_list() {
                                            print_warning(e, y);
                                            break 'command;
                                        }
                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        print!("{}", cursor::Hide);
                                        state.move_cursor(&nums, STARTING_POINT);
                                        break 'command;
                                    } else if command == vec!['e'] {
                                        nums.reset();
                                        state.update_list()?;
                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        print!("{}", cursor::Hide);
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

                                    if (c == "cd" || c == "z") && args.is_empty() {
                                        p_memo_v = Vec::new();
                                        c_memo_v = Vec::new();
                                        state.current_dir = dirs::home_dir().unwrap();
                                        nums.reset();
                                        state.update_list()?;
                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        print!("{}", cursor::Hide);
                                        state.move_cursor(&nums, STARTING_POINT);
                                        break 'command;
                                    }

                                    if c == "z" && args.len() == 1 {
                                        let output = std::process::Command::new("zoxide")
                                            .args(["query", args[0].trim()])
                                            .output()?;
                                        let output = output.stdout;
                                        if !output.is_empty() {
                                            let target_dir = std::str::from_utf8(&output);
                                            match target_dir {
                                                Err(e) => {
                                                    print!("{}", cursor::Hide);
                                                    print_warning(e, y);
                                                    break 'command;
                                                }
                                                Ok(target_dir) => {
                                                    let target_path =
                                                        PathBuf::from(target_dir.trim());
                                                    print_warning(target_path.to_str().unwrap(), y);
                                                    state.current_dir =
                                                        target_path.canonicalize()?;
                                                    nums.reset();
                                                    state.update_list()?;
                                                    p_memo_v = Vec::new();
                                                    c_memo_v = Vec::new();
                                                    clear_and_show(&state.current_dir);
                                                    state.list_up(nums.skip);
                                                    print!("{}", cursor::Hide);
                                                    state.move_cursor(&nums, STARTING_POINT);
                                                    break 'command;
                                                }
                                            }
                                        }
                                    }

                                    if c == "empty" && args.is_empty() {
                                        print_warning(WHEN_EMPTY, y);
                                        screen.flush()?;

                                        'empty: loop {
                                            let input = stdin.next();
                                            if let Some(Ok(key)) = input {
                                                match key {
                                                    Key::Char('y') | Key::Char('Y') => {
                                                        print_info("Processing...", y);
                                                        screen.flush()?;

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
                                            state.update_list()?;
                                            state.list_up(nums.skip);
                                            state.move_cursor(&nums, STARTING_POINT);
                                        } else {
                                            state.move_cursor(&nums, y);
                                        }
                                        break 'command;
                                    }

                                    print!("{}", screen::ToAlternateScreen);
                                    if std::env::set_current_dir(&state.current_dir).is_err() {
                                        print!("{}", screen::ToAlternateScreen);
                                        print!("{}", cursor::Hide,);
                                        print_warning("Cannot execute command", y);
                                        break 'command;
                                    }
                                    if std::process::Command::new(c).args(args).status().is_err() {
                                        print!("{}", screen::ToAlternateScreen);

                                        clear_and_show(&state.current_dir);
                                        state.update_list()?;
                                        state.list_up(nums.skip);

                                        print!("{}", cursor::Hide,);
                                        print_warning("Cannot execute command", y);
                                        break 'command;
                                    }
                                    print!("{}", screen::ToAlternateScreen);

                                    clear_and_show(&state.current_dir);
                                    state.update_list()?;
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
                                }

                                Key::Right => {
                                    if x as usize == eow + 1 {
                                        continue;
                                    };
                                    print!("{}", cursor::Right(1));
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
                                }

                                _ => continue,
                            }
                            screen.flush()?;
                        }
                    }
                }

                Key::Char('Z') => {
                    print!(" {}{}Z", cursor::Goto(2, 2), clear::CurrentLine,);
                    print!("{}", cursor::Show);

                    let mut command: Vec<char> = vec!['Z'];
                    screen.flush()?;

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
                // Show/hide hidden files or directories
                Key::Backspace => {
                    state.show_hidden = !state.show_hidden;
                    state.update_list()?;
                    clear_and_show(&state.current_dir);
                    state.list_up(0);
                    nums.reset();
                    state.move_cursor(&nums, STARTING_POINT);
                }
                _ => {
                    continue;
                }
            }
        }
        screen.flush()?;
    }
    //When exits, restore the cursor
    print!("{}", cursor::Restore);
    state.write_session(session_file_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoxide_test() {
        let output = std::process::Command::new("zoxide")
            .args(["query", "dotfiles"])
            .output()
            .unwrap();
        let stdout = std::str::from_utf8(&output.stdout).unwrap().trim();
        println!("{stdout}");
        let path = PathBuf::from(stdout);
        println!("{:?}", path.canonicalize());
    }
}

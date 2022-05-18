use super::config::make_config_if_not_exist;
use super::errors::FxError;
use super::functions::*;
use super::help::HELP;
use super::nums::*;
use super::state::*;
use crate::session::*;
use log::info;
use simplelog::{ConfigBuilder, LevelFilter, WriteLogger};
use std::ffi::OsStr;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{clear, cursor, screen};

/// frequency to detect terminal size change
const DETECTION_INTERVAL: u64 = 500;

/// Run the app.
pub fn run(arg: PathBuf, log: bool) -> Result<(), FxError> {
    //Prepare config file and trash directory path.
    let config_dir_path = {
        let mut path = dirs::config_dir().unwrap_or_else(|| panic!("Cannot read config dir."));
        path.push(FX_CONFIG_DIR);
        path
    };
    let config_file_path = config_dir_path.join(PathBuf::from(CONFIG_FILE));
    let trash_dir_path = config_dir_path.join(PathBuf::from(TRASH));

    if log {
        let mut log_name = chrono::Local::now().format("%F-%H-%M-%S").to_string();
        log_name.push_str(".log");
        let config = ConfigBuilder::new()
            .set_time_offset_to_local()
            .unwrap()
            .build();
        let log_name = config_dir_path.join(log_name);
        WriteLogger::init(
            LevelFilter::Info,
            config,
            std::fs::File::create(log_name).unwrap(),
        )
        .unwrap();
        info!("===START===");
    }

    //Make config file and trash directory if not exist.
    make_config_if_not_exist(&config_file_path, &trash_dir_path)
        .unwrap_or_else(|_| panic!("Cannot make config file or trash dir."));

    //If session file, which stores sortkey and whether to show hidden items, does not exist (i.e. first launch), make it.
    let session_file_path = config_dir_path.join(PathBuf::from(SESSION_FILE));
    if !session_file_path.exists() {
        make_session(&session_file_path).unwrap_or_else(|_| panic!("Cannot make session file."));
    }

    if !&arg.exists() {
        println!(
            "Invalid path or argument: {}\n`fx -h` shows help.",
            &arg.display()
        );
        return Ok(());
    }

    //Initialize app state
    let mut state = State::new()?;
    state.trash_dir = trash_dir_path;
    state.current_dir = arg.canonicalize()?;

    //filter mode or not
    let mut filtered = false;

    //Initialize num as Arc
    let nums = Num::new();
    let nums_run = Arc::new(Mutex::new(nums));
    let nums_detect = nums_run.clone();

    //Initialize screen as Arc
    let screen = screen::AlternateScreen::from(stdout().into_raw_mode().unwrap());
    let screen_run = Arc::new(Mutex::new(screen));
    let screen_detect = screen_run.clone();

    //Update list, print and flush
    print!("{}", cursor::Hide);
    state.update_list()?;
    clear_and_show(&state.current_dir);
    state.list_up(nums.skip);
    state.move_cursor(&nums, BEGINNING_ROW);
    let mut init_screen = screen_run.lock().unwrap();
    init_screen.flush()?;
    drop(init_screen);

    //Initialize cursor move memo
    let mut p_memo_v: Vec<ParentMemo> = Vec::new();
    let mut c_memo_v: Vec<ChildMemo> = Vec::new();

    //Prepare state as Arc
    let state_run = Arc::new(Mutex::new(state));
    let state_detect = state_run.clone();

    //Loop to detect terminal window size change
    let interval = Duration::from_millis(DETECTION_INTERVAL);
    thread::spawn(move || loop {
        thread::sleep(interval);
        let (mut column, row) = termion::terminal_size().unwrap();

        // Return error if terminal size may cause panic
        if column < 4 {
            log::error!("Too small terminal size.");
            panic!("Error: too small terminal size (less than 4 columns)");
        };
        if row < 4 {
            log::error!("Too small terminal size.");
            panic!("Error: too small terminal size (less than 4 columns)");
        };

        let mut state = state_detect.lock().unwrap();
        column = match state.layout.preview {
            true => column / 2,
            false => column,
        };
        let mut nums = nums_detect.lock().unwrap();
        if column != state.layout.terminal_column || row != state.layout.terminal_row {
            if state.layout.y < row {
                let cursor_pos = state.layout.y;
                state.refresh(column, row, &nums, cursor_pos);
            } else {
                let diff = state.layout.y + 1 - row;
                nums.index -= diff as usize;
                state.refresh(column, row, &nums, row - 1);
            }
            let mut screen = screen_detect.lock().unwrap();
            screen.flush().unwrap();
        }
    });

    let mut stdin = stdin().keys();

    'main: loop {
        let input = stdin.next();
        let mut state = state_run.lock().unwrap();
        let mut screen = screen_run.lock().unwrap();
        screen.flush()?;
        let mut nums = nums_run.lock().unwrap();
        let len = state.list.len();
        let y = state.layout.y;

        if let Some(Ok(key)) = input {
            match key {
                //Go up. If lists exceed max-row, lists "scrolls" before the top of the list
                Key::Char('j') | Key::Down => {
                    if len == 0 || nums.index == len - 1 {
                        continue;
                    } else if y >= state.layout.terminal_row - 4
                        && len > (state.layout.terminal_row - BEGINNING_ROW) as usize - 1
                    {
                        nums.go_down();
                        nums.inc_skip();
                        clear_and_show(&state.current_dir);
                        state.list_up(nums.skip);
                        state.move_cursor(&nums, y);
                    } else {
                        nums.go_down();
                        state.move_cursor(&nums, y + 1);
                    }
                }

                //Go down. If lists exceed max-row, lists "scrolls" before the bottom of the list
                Key::Char('k') | Key::Up => {
                    if nums.index == 0 {
                        continue;
                    } else if y <= BEGINNING_ROW + 3 && nums.skip != 0 {
                        nums.go_up();
                        nums.dec_skip();
                        clear_and_show(&state.current_dir);
                        state.list_up(nums.skip);
                        state.move_cursor(&nums, y);
                    } else {
                        nums.go_up();
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
                                        state.move_cursor(&nums, BEGINNING_ROW);
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
                    if len > (state.layout.terminal_row - BEGINNING_ROW) as usize {
                        nums.skip = (len as u16) + BEGINNING_ROW - state.layout.terminal_row;
                        nums.go_bottom(len - 1);
                        clear_and_show(&state.current_dir);
                        state.list_up(nums.skip);
                        let cursor_pos = state.layout.terminal_row - 1;
                        state.move_cursor(&nums, cursor_pos);
                    } else {
                        nums.reset();
                        nums.go_bottom(len - 1);
                        clear_and_show(&state.current_dir);
                        state.list_up(nums.skip);
                        state.move_cursor(&nums, len as u16 + BEGINNING_ROW - 1);
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
                                    screen.flush()?;
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
                                        screen.flush()?;
                                        continue;
                                    }
                                    Ok(_) => {
                                        let cursor_memo = if !filtered {
                                            ParentMemo {
                                                to_sym_dir: Some(state.current_dir.clone()),
                                                num: *nums,
                                                cursor_pos: y,
                                            }
                                        } else {
                                            ParentMemo {
                                                to_sym_dir: Some(state.current_dir.clone()),
                                                num: Num::new(),
                                                cursor_pos: BEGINNING_ROW,
                                            }
                                        };
                                        p_memo_v.push(cursor_memo);
                                        filtered = false;

                                        state.current_dir = true_path.clone();
                                        if let Err(e) =
                                            std::env::set_current_dir(&state.current_dir)
                                        {
                                            print_warning(e, y);
                                            screen.flush()?;
                                            continue;
                                        }
                                        state.update_list()?;
                                        clear_and_show(&state.current_dir);
                                        nums.reset();
                                        state.list_up(nums.skip);
                                        state.move_cursor(&nums, BEGINNING_ROW);
                                    }
                                },
                                None => {
                                    print!("{}", screen::ToAlternateScreen);
                                    if let Err(e) = state.open_file(nums.index) {
                                        print_warning(e, y);
                                        screen.flush()?;
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
                                        screen.flush()?;
                                        continue;
                                    }
                                    Ok(_) => {
                                        //store the last cursor position and skip number
                                        let cursor_memo = if !filtered {
                                            ParentMemo {
                                                to_sym_dir: None,
                                                num: *nums,
                                                cursor_pos: y,
                                            }
                                        } else {
                                            ParentMemo {
                                                to_sym_dir: None,
                                                num: Num::new(),
                                                cursor_pos: BEGINNING_ROW,
                                            }
                                        };
                                        p_memo_v.push(cursor_memo);
                                        filtered = false;

                                        state.current_dir = item.file_path.clone();
                                        if let Err(e) =
                                            std::env::set_current_dir(&state.current_dir)
                                        {
                                            print_warning(e, y);
                                            screen.flush()?;
                                            continue;
                                        }
                                        state.update_list()?;

                                        match c_memo_v.pop() {
                                            Some(memo) => {
                                                if state.current_dir == memo.dir_path {
                                                    nums.index = memo.num.index;
                                                    nums.skip = memo.num.skip;
                                                    clear_and_show(&state.current_dir);
                                                    state.list_up(nums.skip);
                                                    state.move_cursor(&nums, memo.cursor_pos);
                                                } else {
                                                    clear_and_show(&state.current_dir);
                                                    state.list_up(0);
                                                    nums.reset();
                                                    state.move_cursor(&nums, BEGINNING_ROW);
                                                }
                                            }
                                            None => {
                                                clear_and_show(&state.current_dir);
                                                state.list_up(0);
                                                nums.reset();
                                                state.move_cursor(&nums, BEGINNING_ROW);
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
                                    num: *nums,
                                    cursor_pos: y,
                                }
                            } else {
                                ChildMemo {
                                    dir_path: PathBuf::from(""),
                                    num: Num::new(),
                                    cursor_pos: BEGINNING_ROW,
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
                                        screen.flush()?;
                                        continue;
                                    }
                                    state.update_list()?;
                                    nums.index = memo.num.index;
                                    nums.skip = memo.num.skip;
                                    clear_and_show(&state.current_dir);
                                    state.list_up(nums.skip);
                                    state.move_cursor(&nums, memo.cursor_pos);
                                }
                                None => {
                                    state.current_dir = parent_p.to_path_buf();
                                    if let Err(e) = std::env::set_current_dir(&state.current_dir) {
                                        print_warning(e, y);
                                        screen.flush()?;
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
                                                >= (state.layout.terminal_row - (BEGINNING_ROW + 1))
                                                    .into()
                                            {
                                                nums.skip = (nums.index - 1) as u16;
                                                clear_and_show(&state.current_dir);
                                                state.list_up(nums.skip);
                                                state.move_cursor(&nums, BEGINNING_ROW + 1);
                                            } else {
                                                nums.skip = 0;
                                                clear_and_show(&state.current_dir);
                                                state.list_up(0);
                                                state.move_cursor(
                                                    &nums,
                                                    (nums.index as u16) + BEGINNING_ROW,
                                                );
                                            }
                                        }
                                        None => {
                                            nums.reset();
                                            clear_and_show(&state.current_dir);
                                            state.list_up(0);
                                            state.move_cursor(&nums, BEGINNING_ROW);
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

                //select mode
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
                                    if len == 0 || nums.index == len - 1 {
                                        continue;
                                    } else if y >= state.layout.terminal_row - 4
                                        && len
                                            > (state.layout.terminal_row - BEGINNING_ROW) as usize
                                                - 1
                                    {
                                        nums.go_down();
                                        nums.inc_skip();

                                        if nums.index > start_pos {
                                            let mut item = state.list.get_mut(nums.index).unwrap();
                                            item.selected = true;
                                        } else {
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
                                    }
                                }

                                Key::Char('k') | Key::Up => {
                                    if nums.index == 0 {
                                        continue;
                                    } else if y <= BEGINNING_ROW + 3 && nums.skip != 0 {
                                        nums.go_up();
                                        nums.dec_skip();

                                        if nums.index >= start_pos {
                                            let mut item =
                                                state.list.get_mut(nums.index + 1).unwrap();
                                            item.selected = false;
                                        } else {
                                            let mut item = state.list.get_mut(nums.index).unwrap();
                                            item.selected = true;
                                        }

                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        state.move_cursor(&nums, y);
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
                                                            cursor::Goto(1, BEGINNING_ROW),
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
                                    if len > (state.layout.terminal_row - BEGINNING_ROW) as usize {
                                        nums.skip = (len as u16) + BEGINNING_ROW
                                            - state.layout.terminal_row;
                                        nums.go_bottom(len - 1);
                                        state.select_to_bottom(start_pos);
                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        let cursor_pos = state.layout.terminal_row - 1;
                                        state.move_cursor(&nums, cursor_pos);
                                    } else {
                                        nums.go_bottom(len - 1);
                                        state.select_to_bottom(start_pos);
                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        print!(" ");
                                        state.move_cursor(&nums, len as u16 + BEGINNING_ROW - 1);
                                    }
                                }

                                Key::Char('d') => {
                                    print_info("Processing...", y);
                                    let start = Instant::now();
                                    screen.flush()?;

                                    state.registered.clear();
                                    let clone = state.list.clone();
                                    let selected: Vec<ItemInfo> =
                                        clone.into_iter().filter(|item| item.selected).collect();
                                    let total = selected.len();

                                    if let Err(e) = state.remove_and_yank(&selected, true) {
                                        print_warning(e, y);
                                        screen.flush()?;
                                        break;
                                    }

                                    clear_and_show(&state.current_dir);
                                    state.update_list()?;
                                    let new_len = state.list.len();
                                    if usize::from(nums.skip) >= new_len {
                                        nums.reset();
                                    }
                                    state.list_up(nums.skip);

                                    let duration = duration_to_string(start.elapsed());
                                    let delete_message: String = {
                                        if total == 1 {
                                            format!("1 item deleted [{}]", duration)
                                        } else {
                                            let mut count = total.to_string();
                                            count.push_str(&format!(
                                                " items deleted [{}]",
                                                duration
                                            ));
                                            count
                                        }
                                    };
                                    print_info(delete_message, y);
                                    print!(" ");

                                    if new_len == 0 {
                                        nums.reset();
                                        state.move_cursor(&nums, BEGINNING_ROW);
                                    } else if nums.index > new_len - 1 {
                                        let mut new_y = y - (nums.index - (new_len - 1)) as u16;
                                        if new_y < 3 {
                                            new_y = 3;
                                        }
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

                //toggle sortkey
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
                    state.move_cursor(&nums, BEGINNING_ROW);
                }

                //toggle whether to show preview of text file
                Key::Char('v') => {
                    state.layout.preview = !state.layout.preview;
                    if state.layout.preview {
                        let new_column = state.layout.terminal_column / 2;
                        let new_row = state.layout.terminal_row;
                        state.refresh(new_column, new_row, &nums, y);
                    } else {
                        let (new_column, new_row) = termion::terminal_size().unwrap();
                        state.refresh(new_column, new_row, &nums, y);
                    }
                }

                //delete
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

                                        let target = state.get_item(nums.index)?.clone();
                                        let target = vec![target];

                                        if let Err(e) = state.remove_and_yank(&target, true) {
                                            print_warning(e, y);
                                            screen.flush()?;
                                            continue;
                                        }

                                        clear_and_show(&state.current_dir);
                                        state.update_list()?;
                                        state.list_up(nums.skip);
                                        let cursor_pos = if state.list.is_empty() {
                                            BEGINNING_ROW
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

                //yank
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

                //put
                Key::Char('p') => {
                    if state.registered.is_empty() {
                        continue;
                    }
                    print_info("Processing...", y);
                    let start = Instant::now();
                    screen.flush()?;

                    let targets = state.registered.clone();
                    if let Err(e) = state.put_items(&targets, None) {
                        print_warning(e, y);
                        screen.flush()?;
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

                //rename
                Key::Char('c') => {
                    if len == 0 {
                        continue;
                    }
                    let item = state.get_item(nums.index)?.clone();
                    if !is_editable(&item.file_name) {
                        print_warning("Item name cannot be renamed due to character type.", y);
                        screen.flush()?;
                        continue;
                    }

                    print!("{}", cursor::Show);
                    let mut rename = item.file_name.chars().collect::<Vec<char>>();
                    print!(
                        "{}{}{} {}",
                        cursor::Goto(2, 2),
                        clear::CurrentLine,
                        RIGHT_ARROW,
                        &rename.iter().collect::<String>(),
                    );
                    screen.flush()?;

                    let initial_pos = 4;
                    loop {
                        let (x, _) = screen.cursor_pos()?;
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                //rename item
                                Key::Char('\n') => {
                                    let rename = rename.iter().collect::<String>();
                                    let mut to = state.current_dir.clone();
                                    to.push(rename);
                                    if let Err(e) = std::fs::rename(&item.file_path, &to) {
                                        print!("{}", cursor::Hide);
                                        print_warning(e, y);
                                        break;
                                    }

                                    let mut message = "RENAME: ".to_string();
                                    message.push_str(item.file_path.as_path().to_str().unwrap());
                                    message.push_str(" -> ");
                                    message.push_str(to.as_path().to_str().unwrap());
                                    info!("{}", message);
                                    state.branch_manip();
                                    state.manipulations.manip_list.push(ManipKind::Rename(
                                        RenamedFile {
                                            original_name: item.file_path.clone(),
                                            new_name: to,
                                        },
                                    ));
                                    state.manipulations.count = 0;

                                    print!("{}", cursor::Hide);
                                    clear_and_show(&state.current_dir);
                                    state.update_list()?;
                                    state.list_up(nums.skip);
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
                                    if x == initial_pos {
                                        continue;
                                    };
                                    print!("{}", cursor::Left(1));
                                }

                                Key::Right => {
                                    if x as usize == rename.len() + initial_pos as usize {
                                        continue;
                                    };
                                    print!("{}", cursor::Right(1));
                                }

                                Key::Char(c) => {
                                    rename.insert((x - initial_pos).into(), c);

                                    print!(
                                        "{}{}{} {}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        &rename.iter().collect::<String>(),
                                        cursor::Goto(x + 1, 2)
                                    );
                                }

                                Key::Backspace => {
                                    if x == initial_pos {
                                        continue;
                                    };
                                    rename.remove((x - initial_pos - 1).into());

                                    print!(
                                        "{}{}{} {}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        RIGHT_ARROW,
                                        &rename.iter().collect::<String>(),
                                        cursor::Goto(x - 1, 2)
                                    );
                                }

                                _ => continue,
                            }
                            screen.flush()?;
                        }
                    }
                }

                //filter mode
                Key::Char('/') => {
                    if len == 0 {
                        continue;
                    }
                    print!(" {}{}/", cursor::Goto(2, 2), clear::CurrentLine,);
                    print!("{}", cursor::Show);
                    screen.flush()?;

                    let original_list = state.list.clone();

                    let mut keyword: Vec<char> = Vec::new();
                    let initial_pos = 3;
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
                                    state.move_cursor(&nums, BEGINNING_ROW);
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
                                    if x == initial_pos {
                                        continue;
                                    }
                                    print!("{}", cursor::Left(1));
                                }

                                Key::Right => {
                                    if x as usize == keyword_len + initial_pos as usize {
                                        continue;
                                    }
                                    print!("{}", cursor::Right(1));
                                }

                                Key::Char(c) => {
                                    keyword.insert((x - initial_pos).into(), c);

                                    let result = &keyword.iter().collect::<String>();

                                    state.list = original_list
                                        .clone()
                                        .into_iter()
                                        .filter(|entry| entry.file_name.contains(result))
                                        .collect();

                                    clear_and_show(&state.current_dir);
                                    state.list_up(0);

                                    print!(
                                        "{}/{}{}",
                                        cursor::Goto(2, 2),
                                        result,
                                        cursor::Goto(x + 1, 2)
                                    );
                                }

                                Key::Backspace => {
                                    if x == initial_pos {
                                        continue;
                                    };
                                    keyword.remove((x - initial_pos - 1).into());

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
                                        "{}/{}{}",
                                        cursor::Goto(2, 2),
                                        &keyword.iter().collect::<String>(),
                                        cursor::Goto(x - 1, 2)
                                    );
                                }

                                _ => continue,
                            }
                            screen.flush()?;
                        }
                    }
                    print!("{}", cursor::Hide);
                }

                //shell mode
                Key::Char(':') => {
                    print!(" {}{}:", cursor::Goto(2, 2), clear::CurrentLine,);
                    print!("{}", cursor::Show);

                    let mut command: Vec<char> = Vec::new();
                    screen.flush()?;

                    let initial_pos = 3;
                    'command: loop {
                        let (x, _) = screen.cursor_pos()?;
                        let input = stdin.next();
                        if let Some(Ok(key)) = input {
                            match key {
                                Key::Esc => {
                                    print!("{}", clear::CurrentLine);
                                    print!("{}{}", cursor::Goto(2, 2), DOWN_ARROW);
                                    print!("{}", cursor::Hide);
                                    state.move_cursor(&nums, y);
                                    break 'command;
                                }

                                Key::Left => {
                                    if x == initial_pos {
                                        continue;
                                    };
                                    print!("{}", cursor::Left(1));
                                }

                                Key::Right => {
                                    if x as usize == command.len() + initial_pos as usize {
                                        continue;
                                    };
                                    print!("{}", cursor::Right(1));
                                }

                                Key::Backspace => {
                                    if x == initial_pos {
                                        continue;
                                    };
                                    command.remove((x - initial_pos - 1).into());

                                    print!(
                                        "{}{}:{}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        &command.iter().collect::<String>(),
                                        cursor::Goto(x - 1, 2)
                                    );
                                }

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
                                        state.move_cursor(&nums, BEGINNING_ROW);
                                        break 'command;
                                    } else if command == vec!['e'] {
                                        nums.reset();
                                        state.update_list()?;
                                        clear_and_show(&state.current_dir);
                                        state.list_up(nums.skip);
                                        print!("{}", cursor::Hide);
                                        state.move_cursor(&nums, BEGINNING_ROW);
                                        break 'command;
                                    } else if command == vec!['h'] {
                                        print!(
                                            "{}{}{}",
                                            cursor::Hide,
                                            clear::All,
                                            cursor::Goto(1, 1)
                                        );
                                        screen.flush()?;
                                        let help =
                                            format_txt(HELP, state.layout.terminal_column, true);
                                        let help_len = help.clone().len();
                                        print_help(&help, 0, state.layout.terminal_row);
                                        screen.flush()?;

                                        let mut skip = 0;
                                        loop {
                                            if let Some(Ok(key)) = stdin.next() {
                                                match key {
                                                    Key::Char('j') | Key::Down => {
                                                        if help_len
                                                            < state.layout.terminal_row.into()
                                                            || skip
                                                                == help_len + 1
                                                                    - state.layout.terminal_row
                                                                        as usize
                                                        {
                                                            continue;
                                                        } else {
                                                            print!("{}", clear::All);
                                                            skip += 1;
                                                            print_help(
                                                                &help,
                                                                skip,
                                                                state.layout.terminal_row,
                                                            );
                                                            screen.flush()?;
                                                            continue;
                                                        }
                                                    }
                                                    Key::Char('k') | Key::Up => {
                                                        if skip == 0 {
                                                            continue;
                                                        } else {
                                                            print!("{}", clear::All);
                                                            skip -= 1;
                                                            print_help(
                                                                &help,
                                                                skip,
                                                                state.layout.terminal_row,
                                                            );
                                                            screen.flush()?;
                                                            continue;
                                                        }
                                                    }
                                                    _ => {
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                        print!("{}", cursor::Hide);
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
                                        state.move_cursor(&nums, BEGINNING_ROW);
                                        break 'command;
                                    }

                                    if c == "z" && args.len() == 1 {
                                        if let Ok(output) = std::process::Command::new("zoxide")
                                            .args(["query", args[0].trim()])
                                            .output()
                                        {
                                            let output = output.stdout;
                                            if output.is_empty() {
                                                print_warning(
                                                    "Keyword cannot match the database.",
                                                    y,
                                                );
                                                break 'command;
                                            } else {
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
                                                        print_warning(
                                                            target_path.to_str().unwrap(),
                                                            y,
                                                        );
                                                        state.current_dir =
                                                            target_path.canonicalize()?;
                                                        nums.reset();
                                                        state.update_list()?;
                                                        p_memo_v = Vec::new();
                                                        c_memo_v = Vec::new();
                                                        clear_and_show(&state.current_dir);
                                                        state.list_up(nums.skip);
                                                        print!("{}", cursor::Hide);
                                                        state.move_cursor(&nums, BEGINNING_ROW);
                                                        break 'command;
                                                    }
                                                }
                                            }
                                        } else {
                                            print_warning("zoxide not installed?", y);
                                            break 'command;
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
                                                            screen.flush()?;
                                                            continue 'main;
                                                        }
                                                        if let Err(e) =
                                                            std::fs::create_dir(&state.trash_dir)
                                                        {
                                                            print!("{}", cursor::Hide);
                                                            print_warning(e, y);
                                                            screen.flush()?;
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
                                            state.move_cursor(&nums, BEGINNING_ROW);
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

                                Key::Char(c) => {
                                    command.insert((x - initial_pos).into(), c);

                                    print!(
                                        "{}{}:{}{}",
                                        clear::CurrentLine,
                                        cursor::Goto(2, 2),
                                        &command.iter().collect::<String>(),
                                        cursor::Goto(x + 1, 2)
                                    );
                                }

                                _ => continue,
                            }
                            screen.flush()?;
                        }
                    }
                }

                //undo
                Key::Char('u') => {
                    let mani_len = state.manipulations.manip_list.len();
                    if mani_len < state.manipulations.count + 1 {
                        continue;
                    }
                    if let Some(manipulation) = state
                        .manipulations
                        .manip_list
                        .get(mani_len - state.manipulations.count - 1)
                    {
                        match manipulation.clone() {
                            ManipKind::Rename(m) => {
                                if let Err(e) = std::fs::rename(&m.new_name, &m.original_name) {
                                    print_warning(e, y);
                                    screen.flush()?;
                                    continue;
                                }
                                state.manipulations.count += 1;
                                clear_and_show(&state.current_dir);
                                state.update_list()?;
                                state.list_up(nums.skip);
                                print_info("Undone [rename]", y);
                            }
                            ManipKind::Put(m) => {
                                for x in m.put {
                                    if let Err(e) = std::fs::remove_file(&x) {
                                        print_warning(e, y);
                                        screen.flush()?;
                                        continue;
                                    }
                                }
                                state.manipulations.count += 1;
                                clear_and_show(&state.current_dir);
                                state.update_list()?;
                                state.list_up(nums.skip);
                                print_info("Undone [put]", y);
                            }
                            ManipKind::Delete(m) => {
                                let targets = trash_to_info(&state.trash_dir, m.trash)?;
                                if let Err(e) = state.put_items(&targets, Some(m.dir)) {
                                    print_warning(e, y);
                                    screen.flush()?;
                                    continue;
                                }
                                state.manipulations.count += 1;
                                clear_and_show(&state.current_dir);
                                state.update_list()?;
                                state.list_up(nums.skip);
                                print_info("Undone [delete]", y);
                            }
                        }

                        let new_len = state.list.len();
                        if new_len == 0 {
                            nums.reset();
                            state.move_cursor(&nums, BEGINNING_ROW);
                        } else if nums.index > new_len - 1 {
                            let new_y = y - (nums.index - (new_len - 1)) as u16;
                            nums.index = new_len - 1;
                            state.move_cursor(&nums, new_y)
                        } else {
                            state.move_cursor(&nums, y);
                        }
                        screen.flush()?;
                    }
                }

                //redo
                Key::Ctrl('r') => {
                    let mani_len = state.manipulations.manip_list.len();
                    if mani_len == 0
                        || state.manipulations.count == 0
                        || mani_len < state.manipulations.count
                    {
                        continue;
                    }
                    if let Some(manipulation) = state
                        .manipulations
                        .manip_list
                        .get(mani_len - state.manipulations.count)
                    {
                        let manipulation = manipulation.clone();
                        match manipulation {
                            ManipKind::Rename(m) => {
                                if let Err(e) = std::fs::rename(&m.original_name, &m.new_name) {
                                    print_warning(e, y);
                                    screen.flush()?;
                                    continue;
                                }
                                state.manipulations.count -= 1;
                                clear_and_show(&state.current_dir);
                                state.update_list()?;
                                state.list_up(nums.skip);
                                print_info("Redone [rename]", y);
                            }
                            ManipKind::Put(m) => {
                                if let Err(e) = state.put_items(&m.original, Some(m.dir.clone())) {
                                    print_warning(e, y);
                                    screen.flush()?;
                                    continue;
                                }
                                state.manipulations.count -= 1;
                                clear_and_show(&state.current_dir);
                                state.update_list()?;
                                state.list_up(nums.skip);
                                print_info("Redone [put]", y);
                            }
                            ManipKind::Delete(m) => {
                                if let Err(e) = state.remove_and_yank(&m.original, false) {
                                    print_warning(e, y);
                                    screen.flush()?;
                                    continue;
                                }
                                state.manipulations.count -= 1;
                                clear_and_show(&state.current_dir);
                                state.update_list()?;
                                state.list_up(nums.skip);
                                print_info("Redone [delete]", y);
                            }
                        }

                        let new_len = state.list.len();
                        if new_len == 0 {
                            nums.reset();
                            state.move_cursor(&nums, BEGINNING_ROW);
                        } else if nums.index > new_len - 1 {
                            let new_y = y - (nums.index - (new_len - 1)) as u16;
                            nums.index = new_len - 1;
                            state.move_cursor(&nums, new_y)
                        } else {
                            state.move_cursor(&nums, y);
                        }
                        screen.flush()?;
                    }
                }

                //Debug print for undo/redo
                Key::Char('P') => {
                    if state.rust_log.is_some() {
                        print_info(format!("{:?}", state.manipulations), y);
                    }
                }

                //exit by ZZ
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
                    match state.show_hidden {
                        true => {
                            state.list.retain(|x| !x.is_hidden);
                            state.show_hidden = false;
                        }
                        false => {
                            state.show_hidden = true;
                            state.update_list()?;
                        }
                    }
                    nums.reset();
                    clear_and_show(&state.current_dir);
                    state.list_up(nums.skip);
                    state.move_cursor(&nums, BEGINNING_ROW);
                }
                _ => {
                    continue;
                }
            }
        }
        screen.flush()?;
    }

    let state = state_run.lock().unwrap();
    let mut screen = screen_run.lock().unwrap();

    //Save session, restore screen state and cursor
    state.write_session(session_file_path)?;
    write!(screen, "{}", screen::ToMainScreen)?;
    write!(screen, "{}", cursor::Restore)?;
    screen.flush()?;

    //Back to normal mode
    screen.suspend_raw_mode()?;
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

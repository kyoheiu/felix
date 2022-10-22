use super::config::make_config_if_not_exists;
use super::errors::FxError;
use super::functions::*;
use super::help::HELP;
use super::nums::*;
use super::op::*;
use super::session::*;
use super::state::*;
use super::term::*;

use crossterm::cursor::RestorePosition;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use log::{error, info};
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::io::{stdout, Write};
use std::panic;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Where the item list starts to scroll.
const SCROLL_POINT: u16 = 3;

/// Run the app.
pub fn run(arg: PathBuf, log: bool) -> Result<(), FxError> {
    enter_raw_mode();
    let result = panic::catch_unwind(|| _run(arg, log));
    leave_raw_mode();

    if let Err(panic) = result {
        clear_all();
        move_to(1, 1);
        match panic.downcast::<String>() {
            Ok(msg) => {
                println!("Panic: {}", msg);
            }
            Err(_) => {
                println!("Panic: unknown panic");
            }
        }
        return Err(FxError::Panic);
    }

    result.ok().unwrap()
}

pub fn _run(arg: PathBuf, log: bool) -> Result<(), FxError> {
    //Prepare config file and trash directory path.
    let config_dir_path = {
        let mut path = dirs::config_dir().unwrap_or_else(|| panic!("Cannot read config dir."));
        path.push(FX_CONFIG_DIR);
        path
    };
    let config_file_path = config_dir_path.join(PathBuf::from(CONFIG_FILE));
    let trash_dir_path = config_dir_path.join(PathBuf::from(TRASH));

    if log && init_log(&config_dir_path).is_err() {
        panic!("Cannot initialize log file.");
    }

    //Make config file and trash directory if not exist.
    make_config_if_not_exists(&config_file_path, &trash_dir_path)
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
    state.current_dir = if cfg!(not(windows)) {
        // If executed this on windows, "//?" will be inserted at the beginning of the path.
        arg.canonicalize()?
    } else {
        arg
    };
    let mut nums = Num::new();

    //Enter the alternate screen with crossterm
    let mut screen = stdout();
    execute!(screen, EnterAlternateScreen)?;

    //Update list, print and flush
    state.reload(&nums, BEGINNING_ROW)?;
    screen.flush()?;

    //Initialize cursor move memo
    let mut p_memo_v: Vec<ParentMemo> = Vec::new();
    let mut c_memo_v: Vec<ChildMemo> = Vec::new();

    'main: loop {
        screen.flush()?;
        let len = state.list.len();
        let y = state.layout.y;

        match event::read()? {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                //If you use kitty, you must clear the screen or the previewed image remains.
                if state.layout.is_kitty && state.layout.preview {
                    print!("\x1B[2J");
                    state.clear_and_show_headline();
                    state.list_up(nums.skip);
                    screen.flush()?;
                }
                match code {
                    //Go up. If lists exceed max-row, lists "scrolls" before the top of the list
                    KeyCode::Char('j') | KeyCode::Down => {
                        if len == 0 || nums.index == len - 1 {
                            continue;
                        } else if y >= state.layout.terminal_row - 1 - SCROLL_POINT
                            && len > (state.layout.terminal_row - BEGINNING_ROW) as usize - 1
                        {
                            nums.go_down();
                            nums.inc_skip();
                            state.redraw(&nums, y);
                        } else {
                            nums.go_down();
                            state.move_cursor(&nums, y + 1);
                        }
                    }

                    //Go down. If lists exceed max-row, lists "scrolls" before the bottom of the list
                    KeyCode::Char('k') | KeyCode::Up => {
                        if nums.index == 0 {
                            continue;
                        } else if y <= BEGINNING_ROW + SCROLL_POINT && nums.skip != 0 {
                            nums.go_up();
                            nums.dec_skip();
                            state.redraw(&nums, y);
                        } else {
                            nums.go_up();
                            state.move_cursor(&nums, y - 1);
                        }
                    }

                    //Go to top
                    KeyCode::Char('g') => {
                        to_info_bar();
                        clear_current_line();
                        print!("g");
                        show_cursor();
                        screen.flush()?;

                        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                            match code {
                                KeyCode::Char('g') => {
                                    hide_cursor();
                                    nums.reset();
                                    state.redraw(&nums, BEGINNING_ROW);
                                }

                                _ => {
                                    clear_current_line();
                                    hide_cursor();
                                    state.move_cursor(&nums, y);
                                }
                            }
                        }
                    }

                    //Go to bottom
                    KeyCode::Char('G') => {
                        if len == 0 {
                            continue;
                        }
                        if len > (state.layout.terminal_row - BEGINNING_ROW) as usize {
                            nums.skip = (len as u16) + BEGINNING_ROW - state.layout.terminal_row;
                            nums.go_bottom(len - 1);
                            let cursor_pos = state.layout.terminal_row - 1;
                            state.redraw(&nums, cursor_pos);
                        } else {
                            nums.go_bottom(len - 1);
                            state.move_cursor(&nums, len as u16 + BEGINNING_ROW - 1);
                        }
                    }

                    //Open file or change directory
                    KeyCode::Char('l') | KeyCode::Enter | KeyCode::Right => {
                        if let Ok(item) = state.get_item(nums.index) {
                            match item.file_type {
                                FileType::File => {
                                    execute!(screen, EnterAlternateScreen)?;
                                    if let Err(e) = state.open_file(item) {
                                        print_warning(e, y);
                                        continue;
                                    }
                                    execute!(screen, EnterAlternateScreen)?;
                                    hide_cursor();
                                    state.filtered = false;
                                    //Add thread sleep time after state.open_file().
                                    // This is necessary because, with tiling window managers, the window resizing is sometimes slow and felix reloads the layout so quickly that the display may become broken.
                                    //By the sleep (50ms for now and I think it's not easy to recognize this sleep), this will be avoided.
                                    std::thread::sleep(Duration::from_millis(50));
                                    state.reload(&nums, y)?;
                                    screen.flush()?;
                                }
                                FileType::Symlink => match &item.symlink_dir_path {
                                    Some(true_path) => match std::fs::File::open(true_path) {
                                        Err(e) => {
                                            print_warning(e, y);
                                            continue;
                                        }
                                        Ok(_) => {
                                            let cursor_memo = if !state.filtered {
                                                ParentMemo {
                                                    to_sym_dir: Some(state.current_dir.clone()),
                                                    num: nums,
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

                                            state.current_dir = true_path.clone();
                                            if let Err(e) =
                                                std::env::set_current_dir(&state.current_dir)
                                            {
                                                print_warning(e, y);
                                                continue;
                                            }
                                            state.filtered = false;
                                            nums.reset();
                                            state.reload(&nums, BEGINNING_ROW)?;
                                        }
                                    },
                                    None => {
                                        execute!(screen, EnterAlternateScreen)?;
                                        if let Err(e) = state.open_file(item) {
                                            print_warning(e, y);
                                            continue;
                                        }
                                        execute!(screen, EnterAlternateScreen)?;
                                        hide_cursor();
                                        state.filtered = false;
                                        state.redraw(&nums, y);
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
                                            let cursor_memo = if !state.filtered {
                                                ParentMemo {
                                                    to_sym_dir: None,
                                                    num: nums,
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
                                                        nums.index = memo.num.index;
                                                        nums.skip = memo.num.skip;
                                                        state.filtered = false;
                                                        state.redraw(&nums, memo.cursor_pos);
                                                    } else {
                                                        nums.reset();
                                                        state.filtered = false;
                                                        state.redraw(&nums, BEGINNING_ROW);
                                                    }
                                                }
                                                None => {
                                                    nums.reset();
                                                    state.filtered = false;
                                                    state.redraw(&nums, BEGINNING_ROW);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    //Open a file in a new window
                    //This works only if [exec] is set in config file
                    //and the extension of the item matches the key.
                    //If not, warning message appears.
                    KeyCode::Char('o') => {
                        if let Ok(item) = state.get_item(nums.index) {
                            match item.file_type {
                                FileType::File => {
                                    if let Err(e) = state.open_file_in_new_window(nums.index) {
                                        print_warning(e, y);
                                        continue;
                                    }
                                    hide_cursor();
                                    state.redraw(&nums, y);
                                    continue;
                                }
                                _ => {
                                    continue;
                                }
                            }
                        }
                    }

                    //Go to parent directory if exists.
                    //If the list is filtered, reload current directory.
                    KeyCode::Char('h') | KeyCode::Left => {
                        if state.filtered {
                            nums.reset();
                            state.filtered = false;
                            state.reload(&nums, BEGINNING_ROW)?;
                        }
                        let pre = state.current_dir.clone();

                        match state.current_dir.parent() {
                            Some(parent_p) => {
                                let cursor_memo = if !state.filtered {
                                    ChildMemo {
                                        dir_path: pre.clone(),
                                        num: nums,
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
                                        if let Err(e) =
                                            std::env::set_current_dir(&state.current_dir)
                                        {
                                            print_warning(e, y);
                                            continue;
                                        }
                                        nums.index = memo.num.index;
                                        nums.skip = memo.num.skip;
                                        state.filtered = false;
                                        state.reload(&nums, memo.cursor_pos)?;
                                    }
                                    None => {
                                        state.current_dir = parent_p.to_path_buf();
                                        if let Err(e) =
                                            std::env::set_current_dir(&state.current_dir)
                                        {
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
                                                        - (BEGINNING_ROW + 1))
                                                        .into()
                                                {
                                                    nums.skip = (nums.index - 1) as u16;
                                                    state.filtered = false;
                                                    state.redraw(&nums, BEGINNING_ROW + 1);
                                                } else {
                                                    nums.skip = 0;
                                                    state.filtered = false;
                                                    state.redraw(
                                                        &nums,
                                                        (nums.index as u16) + BEGINNING_ROW,
                                                    );
                                                }
                                            }
                                            None => {
                                                nums.reset();
                                                state.filtered = false;
                                                state.redraw(&nums, BEGINNING_ROW);
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

                    //Jumps to the directory that matches the keyword (zoxide required).
                    KeyCode::Char('z') => {
                        print!(" ");
                        to_info_bar();
                        clear_current_line();
                        print!("z");
                        show_cursor();

                        let mut command: Vec<char> = vec!['z'];
                        screen.flush()?;

                        let initial_pos = 2;
                        let mut current_pos = 3;
                        'zoxide: loop {
                            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                match code {
                                    KeyCode::Esc => {
                                        reset_info_line();
                                        hide_cursor();
                                        state.move_cursor(&nums, y);
                                        break 'zoxide;
                                    }

                                    KeyCode::Left => {
                                        if current_pos == initial_pos {
                                            continue;
                                        };
                                        current_pos -= 1;
                                        move_left(1);
                                    }

                                    KeyCode::Right => {
                                        if current_pos as usize
                                            == command.len() + initial_pos as usize
                                        {
                                            continue;
                                        };
                                        current_pos += 1;
                                        move_right(1);
                                    }

                                    KeyCode::Backspace => {
                                        if current_pos == initial_pos + 1 {
                                            reset_info_line();
                                            hide_cursor();
                                            state.move_cursor(&nums, y);
                                            break 'zoxide;
                                        };
                                        command.remove((current_pos - initial_pos - 1).into());
                                        current_pos -= 1;

                                        clear_current_line();
                                        to_info_bar();
                                        print!("{}", &command.iter().collect::<String>(),);
                                        move_to(current_pos, 2);
                                    }

                                    KeyCode::Enter => {
                                        let command: String = command.iter().collect();
                                        if command.trim() == "z" {
                                            //go to the home directory
                                            p_memo_v = Vec::new();
                                            c_memo_v = Vec::new();
                                            state.current_dir = dirs::home_dir().unwrap();
                                            nums.reset();
                                            if let Err(e) = state.update_list() {
                                                print_warning(e, y);
                                                break 'zoxide;
                                            }
                                            hide_cursor();
                                            state.redraw(&nums, BEGINNING_ROW);
                                            break 'zoxide;
                                        } else if command.len() > 2 {
                                            let (command, arg) = command.split_at(2);
                                            if command == "z " {
                                                if let Ok(output) =
                                                    std::process::Command::new("zoxide")
                                                        .args(["query", arg.trim()])
                                                        .output()
                                                {
                                                    let output = output.stdout;
                                                    if output.is_empty() {
                                                        print_warning(
                                                            "Keyword cannot match the database.",
                                                            y,
                                                        );
                                                        break 'zoxide;
                                                    } else {
                                                        let target_dir =
                                                            std::str::from_utf8(&output);
                                                        match target_dir {
                                                            Err(e) => {
                                                                hide_cursor();
                                                                print_warning(e, y);
                                                                break 'zoxide;
                                                            }
                                                            Ok(target_dir) => {
                                                                hide_cursor();
                                                                p_memo_v = Vec::new();
                                                                c_memo_v = Vec::new();
                                                                nums.reset();
                                                                let target_path = PathBuf::from(
                                                                    target_dir.trim(),
                                                                );
                                                                state.current_dir =
                                                                    if cfg!(not(windows)) {
                                                                        target_path
                                                                            .canonicalize()?
                                                                    } else {
                                                                        target_path
                                                                    };
                                                                state.filtered = false;
                                                                state
                                                                    .reload(&nums, BEGINNING_ROW)?;
                                                                break 'zoxide;
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    print_warning("zoxide not installed?", y);
                                                    break 'zoxide;
                                                }
                                            }
                                        } else {
                                            reset_info_line();
                                            hide_cursor();
                                            state.move_cursor(&nums, y);
                                            break 'zoxide;
                                        }
                                    }

                                    KeyCode::Char(c) => {
                                        command.insert((current_pos - initial_pos).into(), c);
                                        current_pos += 1;
                                        clear_current_line();
                                        to_info_bar();
                                        print!("{}", &command.iter().collect::<String>(),);
                                        move_to(current_pos, 2);
                                    }

                                    _ => continue,
                                }
                                screen.flush()?;
                            }
                        }
                    }

                    //select mode
                    KeyCode::Char('V') => {
                        if len == 0 {
                            continue;
                        }
                        let mut item = state.get_item_mut(nums.index)?;
                        item.selected = true;

                        state.redraw(&nums, y);
                        screen.flush()?;

                        let start_pos = nums.index;
                        let mut current_pos = y;

                        loop {
                            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                match code {
                                    KeyCode::Char('j') | KeyCode::Down => {
                                        if len == 0 || nums.index == len - 1 {
                                            continue;
                                        } else if current_pos >= state.layout.terminal_row - 4
                                            && len
                                                > (state.layout.terminal_row - BEGINNING_ROW)
                                                    as usize
                                                    - 1
                                        {
                                            nums.go_down();
                                            nums.inc_skip();

                                            if nums.index > start_pos {
                                                let mut item = state.get_item_mut(nums.index)?;
                                                item.selected = true;
                                            } else {
                                                let mut item =
                                                    state.get_item_mut(nums.index - 1)?;
                                                item.selected = false;
                                            }

                                            state.redraw(&nums, current_pos);
                                            screen.flush()?;
                                        } else {
                                            nums.go_down();
                                            current_pos += 1;

                                            if nums.index > start_pos {
                                                let mut item = state.get_item_mut(nums.index)?;
                                                item.selected = true;
                                            } else if nums.index <= start_pos {
                                                let mut item =
                                                    state.get_item_mut(nums.index - 1)?;
                                                item.selected = false;
                                            }

                                            state.redraw(&nums, current_pos);
                                        }
                                    }

                                    KeyCode::Char('k') | KeyCode::Up => {
                                        if nums.index == 0 {
                                            continue;
                                        } else if current_pos <= BEGINNING_ROW + 3 && nums.skip != 0
                                        {
                                            nums.go_up();
                                            nums.dec_skip();

                                            if nums.index >= start_pos {
                                                let mut item =
                                                    state.get_item_mut(nums.index + 1)?;
                                                item.selected = false;
                                            } else {
                                                let mut item = state.get_item_mut(nums.index)?;
                                                item.selected = true;
                                            }
                                            state.redraw(&nums, current_pos);
                                        } else {
                                            nums.go_up();
                                            current_pos -= 1;

                                            if nums.index >= start_pos {
                                                let mut item =
                                                    state.get_item_mut(nums.index + 1)?;
                                                item.selected = false;
                                            } else if nums.index < start_pos {
                                                let mut item = state.get_item_mut(nums.index)?;
                                                item.selected = true;
                                            }
                                            state.redraw(&nums, current_pos);
                                        }
                                    }

                                    KeyCode::Char('g') => {
                                        if nums.index == 0 {
                                            continue;
                                        } else {
                                            to_info_bar();
                                            clear_current_line();
                                            print!("g");
                                            show_cursor();
                                            screen.flush()?;

                                            if let Event::Key(KeyEvent { code, .. }) =
                                                event::read()?
                                            {
                                                match code {
                                                    KeyCode::Char('g') => {
                                                        hide_cursor();
                                                        nums.reset();
                                                        state.select_from_top(start_pos);
                                                        current_pos = BEGINNING_ROW;
                                                        state.redraw(&nums, current_pos);
                                                    }

                                                    _ => {
                                                        reset_info_line();
                                                        hide_cursor();
                                                        state.move_cursor(&nums, current_pos);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    KeyCode::Char('G') => {
                                        if len
                                            > (state.layout.terminal_row - BEGINNING_ROW) as usize
                                        {
                                            nums.skip = (len as u16) + BEGINNING_ROW
                                                - state.layout.terminal_row;
                                            nums.go_bottom(len - 1);
                                            state.select_to_bottom(start_pos);
                                            current_pos = state.layout.terminal_row - 1;
                                            state.redraw(&nums, current_pos);
                                        } else {
                                            nums.go_bottom(len - 1);
                                            state.select_to_bottom(start_pos);
                                            current_pos = len as u16 + BEGINNING_ROW - 1;
                                            state.redraw(&nums, current_pos);
                                        }
                                    }

                                    KeyCode::Char('d') => {
                                        print_info("DELETE: Processing...", current_pos);
                                        let start = Instant::now();
                                        screen.flush()?;

                                        state.registered.clear();
                                        let cloned = state.list.clone();
                                        let selected: Vec<ItemInfo> = cloned
                                            .into_iter()
                                            .filter(|item| item.selected)
                                            .collect();
                                        let total = selected.len();

                                        if let Err(e) = state.remove_and_yank(&selected, true) {
                                            print_warning(e, current_pos);
                                            screen.flush()?;
                                            break;
                                        }

                                        state.update_list()?;
                                        let new_len = state.list.len();
                                        if usize::from(nums.skip) >= new_len {
                                            nums.reset();
                                        }
                                        state.clear_and_show_headline();
                                        state.list_up(nums.skip);

                                        let duration = duration_to_string(start.elapsed());
                                        let delete_message: String = {
                                            if total == 1 {
                                                format!("1 item deleted [{}]", duration)
                                            } else {
                                                let mut count = total.to_string();
                                                let _ =
                                                    write!(count, " items deleted [{}]", duration);
                                                count
                                            }
                                        };
                                        print_info(delete_message, y);

                                        if new_len == 0 {
                                            nums.reset();
                                            state.move_cursor(&nums, BEGINNING_ROW);
                                        } else if nums.index > new_len - 1 {
                                            let mut new_y =
                                                current_pos - (nums.index - (new_len - 1)) as u16;
                                            if new_y < BEGINNING_ROW {
                                                new_y = BEGINNING_ROW;
                                            }
                                            nums.index = new_len - 1;
                                            state.move_cursor(&nums, new_y);
                                            screen.flush()?;
                                        } else {
                                            state.move_cursor(&nums, current_pos);
                                            screen.flush()?;
                                        }
                                        break;
                                    }

                                    KeyCode::Char('y') => {
                                        state.yank_item(nums.index, true);
                                        state.reset_selection();
                                        state.list_up(nums.skip);
                                        let mut yank_message: String =
                                            state.registered.len().to_string();
                                        yank_message.push_str(" items yanked");
                                        print_info(yank_message, current_pos);
                                        break;
                                    }

                                    KeyCode::Esc => {
                                        state.reset_selection();
                                        state.redraw(&nums, current_pos);
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
                    KeyCode::Char('t') => {
                        match state.sort_by {
                            SortKey::Name => {
                                state.sort_by = SortKey::Time;
                            }
                            SortKey::Time => {
                                state.sort_by = SortKey::Name;
                            }
                        }
                        nums.reset();
                        state.reload(&nums, BEGINNING_ROW)?;
                    }

                    //toggle whether to show preview of text file
                    KeyCode::Char('v') => {
                        state.layout.preview = !state.layout.preview;
                        if state.layout.preview {
                            let new_column = state.layout.terminal_column / 2;
                            let new_row = state.layout.terminal_row;
                            state.refresh(new_column, new_row, &nums, y);
                        } else {
                            let (new_column, new_row) = crossterm::terminal::size().unwrap();
                            state.refresh(new_column, new_row, &nums, y);
                        }
                    }

                    //delete
                    KeyCode::Char('d') => {
                        if len == 0 {
                            continue;
                        } else {
                            to_info_bar();
                            clear_current_line();
                            print!("d");
                            show_cursor();
                            screen.flush()?;

                            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                match code {
                                    KeyCode::Char('d') => {
                                        hide_cursor();
                                        print_info("DELETE: Processing...", y);
                                        screen.flush()?;
                                        let start = Instant::now();

                                        let target = state.get_item(nums.index)?.clone();
                                        let target = vec![target];

                                        if let Err(e) = state.remove_and_yank(&target, true) {
                                            print_warning(e, y);
                                            continue;
                                        }

                                        state.clear_and_show_headline();
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
                                    }
                                    _ => {
                                        reset_info_line();
                                        hide_cursor();
                                        state.move_cursor(&nums, y);
                                    }
                                }
                            }
                        }
                    }

                    //yank
                    KeyCode::Char('y') => {
                        if len == 0 {
                            continue;
                        }
                        to_info_bar();
                        clear_current_line();
                        print!("y");
                        show_cursor();
                        screen.flush()?;

                        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                            match code {
                                KeyCode::Char('y') => {
                                    state.yank_item(nums.index, false);
                                    reset_info_line();
                                    hide_cursor();
                                    print_info("1 item yanked", y);
                                }

                                _ => {
                                    reset_info_line();
                                    hide_cursor();
                                    state.move_cursor(&nums, y);
                                }
                            }
                        }
                    }

                    //put
                    KeyCode::Char('p') => {
                        if state.registered.is_empty() {
                            continue;
                        }
                        print_info("PUT: Processing...", y);
                        screen.flush()?;
                        let start = Instant::now();

                        let targets = state.registered.clone();
                        if let Err(e) = state.put_items(&targets, None) {
                            print_warning(e, y);
                            continue;
                        }

                        state.reload(&nums, y)?;

                        let duration = duration_to_string(start.elapsed());
                        let registered_len = state.registered.len();
                        let mut put_message: String = registered_len.to_string();
                        if registered_len == 1 {
                            let _ = write!(put_message, " item inserted [{}]", duration);
                        } else {
                            let _ = write!(put_message, " items inserted [{}]", duration);
                        }
                        print_info(put_message, y);
                    }

                    //rename
                    KeyCode::Char('c') => {
                        if len == 0 {
                            continue;
                        }
                        let item = state.get_item(nums.index)?.clone();
                        if !is_editable(&item.file_name) {
                            print_warning(
                                "Item name cannot be renamed due to the character type.",
                                y,
                            );
                            continue;
                        }

                        show_cursor();
                        let mut rename = item.file_name.chars().collect::<Vec<char>>();
                        to_info_bar();
                        clear_current_line();
                        print!("New name: {}", &rename.iter().collect::<String>(),);
                        screen.flush()?;

                        let initial_pos = 12;
                        let mut current_pos: u16 = 12 + item.file_name.len() as u16;
                        loop {
                            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                match code {
                                    //rename item
                                    KeyCode::Enter => {
                                        let rename = rename.iter().collect::<String>();
                                        let mut to = state.current_dir.clone();
                                        to.push(rename);
                                        if let Err(e) = std::fs::rename(&item.file_path, &to) {
                                            hide_cursor();
                                            print_warning(e, y);
                                            break;
                                        }

                                        state.operations.branch();
                                        state.operations.push(OpKind::Rename(RenamedFile {
                                            original_name: item.file_path.clone(),
                                            new_name: to,
                                        }));

                                        hide_cursor();
                                        state.reload(&nums, y)?;
                                        break;
                                    }

                                    KeyCode::Esc => {
                                        reset_info_line();
                                        hide_cursor();
                                        state.move_cursor(&nums, y);
                                        break;
                                    }

                                    KeyCode::Left => {
                                        if current_pos == initial_pos {
                                            continue;
                                        };
                                        current_pos -= 1;
                                        move_left(1);
                                    }

                                    KeyCode::Right => {
                                        if current_pos as usize
                                            == rename.len() + initial_pos as usize
                                        {
                                            continue;
                                        };
                                        current_pos += 1;
                                        move_right(1);
                                    }

                                    KeyCode::Char(c) => {
                                        rename.insert((current_pos - initial_pos).into(), c);
                                        current_pos += 1;

                                        to_info_bar();
                                        clear_current_line();
                                        print!("New name: {}", &rename.iter().collect::<String>(),);
                                        move_to(current_pos, 2);
                                    }

                                    KeyCode::Backspace => {
                                        if current_pos == initial_pos {
                                            continue;
                                        };
                                        rename.remove((current_pos - initial_pos - 1).into());
                                        current_pos -= 1;

                                        to_info_bar();
                                        clear_current_line();
                                        print!("New name: {}", &rename.iter().collect::<String>(),);
                                        move_to(current_pos, 2);
                                    }

                                    _ => continue,
                                }
                                screen.flush()?;
                            }
                        }
                    }

                    //filter mode
                    KeyCode::Char('/') => {
                        if len == 0 {
                            continue;
                        }
                        print!(" ");
                        to_info_bar();
                        clear_current_line();
                        print!("/");
                        show_cursor();
                        state.filtered = true;
                        screen.flush()?;

                        let original_list = state.list.clone();
                        let mut keyword: Vec<char> = Vec::new();
                        let initial_pos = 3;
                        let mut current_pos = initial_pos;
                        loop {
                            let keyword_len = keyword.len();
                            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                match code {
                                    KeyCode::Enter => {
                                        reset_info_line();
                                        nums.reset();
                                        state.move_cursor(&nums, BEGINNING_ROW);
                                        break;
                                    }

                                    KeyCode::Esc => {
                                        hide_cursor();
                                        state.filtered = false;
                                        state.list = original_list;
                                        state.reload(&nums, y)?;
                                        break;
                                    }

                                    KeyCode::Left => {
                                        if current_pos == initial_pos {
                                            continue;
                                        }
                                        current_pos -= 1;
                                        move_left(1);
                                    }

                                    KeyCode::Right => {
                                        if current_pos == keyword_len + initial_pos as usize {
                                            continue;
                                        }
                                        current_pos += 1;
                                        move_right(1);
                                    }

                                    KeyCode::Backspace => {
                                        if current_pos == initial_pos {
                                            hide_cursor();
                                            state.filtered = false;
                                            state.list = original_list;
                                            state.reload(&nums, y)?;
                                            break;
                                        } else {
                                            keyword.remove(current_pos - initial_pos - 1);
                                            current_pos -= 1;

                                            let result = &keyword.iter().collect::<String>();

                                            state.list = original_list
                                                .clone()
                                                .into_iter()
                                                .filter(|entry| entry.file_name.contains(result))
                                                .collect();

                                            state.clear_and_show_headline();
                                            state.list_up(0);

                                            to_info_bar();
                                            print!("/");
                                            print!("{}", result);
                                            move_to((current_pos).try_into().unwrap(), 2)
                                        }
                                    }

                                    KeyCode::Char(c) => {
                                        keyword.insert(current_pos - initial_pos, c);
                                        current_pos += 1;

                                        let result = &keyword.iter().collect::<String>();

                                        state.list = original_list
                                            .clone()
                                            .into_iter()
                                            .filter(|entry| entry.file_name.contains(result))
                                            .collect();

                                        state.clear_and_show_headline();
                                        state.list_up(0);

                                        to_info_bar();
                                        print!("/");
                                        print!("{}", result);
                                        move_to((current_pos).try_into().unwrap(), 2);
                                    }

                                    _ => continue,
                                }
                                screen.flush()?;
                            }
                        }
                        hide_cursor();
                    }

                    //shell mode
                    KeyCode::Char(':') => {
                        print!(" ");
                        to_info_bar();
                        clear_current_line();
                        print!(":");
                        show_cursor();
                        screen.flush()?;

                        let mut command: Vec<char> = Vec::new();

                        let initial_pos = 3;
                        let mut current_pos = 3;
                        'command: loop {
                            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                match code {
                                    KeyCode::Esc => {
                                        reset_info_line();
                                        hide_cursor();
                                        state.move_cursor(&nums, y);
                                        break 'command;
                                    }

                                    KeyCode::Left => {
                                        if current_pos == initial_pos {
                                            continue;
                                        };
                                        current_pos -= 1;
                                        move_left(1);
                                    }

                                    KeyCode::Right => {
                                        if current_pos as usize
                                            == command.len() + initial_pos as usize
                                        {
                                            continue;
                                        };
                                        current_pos += 1;
                                        move_right(1);
                                    }

                                    KeyCode::Backspace => {
                                        if current_pos == initial_pos {
                                            reset_info_line();
                                            hide_cursor();
                                            state.move_cursor(&nums, y);
                                            break 'command;
                                        } else {
                                            command.remove((current_pos - initial_pos - 1).into());
                                            current_pos -= 1;

                                            clear_current_line();
                                            to_info_bar();
                                            print!(":{}", &command.iter().collect::<String>());
                                            move_to(current_pos, 2);
                                        }
                                    }

                                    KeyCode::Enter => {
                                        hide_cursor();
                                        if command.is_empty() {
                                            reset_info_line();
                                            state.move_cursor(&nums, y);
                                            break;
                                        }

                                        if command == vec!['q'] {
                                            //quit
                                            break 'main;
                                        } else if command == vec!['c', 'd'] || command == vec!['z']
                                        {
                                            //go to the home directory
                                            p_memo_v = Vec::new();
                                            c_memo_v = Vec::new();
                                            state.current_dir = dirs::home_dir().unwrap();
                                            nums.reset();
                                            if let Err(e) = state.update_list() {
                                                print_warning(e, y);
                                                break 'command;
                                            }
                                            state.redraw(&nums, BEGINNING_ROW);
                                            break 'command;
                                        } else if command == vec!['e'] {
                                            //reload current dir
                                            nums.reset();
                                            state.filtered = false;
                                            state.reload(&nums, BEGINNING_ROW)?;
                                            break 'command;
                                        } else if command == vec!['h'] {
                                            //Show help
                                            clear_all();
                                            move_to(1, 1);
                                            screen.flush()?;
                                            let help = format_txt(
                                                HELP,
                                                state.layout.terminal_column,
                                                true,
                                            );
                                            let help_len = help.clone().len();
                                            print_help(&help, 0, state.layout.terminal_row);
                                            screen.flush()?;

                                            let mut skip = 0;
                                            loop {
                                                if let Event::Key(KeyEvent { code, .. }) =
                                                    event::read()?
                                                {
                                                    match code {
                                                        KeyCode::Char('j') | KeyCode::Down => {
                                                            if help_len
                                                                < state.layout.terminal_row.into()
                                                                || skip
                                                                    == help_len + 1
                                                                        - state.layout.terminal_row
                                                                            as usize
                                                            {
                                                                continue;
                                                            } else {
                                                                clear_all();
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
                                                        KeyCode::Char('k') | KeyCode::Up => {
                                                            if skip == 0 {
                                                                continue;
                                                            } else {
                                                                clear_all();
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
                                            state.redraw(&nums, y);
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
                                            //Change directory
                                            p_memo_v = Vec::new();
                                            c_memo_v = Vec::new();
                                            nums.reset();
                                            state.filtered = false;
                                            state.current_dir = dirs::home_dir().unwrap();
                                            state.reload(&nums, BEGINNING_ROW)?;
                                            break 'command;
                                        }

                                        if c == "z" && args.len() == 1 {
                                            //Change directory using zoxide
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
                                                            print_warning(e, y);
                                                            break 'command;
                                                        }
                                                        Ok(target_dir) => {
                                                            p_memo_v = Vec::new();
                                                            c_memo_v = Vec::new();
                                                            nums.reset();
                                                            let target_path =
                                                                PathBuf::from(target_dir.trim());
                                                            state.current_dir =
                                                                if cfg!(not(windows)) {
                                                                    target_path.canonicalize()?
                                                                } else {
                                                                    target_path
                                                                };
                                                            state.filtered = false;
                                                            state.reload(&nums, BEGINNING_ROW)?;
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
                                            //Empty the trash dir
                                            print_warning(WHEN_EMPTY, y);
                                            screen.flush()?;

                                            if let Event::Key(KeyEvent { code, .. }) =
                                                event::read()?
                                            {
                                                match code {
                                                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                                                        print_info("EMPTY: Processing...", y);
                                                        screen.flush()?;

                                                        if let Err(e) = std::fs::remove_dir_all(
                                                            &state.trash_dir,
                                                        ) {
                                                            print_warning(e, y);
                                                            screen.flush()?;
                                                            continue 'main;
                                                        }
                                                        if let Err(e) =
                                                            std::fs::create_dir(&state.trash_dir)
                                                        {
                                                            print_warning(e, y);
                                                            screen.flush()?;
                                                            continue 'main;
                                                        }
                                                        reset_info_line();
                                                        if state.current_dir == state.trash_dir {
                                                            state.reload(&nums, BEGINNING_ROW)?;
                                                            print_info("Trash dir emptied", y);
                                                        } else {
                                                            print_info("Trash dir emptied", y);
                                                            state.move_cursor(&nums, y);
                                                        }
                                                        screen.flush()?;
                                                        break 'command;
                                                    }
                                                    _ => {
                                                        reset_info_line();
                                                        state.move_cursor(&nums, y);
                                                        break 'command;
                                                    }
                                                }
                                            }
                                        }

                                        //Execute the command as it is
                                        execute!(screen, EnterAlternateScreen)?;
                                        if std::env::set_current_dir(&state.current_dir).is_err() {
                                            execute!(screen, EnterAlternateScreen)?;
                                            print_warning("Cannot execute command", y);
                                            break 'command;
                                        }
                                        if std::process::Command::new(c)
                                            .args(args.clone())
                                            .status()
                                            .is_err()
                                        {
                                            execute!(screen, EnterAlternateScreen)?;
                                            state.redraw(&nums, y);
                                            print_warning("Cannot execute command", y);
                                            break 'command;
                                        }
                                        execute!(screen, EnterAlternateScreen)?;
                                        info!("SHELL: {} {:?}", c, args);
                                        state.reload(&nums, y)?;
                                        break 'command;
                                    }

                                    KeyCode::Char(c) => {
                                        command.insert((current_pos - initial_pos).into(), c);
                                        current_pos += 1;
                                        clear_current_line();
                                        to_info_bar();
                                        print!(":{}", &command.iter().collect::<String>(),);
                                        move_to(current_pos, 2);
                                    }

                                    _ => continue,
                                }
                                screen.flush()?;
                            }
                        }
                    }

                    //undo
                    KeyCode::Char('u') => {
                        let op_len = state.operations.op_list.len();
                        if op_len < state.operations.pos + 1 {
                            print_info("No operations left.", y);
                            continue;
                        }
                        if let Some(op) = state
                            .operations
                            .op_list
                            .get(op_len - state.operations.pos - 1)
                        {
                            let op = op.clone();
                            if let Err(e) = state.undo(&nums, &op) {
                                print_warning(e, y);
                                continue;
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
                        }
                    }

                    //redo
                    KeyCode::Char('r') if modifiers == KeyModifiers::CONTROL => {
                        let op_len = state.operations.op_list.len();
                        if op_len == 0 || state.operations.pos == 0 || op_len < state.operations.pos
                        {
                            print_info("No operations left.", y);
                            continue;
                        }
                        if let Some(op) =
                            state.operations.op_list.get(op_len - state.operations.pos)
                        {
                            let op = op.clone();
                            if let Err(e) = state.redo(&nums, &op) {
                                print_warning(e, y);
                                continue;
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
                        }
                    }

                    //Debug print for undo/redo
                    KeyCode::Char('P') => {
                        if state.rust_log.is_some() {
                            print_info(format!("{:?}", state), y);
                        }
                    }

                    //exit by ZZ
                    KeyCode::Char('Z') => {
                        print!(" ");
                        to_info_bar();
                        clear_current_line();
                        print!("Z");
                        show_cursor();
                        screen.flush()?;

                        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                            match code {
                                KeyCode::Char('Z') => {
                                    break 'main;
                                }

                                _ => {
                                    reset_info_line();
                                    hide_cursor();
                                    state.move_cursor(&nums, y);
                                }
                            }
                        }
                    }
                    // Show/hide hidden files or directories
                    KeyCode::Backspace => {
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
                        state.redraw(&nums, BEGINNING_ROW);
                    }
                    _ => {
                        continue;
                    }
                }
            }
            Event::Resize(column, row) => {
                // Return error if terminal size may cause panic
                if column < 4 {
                    error!("Too small terminal size (less than 4 columns).");
                    panic!("Error: Too small terminal size (less than 4 columns). Please restart.");
                };
                if row < 4 {
                    error!("Too small terminal size (less than 4 rows).");
                    panic!("Error: Too small terminal size (less than 4 rows). Please restart.");
                };

                let column = match state.layout.preview {
                    true => column / 2,
                    false => column,
                };
                if column != state.layout.terminal_column || row != state.layout.terminal_row {
                    if state.layout.y < row {
                        let cursor_pos = state.layout.y;
                        state.refresh(column, row, &nums, cursor_pos);
                    } else {
                        let diff = state.layout.y + 1 - row;
                        nums.index -= diff as usize;
                        state.refresh(column, row, &nums, row - 1);
                    }
                }
            }
            _ => {}
        }
    }

    //Save session, restore screen state and cursor
    state.write_session(session_file_path)?;
    execute!(screen, LeaveAlternateScreen)?;
    write!(screen, "{}", RestorePosition)?;
    screen.flush()?;

    info!("===FINISH===");
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

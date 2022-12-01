use super::config::{make_config_if_not_exists, CONFIG_FILE};
use super::errors::FxError;
use super::functions::*;
use super::help::HELP;
use super::layout::Split;
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
use std::env::set_current_dir;
use std::fmt::Write as _;
use std::io::{stdout, Write};
use std::panic;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Where the item list starts to scroll.
const SCROLL_POINT: u16 = 3;

/// Launch the app. If initializing goes wrong, return error.
pub fn run(arg: PathBuf, log: bool) -> Result<(), FxError> {
    //Prepare config file and trash directory path.
    let config_dir_path = {
        let mut path = dirs::config_dir()
            .ok_or_else(|| FxError::Dirs("Cannot read config dir.".to_string()))?;
        path.push(FX_CONFIG_DIR);
        path
    };
    let config_file_path = config_dir_path.join(PathBuf::from(CONFIG_FILE));
    let trash_dir_path = config_dir_path.join(PathBuf::from(TRASH));

    if log {
        init_log(&config_dir_path)?;
    }

    //Make config file and trash directory if not exists.
    make_config_if_not_exists(&config_file_path, &trash_dir_path)?;

    //If session file, which stores sortkey and whether to show hidden items, does not exist (i.e. first launch), make it.
    let session_file_path = config_dir_path.join(PathBuf::from(SESSION_FILE));
    if !session_file_path.exists() {
        make_session(&session_file_path)?;
    }

    if !&arg.exists() {
        println!(
            "Invalid path or argument: {}\n`fx -h` shows help.",
            &arg.display()
        );
        return Ok(());
    }

    //Initialize app state
    let mut state = State::new(&config_file_path)?;
    state.trash_dir = trash_dir_path;
    state.current_dir = if cfg!(not(windows)) {
        // If executed this on windows, "//?" will be inserted at the beginning of the path.
        arg.canonicalize()?
    } else {
        arg
    };

    let result = panic::catch_unwind(|| _run(state, session_file_path));
    leave_raw_mode();

    if let Err(panic) = result {
        clear_all();
        move_to(1, 1);
        match panic.downcast::<String>() {
            Ok(msg) => {
                println!("Panic: {}", msg);
            }
            Err(e) => {
                println!("{:#?}", e);
            }
        }
        return Err(FxError::Panic);
    }

    result.ok().unwrap()
}

/// Run the app. (Containing the main loop)
fn _run(mut state: State, session_path: PathBuf) -> Result<(), FxError> {
    //Enter the alternate screen with crossterm
    let mut screen = stdout();
    enter_raw_mode();
    execute!(screen, EnterAlternateScreen)?;

    //If preview is on, refresh the layout.
    if state.layout.preview {
        state.update_list()?;
        let new_column = match state.layout.split {
            Split::Vertical => state.layout.terminal_column / 2,
            Split::Horizontal => state.layout.terminal_column,
        };
        let new_row = match state.layout.split {
            Split::Vertical => state.layout.terminal_row,
            Split::Horizontal => state.layout.terminal_row / 2,
        };
        state.refresh(new_column, new_row, BEGINNING_ROW)?;
    } else {
        state.reload(BEGINNING_ROW)?;
    }
    screen.flush()?;

    'main: loop {
        screen.flush()?;
        let len = state.list.len();

        match event::read()? {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                //If you use kitty, you must clear the screen or the previewed image remains.
                if state.layout.is_kitty && state.layout.preview {
                    print!("\x1B[2J");
                    state.clear_and_show_headline();
                    state.list_up();
                    screen.flush()?;
                }
                match code {
                    //Go up. If lists exceed max-row, lists "scrolls" before the top of the list
                    KeyCode::Char('j') | KeyCode::Down => {
                        if modifiers == KeyModifiers::ALT {
                            if state.layout.preview {
                                state.scroll_down_preview(state.layout.y);
                            }
                        } else if len == 0 || state.layout.nums.index == len - 1 {
                            continue;
                        } else if state.layout.y >= state.layout.terminal_row - 1 - SCROLL_POINT
                            && len > (state.layout.terminal_row - BEGINNING_ROW) as usize - 1
                        {
                            state.layout.nums.go_down();
                            state.layout.nums.inc_skip();
                            state.redraw(state.layout.y);
                        } else {
                            state.layout.nums.go_down();
                            state.move_cursor(state.layout.y + 1);
                        }
                    }

                    //Go down. If lists exceed max-row, lists "scrolls" before the bottom of the list
                    KeyCode::Char('k') | KeyCode::Up => {
                        if modifiers == KeyModifiers::ALT {
                            if state.layout.preview {
                                state.scroll_up_preview(state.layout.y);
                            }
                        } else if state.layout.nums.index == 0 {
                            continue;
                        } else if state.layout.y <= BEGINNING_ROW + SCROLL_POINT
                            && state.layout.nums.skip != 0
                        {
                            state.layout.nums.go_up();
                            state.layout.nums.dec_skip();
                            state.redraw(state.layout.y);
                        } else {
                            state.layout.nums.go_up();
                            state.move_cursor(state.layout.y - 1);
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
                                    state.layout.nums.reset();
                                    state.redraw(BEGINNING_ROW);
                                }

                                _ => {
                                    hide_cursor();
                                    clear_current_line();
                                    state.move_cursor(state.layout.y);
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
                            state.layout.nums.skip =
                                (len as u16) + BEGINNING_ROW - state.layout.terminal_row;
                            state.layout.nums.go_bottom(len - 1);
                            let cursor_pos = state.layout.terminal_row - 1;
                            state.redraw(cursor_pos);
                        } else {
                            state.layout.nums.go_bottom(len - 1);
                            state.move_cursor(len as u16 + BEGINNING_ROW - 1);
                        }
                    }

                    //Open file or change directory
                    KeyCode::Char('l') | KeyCode::Enter | KeyCode::Right => {
                        let mut dest: Option<PathBuf> = None;
                        if let Ok(item) = state.get_item() {
                            match item.file_type {
                                FileType::File => {
                                    execute!(screen, EnterAlternateScreen)?;
                                    if let Err(e) = state.open_file(item) {
                                        print_warning(e, state.layout.y);
                                        continue;
                                    }
                                    execute!(screen, EnterAlternateScreen)?;
                                    hide_cursor();
                                    //Add thread sleep time after state.open_file().
                                    // This is necessary because, with tiling window managers, the window resizing is sometimes slow and felix reloads the layout so quickly that the display may become broken.
                                    //By the sleep (50ms for now and I think it's not easy to recognize this sleep), this will be avoided.
                                    std::thread::sleep(Duration::from_millis(50));
                                    state.reload(state.layout.y)?;
                                    continue;
                                }
                                FileType::Symlink => match &item.symlink_dir_path {
                                    Some(true_path) => {
                                        if true_path.exists() {
                                            dest = Some(true_path.to_path_buf());
                                        } else {
                                            print_warning("Broken link.", state.layout.y);
                                            continue;
                                        }
                                    }
                                    None => {
                                        execute!(screen, EnterAlternateScreen)?;
                                        if let Err(e) = state.open_file(item) {
                                            print_warning(e, state.layout.y);
                                            continue;
                                        }
                                        execute!(screen, EnterAlternateScreen)?;
                                        hide_cursor();
                                        state.redraw(state.layout.y);
                                        continue;
                                    }
                                },
                                FileType::Directory => {
                                    if item.file_path.exists() {
                                        dest = Some(item.file_path.clone());
                                    } else {
                                        print_warning("Invalid directory.", state.layout.y);
                                        continue;
                                    }
                                }
                            }
                        }
                        if let Some(dest) = dest {
                            if let Err(e) = state.chdir(&dest, Move::Down) {
                                print_warning(e, state.layout.y);
                            }
                        }
                    }

                    //Open a file in a new window
                    //This works only if [exec] is set in config file
                    //and the extension of the item matches the key.
                    //If not, warning message appears.
                    KeyCode::Char('o') => {
                        if let Ok(item) = state.get_item() {
                            match item.file_type {
                                FileType::File => {
                                    if let Err(e) = state.open_file_in_new_window() {
                                        hide_cursor();
                                        state.redraw(state.layout.y);
                                        print_warning(e, state.layout.y);
                                        continue;
                                    }
                                    hide_cursor();
                                    state.redraw(state.layout.y);
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
                        let pre = state.current_dir.clone();

                        match pre.parent() {
                            Some(parent_p) => {
                                if let Err(e) = state.chdir(parent_p, Move::Up) {
                                    print_warning(e, state.layout.y);
                                }
                            }
                            None => {
                                continue;
                            }
                        }
                    }

                    //Unpack archive file. Fails if it is not an archive file or any of supported types.
                    KeyCode::Char('e') => {
                        print_info("Unpacking...", state.layout.y);
                        screen.flush()?;
                        let start = Instant::now();
                        if let Err(e) = state.unpack() {
                            state.reload(state.layout.y)?;
                            print_warning(e, state.layout.y);
                            continue;
                        }
                        let duration = duration_to_string(start.elapsed());
                        state.reload(state.layout.y)?;
                        print_info(format!("Unpacked. [{}]", duration), state.layout.y);
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
                                        go_to_and_rest_info();
                                        hide_cursor();
                                        state.move_cursor(state.layout.y);
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
                                            go_to_and_rest_info();
                                            hide_cursor();
                                            state.move_cursor(state.layout.y);
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
                                        hide_cursor();
                                        let command: String = command.iter().collect();
                                        if command.trim() == "z" {
                                            //go to the home directory
                                            let home_dir = dirs::home_dir().ok_or_else(|| {
                                                FxError::Dirs("Cannot read home dir.".to_string())
                                            })?;
                                            if let Err(e) = state.chdir(&home_dir, Move::Jump) {
                                                print_warning(e, state.layout.y);
                                            }
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
                                                            state.layout.y,
                                                        );
                                                        break 'zoxide;
                                                    } else {
                                                        let target_dir =
                                                            std::str::from_utf8(&output);
                                                        match target_dir {
                                                            Err(e) => {
                                                                print_warning(e, state.layout.y);
                                                                break 'zoxide;
                                                            }
                                                            Ok(target_dir) => {
                                                                hide_cursor();
                                                                state.layout.nums.reset();
                                                                let target_path = PathBuf::from(
                                                                    target_dir.trim(),
                                                                );
                                                                std::env::set_current_dir(
                                                                    &target_path,
                                                                )?;
                                                                state.current_dir =
                                                                    if cfg!(not(windows)) {
                                                                        target_path
                                                                            .canonicalize()?
                                                                    } else {
                                                                        target_path
                                                                    };
                                                                state.reload(BEGINNING_ROW)?;
                                                                break 'zoxide;
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    print_warning(
                                                        "zoxide not installed?",
                                                        state.layout.y,
                                                    );
                                                    break 'zoxide;
                                                }
                                            }
                                        } else {
                                            go_to_and_rest_info();
                                            hide_cursor();
                                            state.move_cursor(state.layout.y);
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
                        let mut item = state.get_item_mut()?;
                        item.selected = true;

                        state.redraw(state.layout.y);
                        screen.flush()?;

                        let start_pos = state.layout.nums.index;

                        loop {
                            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                match code {
                                    KeyCode::Char('j') | KeyCode::Down => {
                                        if len == 0 || state.layout.nums.index == len - 1 {
                                            continue;
                                        } else if state.layout.y >= state.layout.terminal_row - 4
                                            && len
                                                > (state.layout.terminal_row - BEGINNING_ROW)
                                                    as usize
                                                    - 1
                                        {
                                            if state.layout.nums.index >= start_pos {
                                                state.layout.nums.go_down();
                                                state.layout.nums.inc_skip();
                                                let mut item = state.get_item_mut()?;
                                                item.selected = true;
                                                state.redraw(state.layout.y);
                                            } else {
                                                let mut item = state.get_item_mut()?;
                                                item.selected = false;
                                                state.layout.nums.go_down();
                                                state.layout.nums.inc_skip();
                                                state.redraw(state.layout.y);
                                            }
                                        } else if state.layout.nums.index >= start_pos {
                                            state.layout.nums.go_down();
                                            let mut item = state.get_item_mut()?;
                                            item.selected = true;
                                            state.redraw(state.layout.y + 1);
                                        } else {
                                            let mut item = state.get_item_mut()?;
                                            item.selected = false;
                                            state.layout.nums.go_down();
                                            state.redraw(state.layout.y + 1);
                                        }
                                    }

                                    KeyCode::Char('k') | KeyCode::Up => {
                                        if state.layout.nums.index == 0 {
                                            continue;
                                        } else if state.layout.y <= BEGINNING_ROW + 3
                                            && state.layout.nums.skip != 0
                                        {
                                            if state.layout.nums.index > start_pos {
                                                let mut item = state.get_item_mut()?;
                                                item.selected = false;
                                                state.layout.nums.go_up();
                                                state.layout.nums.dec_skip();
                                                state.redraw(state.layout.y);
                                            } else {
                                                state.layout.nums.go_up();
                                                state.layout.nums.dec_skip();
                                                let mut item = state.get_item_mut()?;
                                                item.selected = true;
                                                state.redraw(state.layout.y);
                                            }
                                        } else if state.layout.nums.index > start_pos {
                                            let mut item = state.get_item_mut()?;
                                            item.selected = false;
                                            state.layout.nums.go_up();
                                            state.redraw(state.layout.y - 1);
                                        } else {
                                            state.layout.nums.go_up();
                                            let mut item = state.get_item_mut()?;
                                            item.selected = true;
                                            state.redraw(state.layout.y - 1);
                                        }
                                    }

                                    KeyCode::Char('g') => {
                                        if state.layout.nums.index == 0 {
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
                                                        state.select_from_top(start_pos);
                                                        state.layout.nums.reset();
                                                        state.redraw(BEGINNING_ROW);
                                                    }

                                                    _ => {
                                                        go_to_and_rest_info();
                                                        hide_cursor();
                                                        state.move_cursor(state.layout.y);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    KeyCode::Char('G') => {
                                        if len
                                            > (state.layout.terminal_row - BEGINNING_ROW) as usize
                                        {
                                            state.select_to_bottom(start_pos);
                                            state.layout.nums.skip = (len as u16) + BEGINNING_ROW
                                                - state.layout.terminal_row;
                                            state.layout.nums.go_bottom(len - 1);
                                            state.redraw(state.layout.terminal_row - 1);
                                        } else {
                                            state.select_to_bottom(start_pos);
                                            state.layout.nums.go_bottom(len - 1);
                                            state.redraw(len as u16 + BEGINNING_ROW - 1);
                                        }
                                    }

                                    KeyCode::Char('d') => {
                                        print_info("DELETE: Processing...", state.layout.y);
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
                                            print_warning(e, state.layout.y);
                                            break;
                                        }

                                        state.update_list()?;
                                        let new_len = state.list.len();
                                        if usize::from(state.layout.nums.skip) >= new_len {
                                            state.layout.nums.reset();
                                        }
                                        state.clear_and_show_headline();
                                        state.list_up();

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
                                        print_info(delete_message, state.layout.y);
                                        delete_cursor();

                                        if new_len == 0 {
                                            state.layout.nums.reset();
                                            state.move_cursor(BEGINNING_ROW);
                                        } else if state.layout.nums.index > new_len - 1 {
                                            let mut new_y = state.layout.y
                                                - (state.layout.nums.index - (new_len - 1)) as u16;
                                            if new_y < BEGINNING_ROW {
                                                new_y = BEGINNING_ROW;
                                            }
                                            state.layout.nums.index = new_len - 1;
                                            state.move_cursor(new_y);
                                            screen.flush()?;
                                        } else {
                                            state.move_cursor(state.layout.y);
                                            screen.flush()?;
                                        }
                                        break;
                                    }

                                    KeyCode::Char('y') => {
                                        state.yank_item(true);
                                        state.reset_selection();
                                        state.list_up();
                                        let mut yank_message: String =
                                            state.registered.len().to_string();
                                        yank_message.push_str(" items yanked");
                                        print_info(yank_message, state.layout.y);
                                        break;
                                    }

                                    KeyCode::Esc => {
                                        state.reset_selection();
                                        state.redraw(state.layout.y);
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
                        match state.layout.sort_by {
                            SortKey::Name => {
                                state.layout.sort_by = SortKey::Time;
                            }
                            SortKey::Time => {
                                state.layout.sort_by = SortKey::Name;
                            }
                        }
                        state.layout.nums.reset();
                        state.reorder(BEGINNING_ROW);
                    }

                    // Show/hide hidden files or directories
                    KeyCode::Backspace => {
                        match state.layout.show_hidden {
                            true => {
                                state.list.retain(|x| !x.is_hidden);
                                state.layout.show_hidden = false;
                            }
                            false => {
                                state.layout.show_hidden = true;
                                state.update_list()?;
                            }
                        }
                        state.layout.nums.reset();
                        state.redraw(BEGINNING_ROW);
                    }

                    //toggle whether to show preview of text file
                    KeyCode::Char('v') => {
                        state.layout.preview = !state.layout.preview;
                        if state.layout.preview {
                            match state.layout.split {
                                Split::Vertical => {
                                    let new_column = state.layout.terminal_column / 2;
                                    let new_row = state.layout.terminal_row;
                                    state.refresh(new_column, new_row, state.layout.y)?;
                                }
                                Split::Horizontal => {
                                    let new_row = state.layout.terminal_row / 2;
                                    let new_column = state.layout.terminal_column;
                                    state.refresh(new_column, new_row, state.layout.y)?;
                                }
                            }
                        } else {
                            let (new_column, new_row) = terminal_size()?;
                            state.refresh(new_column, new_row, state.layout.y)?;
                        }
                    }

                    //toggle vertical or horizontal split
                    KeyCode::Char('s') => match state.layout.split {
                        Split::Vertical => {
                            state.layout.split = Split::Horizontal;
                            if state.layout.preview {
                                let (new_column, mut new_row) = terminal_size()?;
                                new_row /= 2;
                                state.refresh(new_column, new_row, state.layout.y)?;
                            }
                        }
                        Split::Horizontal => {
                            state.layout.split = Split::Vertical;
                            if state.layout.preview {
                                let (mut new_column, new_row) = terminal_size()?;
                                new_column /= 2;
                                state.refresh(new_column, new_row, state.layout.y)?;
                            }
                        }
                    },

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
                                        print_info("DELETE: Processing...", state.layout.y);
                                        screen.flush()?;
                                        let start = Instant::now();

                                        let target = state.get_item()?.clone();
                                        let target = vec![target];

                                        if let Err(e) = state.remove_and_yank(&target, true) {
                                            print_warning(e, state.layout.y);
                                            continue;
                                        }

                                        state.clear_and_show_headline();
                                        state.update_list()?;
                                        state.list_up();
                                        state.layout.y = if state.list.is_empty() {
                                            BEGINNING_ROW
                                        } else if state.layout.nums.index == len - 1 {
                                            state.layout.nums.go_up();
                                            state.layout.y - 1
                                        } else {
                                            state.layout.y
                                        };
                                        let duration = duration_to_string(start.elapsed());
                                        print_info(
                                            format!("1 item deleted [{}]", duration),
                                            state.layout.y,
                                        );
                                        state.move_cursor(state.layout.y);
                                    }
                                    _ => {
                                        go_to_and_rest_info();
                                        hide_cursor();
                                        state.move_cursor(state.layout.y);
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
                                    state.yank_item(false);
                                    go_to_and_rest_info();
                                    hide_cursor();
                                    print_info("1 item yanked", state.layout.y);
                                }

                                _ => {
                                    go_to_and_rest_info();
                                    hide_cursor();
                                    state.move_cursor(state.layout.y);
                                }
                            }
                        }
                    }

                    //put
                    KeyCode::Char('p') => {
                        if state.registered.is_empty() {
                            continue;
                        }
                        print_info("PUT: Processing...", state.layout.y);
                        screen.flush()?;
                        let start = Instant::now();

                        let targets = state.registered.clone();
                        if let Err(e) = state.put_items(&targets, None) {
                            print_warning(e, state.layout.y);
                            continue;
                        }

                        state.reload(state.layout.y)?;

                        let duration = duration_to_string(start.elapsed());
                        let registered_len = state.registered.len();
                        let mut put_message: String = registered_len.to_string();
                        if registered_len == 1 {
                            let _ = write!(put_message, " item inserted [{}]", duration);
                        } else {
                            let _ = write!(put_message, " items inserted [{}]", duration);
                        }
                        print_info(put_message, state.layout.y);
                    }

                    //rename
                    KeyCode::Char('c') => {
                        if len == 0 {
                            continue;
                        }
                        let item = state.get_item()?.clone();
                        if !is_editable(&item.file_name) {
                            print_warning(
                                "Item name cannot be renamed due to the character type.",
                                state.layout.y,
                            );
                            continue;
                        }

                        show_cursor();
                        let mut rename = item.file_name.chars().collect::<Vec<char>>();
                        to_info_bar();
                        clear_current_line();
                        print!("New name: {}", &rename.iter().collect::<String>(),);
                        screen.flush()?;

                        // position after "New name: "
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
                                            print_warning(e, state.layout.y);
                                            break;
                                        }

                                        state.operations.branch();
                                        state.operations.push(OpKind::Rename(RenamedFile {
                                            original_name: item.file_path.clone(),
                                            new_name: to,
                                        }));

                                        hide_cursor();
                                        state.reload(state.layout.y)?;
                                        break;
                                    }

                                    KeyCode::Esc => {
                                        go_to_and_rest_info();
                                        hide_cursor();
                                        state.move_cursor(state.layout.y);
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

                    //search mode
                    KeyCode::Char('/') => {
                        if len == 0 {
                            continue;
                        }
                        print!(" ");
                        show_cursor();
                        to_info_bar();
                        clear_current_line();
                        print!("/");
                        screen.flush()?;

                        let original_nums = state.layout.nums;
                        let original_y = state.layout.y;
                        let mut keyword: Vec<char> = Vec::new();
                        // position after " /"
                        let initial_pos = 3;
                        let mut current_pos = initial_pos;
                        loop {
                            let keyword_len = keyword.len();
                            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                match code {
                                    KeyCode::Enter => {
                                        go_to_and_rest_info();
                                        state.keyword = Some(keyword.iter().collect());
                                        state.move_cursor(state.layout.y);
                                        break;
                                    }

                                    KeyCode::Esc => {
                                        hide_cursor();
                                        state.redraw(state.layout.y);
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
                                            state.redraw(state.layout.y);
                                            break;
                                        } else {
                                            keyword.remove(current_pos - initial_pos - 1);
                                            current_pos -= 1;

                                            let key = &keyword.iter().collect::<String>();

                                            let target = state
                                                .list
                                                .iter()
                                                .position(|x| x.file_name.contains(key));

                                            match target {
                                                Some(i) => {
                                                    state.layout.nums.skip = i as u16;
                                                    state.layout.nums.index = i;
                                                    state.highlight_matches(key);
                                                    state.redraw(BEGINNING_ROW);
                                                }
                                                None => {
                                                    state.highlight_matches(key);
                                                    state.layout.nums = original_nums;
                                                    state.layout.y = original_y;
                                                    state.redraw(state.layout.y);
                                                }
                                            }
                                            to_info_bar();
                                            clear_current_line();
                                            print!("/{}", key.clone());
                                            move_to(current_pos as u16, 2);
                                        }
                                    }

                                    KeyCode::Char(c) => {
                                        keyword.insert(current_pos - initial_pos, c);
                                        current_pos += 1;

                                        let key = &keyword.iter().collect::<String>();

                                        let target = state
                                            .list
                                            .iter()
                                            .position(|x| x.file_name.contains(key));

                                        match target {
                                            Some(i) => {
                                                state.layout.nums.skip = i as u16;
                                                state.layout.nums.index = i;
                                                state.highlight_matches(key);
                                                state.redraw(BEGINNING_ROW);
                                            }
                                            None => {
                                                state.highlight_matches(key);
                                                state.layout.nums = original_nums;
                                                state.layout.y = original_y;
                                                state.redraw(state.layout.y);
                                            }
                                        }

                                        to_info_bar();
                                        clear_current_line();
                                        print!("/{}", key.clone());
                                        move_to(current_pos as u16, 2);
                                    }

                                    _ => continue,
                                }
                                screen.flush()?;
                            }
                        }
                        hide_cursor();
                    }

                    KeyCode::Char('n') => match &state.keyword {
                        None => {
                            continue;
                        }
                        Some(keyword) => {
                            let next = state
                                .list
                                .iter()
                                .skip(state.layout.nums.index + 1)
                                .position(|x| x.file_name.contains(keyword));
                            match next {
                                None => {
                                    continue;
                                }
                                Some(i) => {
                                    let i = i + state.layout.nums.index + 1;
                                    state.layout.nums.skip = i as u16;
                                    state.layout.nums.index = i;
                                    state.redraw(BEGINNING_ROW);
                                }
                            }
                        }
                    },

                    KeyCode::Char('N') => match &state.keyword {
                        None => {
                            continue;
                        }
                        Some(keyword) => {
                            let previous = state
                                .list
                                .iter()
                                .take(state.layout.nums.index)
                                .rposition(|x| x.file_name.contains(keyword));
                            match previous {
                                None => {
                                    continue;
                                }
                                Some(i) => {
                                    state.layout.nums.skip = i as u16;
                                    state.layout.nums.index = i;
                                    state.redraw(BEGINNING_ROW);
                                }
                            }
                        }
                    },

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
                                        go_to_and_rest_info();
                                        hide_cursor();
                                        state.move_cursor(state.layout.y);
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
                                            go_to_and_rest_info();
                                            hide_cursor();
                                            state.move_cursor(state.layout.y);
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
                                            go_to_and_rest_info();
                                            state.move_cursor(state.layout.y);
                                            break;
                                        }

                                        if command == vec!['q'] {
                                            //quit
                                            break 'main;
                                        } else if command == vec!['c', 'd'] || command == vec!['z']
                                        {
                                            //go to the home directory
                                            let home_dir = dirs::home_dir().ok_or_else(|| {
                                                FxError::Dirs("Cannot read home dir.".to_string())
                                            })?;
                                            if let Err(e) = state.chdir(&home_dir, Move::Jump) {
                                                print_warning(e, state.layout.y);
                                            }
                                            break 'command;
                                        } else if command == vec!['e'] {
                                            //reload current dir
                                            state.keyword = None;
                                            state.layout.nums.reset();
                                            state.reload(BEGINNING_ROW)?;
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
                                            print_help(&help, 0, state.layout.terminal_row);
                                            screen.flush()?;

                                            let mut skip = 0;
                                            loop {
                                                if let Event::Key(KeyEvent { code, .. }) =
                                                    event::read()?
                                                {
                                                    match code {
                                                        KeyCode::Char('j') | KeyCode::Down => {
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
                                            state.redraw(state.layout.y);
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
                                            state.layout.nums.reset();
                                            let home_dir = dirs::home_dir().ok_or_else(|| {
                                                FxError::Dirs("Cannot read home dir.".to_string())
                                            })?;
                                            if let Err(e) = state.chdir(&home_dir, Move::Jump) {
                                                print_warning(e, state.layout.y);
                                            }
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
                                                        state.layout.y,
                                                    );
                                                    break 'command;
                                                } else {
                                                    let target_dir = std::str::from_utf8(&output);
                                                    match target_dir {
                                                        Err(e) => {
                                                            print_warning(e, state.layout.y);
                                                            break 'command;
                                                        }
                                                        Ok(target_dir) => {
                                                            state.layout.nums.reset();
                                                            let target_path =
                                                                PathBuf::from(target_dir.trim());
                                                            if let Err(e) =
                                                                set_current_dir(target_path.clone())
                                                            {
                                                                print_warning(e, state.layout.y);
                                                                break 'command;
                                                            }
                                                            if let Err(e) = state
                                                                .chdir(&target_path, Move::Jump)
                                                            {
                                                                print_warning(e, state.layout.y);
                                                            }
                                                            break 'command;
                                                        }
                                                    }
                                                }
                                            } else {
                                                print_warning(
                                                    "zoxide not installed?",
                                                    state.layout.y,
                                                );
                                                break 'command;
                                            }
                                        }

                                        if c == "empty" && args.is_empty() {
                                            //Empty the trash dir
                                            print_warning(EMPTY_WARNING, state.layout.y);
                                            screen.flush()?;

                                            if let Event::Key(KeyEvent { code, .. }) =
                                                event::read()?
                                            {
                                                match code {
                                                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                                                        print_info(
                                                            "EMPTY: Processing...",
                                                            state.layout.y,
                                                        );
                                                        screen.flush()?;

                                                        if let Err(e) = std::fs::remove_dir_all(
                                                            &state.trash_dir,
                                                        ) {
                                                            print_warning(e, state.layout.y);
                                                            continue 'main;
                                                        }
                                                        if let Err(e) =
                                                            std::fs::create_dir(&state.trash_dir)
                                                        {
                                                            print_warning(e, state.layout.y);
                                                            continue 'main;
                                                        }
                                                        go_to_and_rest_info();
                                                        if state.current_dir == state.trash_dir {
                                                            state.reload(BEGINNING_ROW)?;
                                                            print_info(
                                                                "Trash dir emptied",
                                                                state.layout.y,
                                                            );
                                                        } else {
                                                            print_info(
                                                                "Trash dir emptied",
                                                                state.layout.y,
                                                            );
                                                            state.move_cursor(state.layout.y);
                                                        }
                                                        screen.flush()?;
                                                        break 'command;
                                                    }
                                                    _ => {
                                                        go_to_and_rest_info();
                                                        state.move_cursor(state.layout.y);
                                                        break 'command;
                                                    }
                                                }
                                            }
                                        }

                                        //Execute the command as it is
                                        execute!(screen, EnterAlternateScreen)?;
                                        if std::env::set_current_dir(&state.current_dir).is_err() {
                                            execute!(screen, EnterAlternateScreen)?;
                                            print_warning("Cannot execute command", state.layout.y);
                                            break 'command;
                                        }
                                        if std::process::Command::new(c)
                                            .args(args.clone())
                                            .status()
                                            .is_err()
                                        {
                                            execute!(screen, EnterAlternateScreen)?;
                                            state.redraw(state.layout.y);
                                            print_warning("Cannot execute command", state.layout.y);
                                            break 'command;
                                        }
                                        execute!(screen, EnterAlternateScreen)?;
                                        hide_cursor();
                                        info!("SHELL: {} {:?}", c, args);
                                        state.reload(state.layout.y)?;
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
                            print_info("No operations left.", state.layout.y);
                            continue;
                        }
                        if let Some(op) = state
                            .operations
                            .op_list
                            .get(op_len - state.operations.pos - 1)
                        {
                            let op = op.clone();
                            if let Err(e) = state.undo(&op) {
                                print_warning(e, state.layout.y);
                                continue;
                            }

                            let new_len = state.list.len();
                            if new_len == 0 {
                                state.layout.nums.reset();
                                state.move_cursor(BEGINNING_ROW);
                            } else if state.layout.nums.index > new_len - 1 {
                                let new_y = state.layout.y
                                    - (state.layout.nums.index - (new_len - 1)) as u16;
                                state.layout.nums.index = new_len - 1;
                                state.move_cursor(new_y)
                            } else {
                                state.move_cursor(state.layout.y);
                            }
                        }
                    }

                    //redo
                    KeyCode::Char('r') if modifiers == KeyModifiers::CONTROL => {
                        let op_len = state.operations.op_list.len();
                        if op_len == 0 || state.operations.pos == 0 || op_len < state.operations.pos
                        {
                            print_info("No operations left.", state.layout.y);
                            continue;
                        }
                        if let Some(op) =
                            state.operations.op_list.get(op_len - state.operations.pos)
                        {
                            let op = op.clone();
                            if let Err(e) = state.redo(&op) {
                                print_warning(e, state.layout.y);
                                continue;
                            }

                            let new_len = state.list.len();
                            if new_len == 0 {
                                state.layout.nums.reset();
                                state.move_cursor(BEGINNING_ROW);
                            } else if state.layout.nums.index > new_len - 1 {
                                let new_y = state.layout.y
                                    - (state.layout.nums.index - (new_len - 1)) as u16;
                                state.layout.nums.index = new_len - 1;
                                state.move_cursor(new_y)
                            } else {
                                state.move_cursor(state.layout.y);
                            }
                        }
                    }

                    //Debug print for undo/redo
                    KeyCode::Char('P') => {
                        if state.rust_log.is_some() {
                            print_info(format!("{:?}", state), state.layout.y);
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
                                    go_to_and_rest_info();
                                    hide_cursor();
                                    state.move_cursor(state.layout.y);
                                }
                            }
                        }
                    }

                    //If input does not match any of the keys up to this point, ignore it
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

                if state.layout.preview {
                    let new_column = match state.layout.split {
                        Split::Vertical => column / 2,
                        Split::Horizontal => column,
                    };
                    let new_row = match state.layout.split {
                        Split::Vertical => row,
                        Split::Horizontal => row / 2,
                    };
                    let cursor_pos = if state.layout.y < new_row {
                        state.layout.y
                    } else {
                        let diff = state.layout.y + 1 - new_row;
                        state.layout.nums.index -= diff as usize;
                        new_row - 1
                    };

                    state.refresh(new_column, new_row, cursor_pos)?;
                } else {
                    let cursor_pos = if state.layout.y < row {
                        state.layout.y
                    } else {
                        let diff = state.layout.y + 1 - row;
                        state.layout.nums.index -= diff as usize;
                        row - 1
                    };
                    state.refresh(column, row, cursor_pos)?;
                }
            }
            _ => {}
        }
    }

    //Save session, restore screen state and cursor
    state.write_session(session_path)?;
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

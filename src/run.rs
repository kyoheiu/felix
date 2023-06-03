#![allow(unreachable_code)]
use super::config::FELIX;
use super::errors::FxError;
use super::functions::*;
use super::layout::{PreviewType, Split};
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
use std::time::Instant;

const TRASH: &str = "Trash";
const SESSION_FILE: &str = ".session";
/// Where the item list starts to scroll.
const SCROLL_POINT: u16 = 3;
const CLRSCR: &str = "\x1B[2J";
const INITIAL_POS_SEARCH: usize = 3;
const INITIAL_POS_SHELL: u16 = 3;

/// Launch the app. If initialization goes wrong, return error.
pub fn run(arg: PathBuf, log: bool) -> Result<(), FxError> {
    //Check if argument path is valid.
    if !&arg.exists() {
        println!();
        return Err(FxError::Arg(format!(
            "Invalid path: {}\n`fx -h` shows help.",
            &arg.display()
        )));
    } else if !&arg.is_dir() {
        return Err(FxError::Arg(
            "Path should be directory.\n`fx -h` shows help.".to_owned(),
        ));
    }

    //Prepare data local and trash dir path.
    let data_local_path = {
        let mut path = dirs::data_local_dir()
            .ok_or_else(|| FxError::Dirs("Cannot read the data local directory.".to_string()))?;
        path.push(FELIX);
        path
    };
    if !data_local_path.exists() {
        std::fs::create_dir_all(&data_local_path)?;
    }

    let trash_dir_path = {
        let mut path = data_local_path.clone();
        path.push(TRASH);
        path
    };
    if !trash_dir_path.exists() {
        std::fs::create_dir_all(&trash_dir_path)?;
    }

    //If `-l / --log` is set, initialize logger.
    if log {
        init_log(&data_local_path)?;
    }

    //Set the session file path.
    let session_path = {
        let mut path = data_local_path;
        path.push(SESSION_FILE);
        path
    };

    //Initialize app state. Inside State::new(), config file is read or created.
    let mut state = State::new(&session_path)?;
    state.trash_dir = trash_dir_path;
    state.current_dir = if cfg!(not(windows)) {
        // If executed this on windows, "//?" will be inserted at the beginning of the path.
        arg.canonicalize()?
    } else {
        arg
    };
    state.is_ro = match has_write_permission(&state.current_dir) {
        Ok(b) => !b,
        Err(_) => false,
    };

    //If the main function causes panic, catch it.
    let result = panic::catch_unwind(|| _run(state, session_path));
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
    if state.layout.is_preview() {
        state.update_list()?;
        let new_column = match state.layout.split {
            Split::Vertical => state.layout.terminal_column >> 1,
            Split::Horizontal => state.layout.terminal_column,
        };
        let new_row = match state.layout.split {
            Split::Vertical => state.layout.terminal_row,
            Split::Horizontal => state.layout.terminal_row >> 1,
        };
        state.refresh(new_column, new_row, BEGINNING_ROW)?;
    } else {
        state.reload(BEGINNING_ROW)?;
    }
    screen.flush()?;

    'main: loop {
        if state.is_out_of_bounds() {
            state.layout.nums.reset();
            state.redraw(BEGINNING_ROW);
        }
        screen.flush()?;
        let len = state.list.len();

        match event::read()? {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                match modifiers {
                    KeyModifiers::CONTROL => match code {
                        KeyCode::Char('r') => {
                            if state.v_start.is_some() {
                                continue;
                            }
                            let op_len = state.operations.op_list.len();
                            if op_len == 0
                                || state.operations.pos == 0
                                || op_len < state.operations.pos
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
                        //Other commands are disabled when Ctrl is pressed.
                        _ => {
                            continue;
                        }
                    },
                    KeyModifiers::ALT => match code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if state.layout.is_preview() {
                                state.scroll_down_preview(state.layout.y);
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if state.layout.is_preview() {
                                state.scroll_up_preview(state.layout.y);
                            }
                        }
                        //Other commands are disabled when Alt is pressed.
                        _ => {
                            continue;
                        }
                    },
                    KeyModifiers::NONE | KeyModifiers::SHIFT => {
                        match code {
                            //Debug print.
                            KeyCode::Char('P') => {
                                if std::env::var("RUST_LOG") == Ok("debug".to_string()) {
                                    println!("{:?}", state.registers);
                                }
                            }

                            KeyCode::Esc => {
                                //reset visual selection and return to normal mode
                                state.reset_selection();
                                state.redraw(state.layout.y);
                                continue;
                            }

                            //Go up. If lists exceed max-row, lists "scrolls" before the top of the list
                            KeyCode::Char('j') | KeyCode::Down => {
                                if let Some(start_pos) = state.v_start {
                                    //In visual mode
                                    if len == 0 || state.layout.nums.index == len - 1 {
                                        continue;
                                    } else if state.layout.y >= state.layout.terminal_row - 4
                                        && len
                                            > (state.layout.terminal_row - BEGINNING_ROW) as usize
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
                                } else {
                                    //normal mode
                                    if len == 0 || state.layout.nums.index == len - 1 {
                                        continue;
                                    } else if state.layout.y
                                        >= state.layout.terminal_row - 1 - SCROLL_POINT
                                        && len
                                            > (state.layout.terminal_row - BEGINNING_ROW) as usize
                                                - 1
                                    {
                                        state.layout.nums.go_down();
                                        state.layout.nums.inc_skip();
                                        state.redraw(state.layout.y);
                                    } else {
                                        state.layout.nums.go_down();
                                        state.move_cursor(state.layout.y + 1);
                                    }
                                }
                            }

                            //Go down. If lists exceed max-row, lists "scrolls" before the bottom of the list
                            KeyCode::Char('k') | KeyCode::Up => {
                                if let Some(start_pos) = state.v_start {
                                    //visual mode
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
                                } else {
                                    //normal mode
                                    if state.layout.nums.index == 0 {
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
                            }

                            //Go to top
                            KeyCode::Char('g') => {
                                if let Some(start_pos) = state.v_start {
                                    //visual mode
                                    if state.layout.nums.index == 0 {
                                        continue;
                                    } else {
                                        go_to_info_line_and_reset();
                                        print!("g");
                                        show_cursor();
                                        screen.flush()?;

                                        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                            match code {
                                                KeyCode::Char('g') => {
                                                    hide_cursor();
                                                    state.select_from_top(start_pos);
                                                    state.layout.nums.reset();
                                                    state.redraw(BEGINNING_ROW);
                                                }

                                                _ => {
                                                    go_to_info_line_and_reset();
                                                    hide_cursor();
                                                    state.move_cursor(state.layout.y);
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    //normal mode
                                    go_to_info_line_and_reset();
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
                            }

                            //Go to bottom
                            KeyCode::Char('G') => {
                                if let Some(start_pos) = state.v_start {
                                    //visual mode
                                    if len > (state.layout.terminal_row - BEGINNING_ROW) as usize {
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
                                } else {
                                    //normal mode
                                    if len == 0 {
                                        continue;
                                    }
                                    if len > (state.layout.terminal_row - BEGINNING_ROW) as usize {
                                        state.layout.nums.skip = (len as u16) + BEGINNING_ROW
                                            - state.layout.terminal_row;
                                        state.layout.nums.go_bottom(len - 1);
                                        let cursor_pos = state.layout.terminal_row - 1;
                                        state.redraw(cursor_pos);
                                    } else {
                                        state.layout.nums.go_bottom(len - 1);
                                        state.move_cursor(len as u16 + BEGINNING_ROW - 1);
                                    }
                                }
                            }

                            //Open file or change directory
                            KeyCode::Char('l') | KeyCode::Enter | KeyCode::Right => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
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
                            //This works only if i) [exec] is set in config file
                            //and ii) the extension of the item matches the key.
                            //If not, warning message appears.
                            KeyCode::Char('o') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
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

                            //Go to the parent directory if exists.
                            KeyCode::Char('h') | KeyCode::Left => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
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
                                //In visual mode, this is disabled.
                                //TODO! Enable this in visual mode.
                                if state.v_start.is_some() {
                                    continue;
                                }
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
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                delete_cursor();
                                go_to_info_line_and_reset();
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
                                                go_to_info_line_and_reset();
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
                                                    go_to_info_line_and_reset();
                                                    hide_cursor();
                                                    state.move_cursor(state.layout.y);
                                                    break 'zoxide;
                                                };
                                                command
                                                    .remove((current_pos - initial_pos - 1).into());
                                                current_pos -= 1;

                                                clear_current_line();
                                                to_info_line();
                                                print!("{}", &command.iter().collect::<String>(),);
                                                move_to(current_pos, 2);
                                            }

                                            KeyCode::Enter => {
                                                hide_cursor();
                                                let command = command.iter().collect::<String>();
                                                let commands = command
                                                    .split_whitespace()
                                                    .collect::<Vec<&str>>();
                                                if commands[0] == "z" {
                                                    if commands.len() > 2 {
                                                        //Invalid argument.
                                                        print_warning(
                                                            "Invalid argument for zoxide.",
                                                            state.layout.y,
                                                        );
                                                        state.move_cursor(state.layout.y);
                                                        break 'zoxide;
                                                    } else if commands.len() == 1 {
                                                        //go to the home directory
                                                        let home_dir = dirs::home_dir()
                                                            .ok_or_else(|| {
                                                                FxError::Dirs(
                                                                    "Cannot read home dir."
                                                                        .to_string(),
                                                                )
                                                            })?;
                                                        if let Err(e) =
                                                            state.chdir(&home_dir, Move::Jump)
                                                        {
                                                            print_warning(e, state.layout.y);
                                                        }
                                                        break 'zoxide;
                                                    } else if let Ok(output) =
                                                        std::process::Command::new("zoxide")
                                                            .args(["query", commands[1]])
                                                            .output()
                                                    {
                                                        let output = output.stdout;
                                                        if output.is_empty() {
                                                            print_warning(
                                                        "Keyword does not match the database.",
                                                        state.layout.y,
                                                    );
                                                            break 'zoxide;
                                                        } else {
                                                            let target_dir =
                                                                std::str::from_utf8(&output);
                                                            match target_dir {
                                                                Err(e) => {
                                                                    print_warning(
                                                                        e,
                                                                        state.layout.y,
                                                                    );
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
                                                //  else {
                                                //     go_to_and_rest_info();
                                                //     hide_cursor();
                                                //     state.move_cursor(state.layout.y);
                                                //     break 'zoxide;
                                                // }
                                            }

                                            KeyCode::Char(c) => {
                                                command
                                                    .insert((current_pos - initial_pos).into(), c);
                                                current_pos += 1;
                                                clear_current_line();
                                                to_info_line();
                                                print!("{}", &command.iter().collect::<String>(),);
                                                move_to(current_pos, 2);
                                            }

                                            _ => continue,
                                        }
                                        screen.flush()?;
                                    }
                                }
                            }

                            //switch to linewise visual mode
                            KeyCode::Char('V') => {
                                //If in visual mode, return to normal mode.
                                if state.v_start.is_some() {
                                    state.reset_selection();
                                    state.redraw(state.layout.y);
                                    continue;
                                }
                                if len == 0 {
                                    continue;
                                }
                                let mut item = state.get_item_mut()?;
                                item.selected = true;
                                state.redraw(state.layout.y);
                                state.v_start = Some(state.layout.nums.index);
                                continue;
                            }

                            //toggle sortkey
                            KeyCode::Char('t') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
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

                            //Show/hide hidden items.
                            KeyCode::Backspace => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
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

                            //Toggle whether to show preview.
                            KeyCode::Char('v') => {
                                if state.layout.is_preview() || state.layout.is_reg() {
                                    state.layout.reset_side();
                                } else {
                                    state.layout.show_preview();
                                }
                                let (new_column, new_row) = state.layout.update_column_and_row()?;
                                state.refresh(new_column, new_row, state.layout.y)?;
                            }

                            //Toggle vertical <-> horizontal split.
                            KeyCode::Char('s') => match state.layout.split {
                                Split::Vertical => {
                                    state.layout.split = Split::Horizontal;
                                    if state.layout.is_preview() || state.layout.is_reg() {
                                        let (new_column, mut new_row) = terminal_size()?;
                                        new_row /= 2;
                                        state.refresh(new_column, new_row, state.layout.y)?;
                                    }
                                }
                                Split::Horizontal => {
                                    state.layout.split = Split::Vertical;
                                    if state.layout.is_preview() || state.layout.is_reg() {
                                        let (mut new_column, new_row) = terminal_size()?;
                                        new_column /= 2;
                                        state.refresh(new_column, new_row, state.layout.y)?;
                                    }
                                }
                            },
                            //delete
                            KeyCode::Char('d') => {
                                //If read-only, deleting is disabled.
                                if state.is_ro {
                                    print_warning(
                                        "Cannot delete item in this directory.",
                                        state.layout.y,
                                    );
                                    continue;
                                }
                                if let Some(_start_pos) = state.v_start {
                                    //visual mode
                                    print_info("DELETE: Processing...", state.layout.y);
                                    let start = Instant::now();
                                    screen.flush()?;

                                    let selected: Vec<ItemBuffer> = state
                                        .list
                                        .iter()
                                        .filter(|item| item.selected)
                                        .map(ItemBuffer::new)
                                        .collect();
                                    let total = selected.len();

                                    if let Err(e) = state.remove_and_yank(&selected, true) {
                                        state.reset_selection();
                                        state.redraw(state.layout.y);
                                        print_warning(e, state.layout.y);
                                        break;
                                    }

                                    state.update_list()?;
                                    let new_len = state.list.len();
                                    state.clear_and_show_headline();

                                    let duration = duration_to_string(start.elapsed());
                                    let delete_message: String = {
                                        if total == 1 {
                                            format!("1 item deleted [{}]", duration)
                                        } else {
                                            let mut count = total.to_string();
                                            let _ = write!(count, " items deleted [{}]", duration);
                                            count
                                        }
                                    };
                                    print_info(delete_message, state.layout.y);
                                    delete_cursor();

                                    state.reset_selection();
                                    if new_len == 0 {
                                        state.layout.nums.reset();
                                        state.list_up();
                                        state.move_cursor(BEGINNING_ROW);
                                    } else if state.is_out_of_bounds() {
                                        if state.layout.nums.skip as usize >= new_len {
                                            state.layout.nums.skip = (new_len - 1) as u16;
                                            state.layout.nums.index = state.list.len() - 1;
                                            state.list_up();
                                            state.move_cursor(BEGINNING_ROW);
                                        } else {
                                            state.layout.nums.index = state.list.len() - 1;
                                            state.list_up();
                                            state.move_cursor(
                                                (state.list.len() as u16) - state.layout.nums.skip
                                                    + BEGINNING_ROW
                                                    - 1,
                                            );
                                        }
                                    } else {
                                        state.list_up();
                                        state.move_cursor(state.layout.y);
                                    }
                                } else {
                                    //normal mode
                                    if len == 0 {
                                        continue;
                                    } else {
                                        go_to_info_line_and_reset();
                                        print!("d");
                                        show_cursor();
                                        screen.flush()?;

                                        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                            match code {
                                                KeyCode::Char('d') => {
                                                    hide_cursor();
                                                    print_info(
                                                        "DELETE: Processing...",
                                                        state.layout.y,
                                                    );
                                                    screen.flush()?;
                                                    let start = Instant::now();

                                                    let target = state.get_item()?;
                                                    let target = vec![ItemBuffer::new(target)];

                                                    if let Err(e) =
                                                        state.remove_and_yank(&target, true)
                                                    {
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
                                                    let duration =
                                                        duration_to_string(start.elapsed());
                                                    print_info(
                                                        format!("1 item deleted. [{}]", duration),
                                                        state.layout.y,
                                                    );
                                                    state.move_cursor(state.layout.y);
                                                }
                                                _ => {
                                                    go_to_info_line_and_reset();
                                                    hide_cursor();
                                                    state.move_cursor(state.layout.y);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            //yank
                            KeyCode::Char('y') => {
                                if let Some(_start_pos) = state.v_start {
                                    //visual mode
                                    let items: Vec<ItemBuffer> = state
                                        .list
                                        .iter()
                                        .filter(|item| item.selected)
                                        .map(ItemBuffer::new)
                                        .collect();
                                    let item_len = state.yank_item(&items, None);
                                    state.reset_selection();
                                    state.list_up();
                                    let mut yank_message: String = item_len.to_string();
                                    yank_message.push_str(" items yanked");
                                    print_info(yank_message, state.layout.y);
                                } else {
                                    //normal mode
                                    if len == 0 {
                                        continue;
                                    }
                                    go_to_info_line_and_reset();
                                    print!("y");
                                    show_cursor();
                                    screen.flush()?;

                                    if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                        match code {
                                            KeyCode::Char('y') => {
                                                if let Ok(item) = state.get_item() {
                                                    state.yank_item(&[ItemBuffer::new(item)], None);
                                                    go_to_info_line_and_reset();
                                                    hide_cursor();
                                                    print_info("1 item yanked.", state.layout.y);
                                                }
                                            }

                                            _ => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                                state.move_cursor(state.layout.y);
                                            }
                                        }
                                    }
                                }
                            }

                            //put
                            KeyCode::Char('p') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                if let Err(e) =
                                    state.put(state.registers.unnamed.clone(), &mut screen)
                                {
                                    print_warning(e, state.layout.y);
                                }
                            }

                            //rename
                            KeyCode::Char('c') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                if len == 0 {
                                    continue;
                                }
                                let item = state.get_item()?.clone();
                                show_cursor();
                                let mut rename = item.file_name.chars().collect::<Vec<char>>();
                                to_info_line();
                                clear_current_line();
                                print!("New name: {}", &rename.iter().collect::<String>(),);
                                screen.flush()?;

                                let (mut current_pos, _) = cursor_pos()?;
                                let mut current_char_pos = rename.len();
                                loop {
                                    if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                        match code {
                                            //rename item
                                            KeyCode::Enter => {
                                                let rename = rename.iter().collect::<String>();
                                                let mut to = state.current_dir.clone();
                                                to.push(rename);
                                                if let Err(e) =
                                                    std::fs::rename(&item.file_path, &to)
                                                {
                                                    hide_cursor();
                                                    print_warning(e, state.layout.y);
                                                    break;
                                                }

                                                state.operations.branch();
                                                state.operations.push(OpKind::Rename(
                                                    RenamedFile {
                                                        original_name: item.file_path.clone(),
                                                        new_name: to,
                                                    },
                                                ));

                                                hide_cursor();
                                                state.reload(state.layout.y)?;
                                                break;
                                            }

                                            KeyCode::Esc => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                                state.move_cursor(state.layout.y);
                                                break;
                                            }

                                            KeyCode::Left => {
                                                if current_char_pos == 0 {
                                                    continue;
                                                };
                                                if let Some(to_be_skipped) =
                                                    unicode_width::UnicodeWidthChar::width(
                                                        rename[current_char_pos - 1],
                                                    )
                                                {
                                                    current_char_pos -= 1;
                                                    current_pos -= to_be_skipped as u16;
                                                    move_left(to_be_skipped as u16);
                                                }
                                            }

                                            KeyCode::Right => {
                                                if current_char_pos as usize == rename.len() {
                                                    continue;
                                                };
                                                if let Some(to_be_skipped) =
                                                    unicode_width::UnicodeWidthChar::width(
                                                        rename[current_char_pos],
                                                    )
                                                {
                                                    current_char_pos += 1;
                                                    current_pos += to_be_skipped as u16;
                                                    move_right(to_be_skipped as u16);
                                                }
                                            }

                                            KeyCode::Char(c) => {
                                                if let Some(to_be_added) =
                                                    unicode_width::UnicodeWidthChar::width(c)
                                                {
                                                    rename.insert((current_char_pos).into(), c);
                                                    current_char_pos += 1;
                                                    current_pos += to_be_added as u16;

                                                    go_to_info_line_and_reset();
                                                    print!(
                                                        "New name: {}",
                                                        &rename.iter().collect::<String>(),
                                                    );
                                                    move_to(current_pos + 1, 2);
                                                }
                                            }

                                            KeyCode::Backspace => {
                                                if current_char_pos == 0 {
                                                    continue;
                                                };
                                                let removed =
                                                    rename.remove((current_char_pos - 1).into());
                                                if let Some(to_be_removed) =
                                                    unicode_width::UnicodeWidthChar::width(removed)
                                                {
                                                    current_char_pos -= 1;
                                                    current_pos -= to_be_removed as u16;

                                                    go_to_info_line_and_reset();
                                                    print!(
                                                        "New name: {}",
                                                        &rename.iter().collect::<String>(),
                                                    );
                                                    move_to(current_pos + 1, 2);
                                                }
                                            }

                                            _ => continue,
                                        }
                                        screen.flush()?;
                                    }
                                }
                            }

                            //search mode
                            KeyCode::Char('/') => {
                                //In visual mode, this is disabled.
                                //TODO! Enable this in visual mode.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                if len == 0 {
                                    continue;
                                }
                                delete_cursor();
                                show_cursor();
                                go_to_info_line_and_reset();
                                print!("/");
                                screen.flush()?;

                                let original_nums = state.layout.nums;
                                let original_y = state.layout.y;
                                let mut keyword: Vec<char> = Vec::new();

                                let mut current_pos = INITIAL_POS_SEARCH;
                                loop {
                                    let keyword_len = keyword.len();
                                    if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                        match code {
                                            KeyCode::Enter => {
                                                go_to_info_line_and_reset();
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
                                                if current_pos == INITIAL_POS_SEARCH {
                                                    continue;
                                                }
                                                current_pos -= 1;
                                                move_left(1);
                                            }

                                            KeyCode::Right => {
                                                if current_pos == keyword_len + INITIAL_POS_SEARCH {
                                                    continue;
                                                }
                                                current_pos += 1;
                                                move_right(1);
                                            }

                                            KeyCode::Backspace => {
                                                if current_pos == INITIAL_POS_SEARCH {
                                                    hide_cursor();
                                                    state.redraw(state.layout.y);
                                                    break;
                                                } else {
                                                    keyword.remove(
                                                        current_pos - INITIAL_POS_SEARCH - 1,
                                                    );
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
                                                    go_to_info_line_and_reset();
                                                    print!("/{}", key.clone());
                                                    move_to(current_pos as u16, 2);
                                                }
                                            }

                                            KeyCode::Char(c) => {
                                                keyword.insert(current_pos - INITIAL_POS_SEARCH, c);
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

                                                go_to_info_line_and_reset();
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

                            //Search forward.
                            KeyCode::Char('n') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                match &state.keyword {
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
                                }
                            }

                            //Search backward.
                            KeyCode::Char('N') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                match &state.keyword {
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
                                }
                            }

                            //tinker with registers!
                            KeyCode::Char('"') => {
                                delete_cursor();
                                go_to_info_line_and_reset();
                                print!("\"");
                                show_cursor();
                                screen.flush()?;

                                let mut command: Vec<char> = Vec::new();

                                let mut current_pos = INITIAL_POS_SHELL;
                                'command: loop {
                                    if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                        match code {
                                            KeyCode::Esc => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                                state.move_cursor(state.layout.y);
                                                break 'command;
                                            }

                                            KeyCode::Left => {
                                                if current_pos == INITIAL_POS_SHELL {
                                                    continue;
                                                };
                                                current_pos -= 1;
                                                move_left(1);
                                            }

                                            KeyCode::Right => {
                                                if current_pos as usize
                                                    == command.len() + INITIAL_POS_SHELL as usize
                                                {
                                                    continue;
                                                };
                                                current_pos += 1;
                                                move_right(1);
                                            }

                                            KeyCode::Backspace => {
                                                if current_pos == INITIAL_POS_SHELL {
                                                    go_to_info_line_and_reset();
                                                    hide_cursor();
                                                    state.move_cursor(state.layout.y);
                                                    break 'command;
                                                } else {
                                                    command.remove(
                                                        (current_pos - INITIAL_POS_SHELL - 1)
                                                            .into(),
                                                    );
                                                    current_pos -= 1;

                                                    clear_current_line();
                                                    to_info_line();
                                                    print!(
                                                        "\"{}",
                                                        &command.iter().collect::<String>()
                                                    );
                                                    move_to(current_pos, 2);
                                                }
                                            }

                                            KeyCode::Char(c) => {
                                                command.insert(
                                                    (current_pos - INITIAL_POS_SHELL).into(),
                                                    c,
                                                );
                                                current_pos += 1;
                                                clear_current_line();
                                                to_info_line();
                                                print!("\"{}", &command.iter().collect::<String>(),);
                                                move_to(current_pos, 2);
                                            }

                                            KeyCode::Enter => {
                                                clear_current_line();
                                                hide_cursor();
                                                //check the length of the input and the char
                                                if command.len() > 3
                                                    || command.len() < 2
                                                    || !command[0].is_ascii_alphanumeric()
                                                {
                                                    print_warning(
                                                        "Input not supported.",
                                                        state.layout.y,
                                                    );
                                                    break 'command;
                                                }

                                                let action: String = command[1..].iter().collect();
                                                match action.as_str() {
                                                    "p" => {
                                                        let target = match command[0] {
                                                            '0' => Some(&state.registers.zero),
                                                            '1'..='9' => {
                                                                state.registers.numbered.get(
                                                                    command[0].to_digit(10).unwrap()
                                                                        as usize
                                                                        - 1,
                                                                )
                                                            }
                                                            'a'..='z' => state
                                                                .registers
                                                                .named
                                                                .get(&command[0]),
                                                            _ => None,
                                                        };

                                                        if let Some(target) = target {
                                                            let target = target.clone();
                                                            if let Err(e) =
                                                                state.put(target, &mut screen)
                                                            {
                                                                print_warning(e, state.layout.y);
                                                                break 'command;
                                                            }
                                                        } else {
                                                            print_warning(
                                                                "Register not found.",
                                                                state.layout.y,
                                                            );
                                                        }
                                                        break 'command;
                                                    }
                                                    _ => {
                                                        break 'command;
                                                    }
                                                }
                                            }

                                            _ => continue,
                                        }
                                        screen.flush()?;
                                    }
                                }
                            }

                            //shell mode
                            KeyCode::Char(':') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                delete_cursor();
                                go_to_info_line_and_reset();
                                print!(":");
                                show_cursor();
                                screen.flush()?;

                                let mut command: Vec<char> = Vec::new();

                                let mut current_pos = INITIAL_POS_SHELL;
                                'command: loop {
                                    if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                        match code {
                                            KeyCode::Esc => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                                state.move_cursor(state.layout.y);
                                                break 'command;
                                            }

                                            KeyCode::Left => {
                                                if current_pos == INITIAL_POS_SHELL {
                                                    continue;
                                                };
                                                current_pos -= 1;
                                                move_left(1);
                                            }

                                            KeyCode::Right => {
                                                if current_pos as usize
                                                    == command.len() + INITIAL_POS_SHELL as usize
                                                {
                                                    continue;
                                                };
                                                current_pos += 1;
                                                move_right(1);
                                            }

                                            KeyCode::Backspace => {
                                                if current_pos == INITIAL_POS_SHELL {
                                                    go_to_info_line_and_reset();
                                                    hide_cursor();
                                                    state.move_cursor(state.layout.y);
                                                    break 'command;
                                                } else {
                                                    command.remove(
                                                        (current_pos - INITIAL_POS_SHELL - 1)
                                                            .into(),
                                                    );
                                                    current_pos -= 1;

                                                    clear_current_line();
                                                    to_info_line();
                                                    print!(
                                                        ":{}",
                                                        &command.iter().collect::<String>()
                                                    );
                                                    move_to(current_pos, 2);
                                                }
                                            }

                                            KeyCode::Char(c) => {
                                                command.insert(
                                                    (current_pos - INITIAL_POS_SHELL).into(),
                                                    c,
                                                );
                                                current_pos += 1;
                                                clear_current_line();
                                                to_info_line();
                                                print!(":{}", &command.iter().collect::<String>(),);
                                                move_to(current_pos, 2);
                                            }

                                            KeyCode::Enter => {
                                                hide_cursor();
                                                //Set the command and argument(s).
                                                let commands: String = command.iter().collect();
                                                let commands: Vec<&str> =
                                                    commands.split_whitespace().collect();
                                                if commands.is_empty() {
                                                    go_to_info_line_and_reset();
                                                    state.move_cursor(state.layout.y);
                                                    break;
                                                }
                                                let command = commands[0];

                                                if commands.len() == 1 {
                                                    if command == "q" {
                                                        //quit
                                                        break 'main;
                                                    } else if command == "cd" || command == "z" {
                                                        //go to the home directory
                                                        let home_dir = dirs::home_dir()
                                                            .ok_or_else(|| {
                                                                FxError::Dirs(
                                                                    "Cannot read home dir."
                                                                        .to_string(),
                                                                )
                                                            })?;
                                                        if let Err(e) =
                                                            state.chdir(&home_dir, Move::Jump)
                                                        {
                                                            print_warning(e, state.layout.y);
                                                        }
                                                        break 'command;
                                                    } else if command == "e" {
                                                        //reload current dir
                                                        state.keyword = None;
                                                        state.layout.nums.reset();
                                                        state.reload(BEGINNING_ROW)?;
                                                        break 'command;
                                                    } else if command == "h" {
                                                        //show help
                                                        state.show_help(&screen)?;
                                                        state.redraw(state.layout.y);
                                                        break 'command;
                                                    } else if command == "reg" {
                                                        //:reg
                                                        if state.layout.is_preview() {
                                                            state.layout.show_reg();
                                                            state.redraw(state.layout.y);
                                                        } else if state.layout.is_reg() {
                                                            go_to_info_line_and_reset();
                                                            hide_cursor();
                                                            state.move_cursor(state.layout.y);
                                                        } else {
                                                            state.layout.show_reg();
                                                            let (new_column, new_row) = state
                                                                .layout
                                                                .update_column_and_row()?;
                                                            state.refresh(
                                                                new_column,
                                                                new_row,
                                                                state.layout.y,
                                                            )?;
                                                            go_to_info_line_and_reset();
                                                            hide_cursor();
                                                            state.move_cursor(state.layout.y);
                                                        }
                                                        break 'command;
                                                    } else if command == "trash" {
                                                        //move to trash dir
                                                        state.layout.nums.reset();
                                                        if let Err(e) = state.chdir(
                                                            &(state.trash_dir.clone()),
                                                            Move::Jump,
                                                        ) {
                                                            print_warning(e, state.layout.y);
                                                        }
                                                        break 'command;
                                                    } else if command == "empty" {
                                                        //empty the trash dir
                                                        state.empty_trash(&screen)?;
                                                        break 'command;
                                                    }
                                                }

                                                //zoxide jump
                                                if command == "z" && commands.len() == 2 {
                                                    //Change directory using zoxide
                                                    if let Ok(output) =
                                                        std::process::Command::new("zoxide")
                                                            .args(["query", commands[1].trim()])
                                                            .output()
                                                    {
                                                        let output = output.stdout;
                                                        if output.is_empty() {
                                                            print_warning(
                                                        "Keyword does not match the database.",
                                                        state.layout.y,
                                                    );
                                                            break 'command;
                                                        } else {
                                                            let target_dir =
                                                                std::str::from_utf8(&output);
                                                            match target_dir {
                                                                Err(e) => {
                                                                    print_warning(
                                                                        e,
                                                                        state.layout.y,
                                                                    );
                                                                    break 'command;
                                                                }
                                                                Ok(target_dir) => {
                                                                    state.layout.nums.reset();
                                                                    let target_path = PathBuf::from(
                                                                        target_dir.trim(),
                                                                    );
                                                                    if let Err(e) = set_current_dir(
                                                                        target_path.clone(),
                                                                    ) {
                                                                        print_warning(
                                                                            e,
                                                                            state.layout.y,
                                                                        );
                                                                        break 'command;
                                                                    }
                                                                    if let Err(e) = state.chdir(
                                                                        &target_path,
                                                                        Move::Jump,
                                                                    ) {
                                                                        print_warning(
                                                                            e,
                                                                            state.layout.y,
                                                                        );
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

                                                //Execute the command as it is
                                                execute!(screen, EnterAlternateScreen)?;
                                                if std::env::set_current_dir(&state.current_dir)
                                                    .is_err()
                                                {
                                                    execute!(screen, EnterAlternateScreen)?;
                                                    print_warning(
                                                        "Cannot execute command",
                                                        state.layout.y,
                                                    );
                                                    break 'command;
                                                }
                                                if std::process::Command::new(command)
                                                    .args(&commands[1..])
                                                    .status()
                                                    .is_err()
                                                {
                                                    execute!(screen, EnterAlternateScreen)?;
                                                    state.redraw(state.layout.y);
                                                    print_warning(
                                                        "Cannot execute command",
                                                        state.layout.y,
                                                    );
                                                    break 'command;
                                                }
                                                execute!(screen, EnterAlternateScreen)?;
                                                hide_cursor();
                                                info!("SHELL: {:?}", commands);
                                                state.reload(state.layout.y)?;
                                                break 'command;
                                            }

                                            _ => continue,
                                        }
                                        screen.flush()?;
                                    }
                                }
                            }

                            //undo
                            KeyCode::Char('u') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                let op_len = state.operations.op_list.len();
                                if op_len <= state.operations.pos {
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
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                let op_len = state.operations.op_list.len();
                                if op_len == 0
                                    || state.operations.pos == 0
                                    || op_len < state.operations.pos
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

                            //Add new temp file or directory.
                            //It has to feel like more "modal", so I comment this out for now.
                            // KeyCode::Char('a') => {
                            //     to_info_line();
                            //     clear_current_line();
                            //     print!("a");
                            //     show_cursor();
                            //     screen.flush()?;

                            //     if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                            //         match code {
                            //             //Add new file
                            //             KeyCode::Char('f') => {
                            //                 hide_cursor();
                            //                 match state.create_temp(false) {
                            //                     Err(e) => {
                            //                         print_warning(e, state.layout.y);
                            //                         continue;
                            //                     }
                            //                     Ok(p) => {
                            //                         state.reload(state.layout.y)?;
                            //                         print_info(
                            //                             format!("New file {} added.", p.display()),
                            //                             state.layout.y,
                            //                         );
                            //                     }
                            //                 }
                            //             }
                            //             //Add new directory
                            //             KeyCode::Char('d') => {
                            //                 hide_cursor();
                            //                 match state.create_temp(true) {
                            //                     Err(e) => {
                            //                         print_warning(e, state.layout.y);
                            //                         continue;
                            //                     }
                            //                     Ok(p) => {
                            //                         state.reload(state.layout.y)?;
                            //                         print_info(
                            //                             format!("New dir {} added.", p.display()),
                            //                             state.layout.y,
                            //                         );
                            //                     }
                            //                 }
                            //             }
                            //             _ => {
                            //                 go_to_and_rest_info();
                            //                 hide_cursor();
                            //                 state.move_cursor(state.layout.y);
                            //             }
                            //         }
                            //     }
                            // }

                            //exit by ZZ
                            KeyCode::Char('Z') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                delete_cursor();
                                go_to_info_line_and_reset();
                                print!("Z");
                                show_cursor();
                                screen.flush()?;

                                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                                    match code {
                                        KeyCode::Char('Z') => {
                                            break 'main;
                                        }

                                        _ => {
                                            go_to_info_line_and_reset();
                                            hide_cursor();
                                            state.move_cursor(state.layout.y);
                                        }
                                    }
                                }
                            }

                            //If input does not match any of the defined keys, ignore it.
                            _ => {
                                continue;
                            }
                        }
                    }
                    //Other modifiers disable commands when pressed.
                    _ => {
                        continue;
                    }
                }
                //If you use kitty, you must clear the screen by the escape sequence or the previewed image remains.
                if state.layout.is_kitty && state.layout.is_preview() {
                    if let Ok(item) = state.get_item() {
                        if item.preview_type == Some(PreviewType::Image) {
                            print!("{}", CLRSCR);
                            state.clear_and_show_headline();
                            state.list_up();
                            state.move_cursor(state.layout.y);
                            screen.flush()?;
                        }
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

                if state.layout.is_preview() || state.layout.is_reg() {
                    let new_column = match state.layout.split {
                        Split::Vertical => column >> 1,
                        Split::Horizontal => column,
                    };
                    let new_row = match state.layout.split {
                        Split::Vertical => row,
                        Split::Horizontal => row >> 1,
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
            //Other events are disabled.
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

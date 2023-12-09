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
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use log::{error, info};
use std::env;
use std::io::{stdout, Write};
use std::panic;
use std::path::PathBuf;
use std::time::Instant;

const TRASH: &str = "Trash";
const SESSION_FILE: &str = ".session";
/// Where the item list starts to scroll.
const SCROLL_POINT: u16 = 3;
const CLRSCR: &str = "\x1B[2J";
const INITIAL_POS_COMMAND_LINE: u16 = 3;
const INITIAL_POS_Z: u16 = 2;

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

    let shell_pid: Option<String> = env::var("SHELL_PID").ok();

    //Prepare config and data local path.
    let config_dir_path = {
        let mut path = dirs::config_dir()
            .ok_or_else(|| FxError::Dirs("Cannot read the config directory.".to_string()))?;
        path.push(FELIX);
        path
    };
    //Prepare data local and trash dir path.
    let data_local_path = {
        let mut path = dirs::data_local_dir()
            .ok_or_else(|| FxError::Dirs("Cannot read the data local directory.".to_string()))?;
        path.push(FELIX);
        path
    };
    let runtime_path = {
        let mut path = {
            #[cfg(not(target_os = "macos"))]
            let path = dirs::runtime_dir()
                .or_else(|| Some(env::temp_dir()))
                .ok_or_else(|| FxError::Dirs("Cannot read the runtime directory.".to_string()))?;

            #[cfg(target_os = "macos")]
            let path = env::temp_dir();

            path
        };
        path.push(FELIX);
        path
    };
    if !config_dir_path.exists() {
        std::fs::create_dir_all(&config_dir_path)?;
    }
    if !data_local_path.exists() {
        std::fs::create_dir_all(&data_local_path)?;
    }
    if !runtime_path.exists() {
        std::fs::create_dir_all(&runtime_path)?;
    }

    //Path of the file used to store lwd (Last Working Directory) at the end of the session.
    let lwd_file_path = shell_pid.map(|basename| runtime_path.join(basename));

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
    state.lwd_file = lwd_file_path;
    state.current_dir = if cfg!(not(windows)) {
        // If executed this on windows, "//?" will be inserted at the beginning of the path.
        arg.canonicalize()?
    } else {
        arg
    };
    state.jumplist.add(&state.current_dir);
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
                code,
                modifiers,
                // Explicitly ignore the key release events for Windows.
                kind: KeyEventKind::Press,
                ..
            }) => {
                match modifiers {
                    KeyModifiers::CONTROL => match code {
                        //redo
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

                        // jump backward
                        KeyCode::Char('o') => {
                            if let Some(path_to_jump_to) = state.jumplist.get_backward() {
                                if path_to_jump_to.exists() {
                                    state.chdir(&path_to_jump_to, Move::List)?;
                                    state.jumplist.pos_backward();
                                } else {
                                    print_warning(
                                        "Directory backward not found: Removed from jumplist.",
                                        state.layout.y,
                                    );
                                    state.jumplist.remove_backward();
                                }
                            }
                        }

                        //Other commands are disabled when Ctrl is pressed,
                        //except <C-i> (equivalent to Tab).
                        _ => {
                            continue;
                        }
                    },
                    KeyModifiers::ALT => match code {
                        //scroll down the previewed text
                        KeyCode::Char('j') | KeyCode::Down => {
                            if state.layout.is_preview() {
                                state.scroll_down_preview(state.layout.y);
                            }
                        }
                        //scroll up the previewed text
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
                            //Reset visual selection and return to normal mode
                            KeyCode::Esc => {
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
                                            let item = state.get_item_mut()?;
                                            item.selected = true;
                                            state.redraw(state.layout.y);
                                        } else {
                                            let item = state.get_item_mut()?;
                                            item.selected = false;
                                            state.layout.nums.go_down();
                                            state.layout.nums.inc_skip();
                                            state.redraw(state.layout.y);
                                        }
                                    } else if state.layout.nums.index >= start_pos {
                                        state.layout.nums.go_down();
                                        let item = state.get_item_mut()?;
                                        item.selected = true;
                                        state.redraw(state.layout.y + 1);
                                    } else {
                                        let item = state.get_item_mut()?;
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
                                            let item = state.get_item_mut()?;
                                            item.selected = false;
                                            state.layout.nums.go_up();
                                            state.layout.nums.dec_skip();
                                            state.redraw(state.layout.y);
                                        } else {
                                            state.layout.nums.go_up();
                                            state.layout.nums.dec_skip();
                                            let item = state.get_item_mut()?;
                                            item.selected = true;
                                            state.redraw(state.layout.y);
                                        }
                                    } else if state.layout.nums.index > start_pos {
                                        let item = state.get_item_mut()?;
                                        item.selected = false;
                                        state.layout.nums.go_up();
                                        state.redraw(state.layout.y - 1);
                                    } else {
                                        state.layout.nums.go_up();
                                        let item = state.get_item_mut()?;
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

                                        if let Event::Key(KeyEvent {
                                            code,
                                            kind: KeyEventKind::Press,
                                            ..
                                        }) = event::read()?
                                        {
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

                                    if let Event::Key(KeyEvent {
                                        code,
                                        kind: KeyEventKind::Press,
                                        ..
                                    }) = event::read()?
                                    {
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

                            //Go to the parent directory if exists
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

                            // jump forward
                            KeyCode::Tab => {
                                if let Some(path_to_jump_to) = state.jumplist.get_forward() {
                                    if path_to_jump_to.exists() {
                                        state.chdir(&path_to_jump_to, Move::List)?;
                                    } else {
                                        print_warning(
                                            "Directory forward not found: Removed from jumplist.",
                                            state.layout.y,
                                        );
                                        state.jumplist.remove_forward();
                                    }
                                    state.jumplist.pos_forward();
                                }
                            }

                            //Unpack archive file. Fails if it is not any of supported types
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

                            //Jumps to the directory that matches the keyword (zoxide required)
                            KeyCode::Char('z') => {
                                //If zoxide is not found, show error message.
                                if !state.has_zoxide {
                                    print_warning("zoxide not found.", state.layout.y);
                                    continue;
                                }
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                delete_pointer();
                                go_to_info_line_and_reset();
                                print!("z");
                                show_cursor();

                                let mut command: Vec<char> = vec!['z'];
                                screen.flush()?;

                                let mut current_pos = 3;
                                'zoxide: loop {
                                    if let Event::Key(KeyEvent {
                                        code,
                                        modifiers,
                                        kind: KeyEventKind::Press,
                                        ..
                                    }) = event::read()?
                                    {
                                        match (code, modifiers) {
                                            (KeyCode::Esc, KeyModifiers::NONE) => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                                state.move_cursor(state.layout.y);
                                                break 'zoxide;
                                            }

                                            (KeyCode::Left, KeyModifiers::NONE) => {
                                                if current_pos == INITIAL_POS_Z {
                                                    continue;
                                                };
                                                current_pos -= 1;
                                                move_left(1);
                                            }

                                            (KeyCode::Right, KeyModifiers::NONE) => {
                                                if current_pos as usize
                                                    == command.len() + INITIAL_POS_Z as usize
                                                {
                                                    continue;
                                                };
                                                current_pos += 1;
                                                move_right(1);
                                            }

                                            (KeyCode::Backspace, KeyModifiers::NONE)
                                            | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                                                if current_pos == INITIAL_POS_Z + 1 {
                                                    go_to_info_line_and_reset();
                                                    hide_cursor();
                                                    state.move_cursor(state.layout.y);
                                                    break 'zoxide;
                                                };
                                                command.remove(
                                                    (current_pos - INITIAL_POS_Z - 1).into(),
                                                );
                                                current_pos -= 1;

                                                clear_current_line();
                                                to_info_line();
                                                print!("{}", &command.iter().collect::<String>(),);
                                                move_to(current_pos, 2);
                                            }

                                            (KeyCode::Enter, KeyModifiers::NONE) => {
                                                hide_cursor();
                                                let command = command.iter().collect::<String>();
                                                let commands = command
                                                    .split_whitespace()
                                                    .collect::<Vec<&str>>();
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
                                                    let home_dir =
                                                        dirs::home_dir().ok_or_else(|| {
                                                            FxError::Dirs(
                                                                "Cannot read home dir.".to_string(),
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
                                                                print_warning(e, state.layout.y);
                                                                break 'zoxide;
                                                            }
                                                            Ok(target_dir) => {
                                                                hide_cursor();
                                                                state.layout.nums.reset();
                                                                let target_path = PathBuf::from(
                                                                    target_dir.trim(),
                                                                );
                                                                if let Err(e) = state
                                                                    .chdir(&target_path, Move::Jump)
                                                                {
                                                                    print_warning(
                                                                        e,
                                                                        state.layout.y,
                                                                    );
                                                                }
                                                                break 'zoxide;
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    print_warning(
                                                        "Failed to execute zoxide",
                                                        state.layout.y,
                                                    );
                                                    break 'zoxide;
                                                }
                                            }

                                            (KeyCode::Char(c), _) => {
                                                command.insert(
                                                    (current_pos - INITIAL_POS_Z).into(),
                                                    c,
                                                );
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

                            //insert mode
                            KeyCode::Char('i') | KeyCode::Char('I') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                let is_dir = code == KeyCode::Char('I');
                                delete_pointer();
                                go_to_info_line_and_reset();
                                // No need to place sign of the insert mode
                                print!(" ");
                                show_cursor();
                                screen.flush()?;

                                let mut new_name: Vec<char> = Vec::new();

                                // express position in terminal
                                let mut current_pos = INITIAL_POS_COMMAND_LINE;
                                // express position in Vec<Char>
                                let mut current_char_pos = 0;
                                'insert: loop {
                                    if let Event::Key(KeyEvent {
                                        code,
                                        modifiers,
                                        kind: KeyEventKind::Press,
                                        ..
                                    }) = event::read()?
                                    {
                                        match (code, modifiers) {
                                            // <C-r> to put the item name(s) from register
                                            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                                                if let Event::Key(KeyEvent {
                                                    code,
                                                    kind: KeyEventKind::Press,
                                                    ..
                                                }) = event::read()?
                                                {
                                                    if let Some(reg) =
                                                        state.registers.check_reg(&code)
                                                    {
                                                        if !reg.is_empty() {
                                                            let to_be_inserted = reg
                                                                .iter()
                                                                .map(|x| x.file_name.clone())
                                                                .collect::<Vec<String>>()
                                                                .join(" ");
                                                            for c in to_be_inserted.chars() {
                                                                if let Some(to_be_added) =
                                                                    unicode_width::UnicodeWidthChar::width(c)
                                                                {
                                                                    if current_pos + to_be_added as u16
                                                                        > state.layout.terminal_column
                                                                    {
                                                                        continue;
                                                                    }
                                                                    new_name.insert(current_char_pos, c);
                                                                    current_char_pos += 1;
                                                                    current_pos += to_be_added as u16;
                                                                }
                                                            }
                                                            go_to_info_line_and_reset();
                                                            print!(
                                                                " {}",
                                                                &new_name
                                                                    .iter()
                                                                    .collect::<String>(),
                                                            );
                                                            move_to(current_pos, 2);
                                                            screen.flush()?;
                                                            continue;
                                                        } else {
                                                            continue;
                                                        }
                                                    } else {
                                                        continue;
                                                    }
                                                }
                                            }

                                            (KeyCode::Esc, KeyModifiers::NONE) => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                                state.move_cursor(state.layout.y);
                                                break 'insert;
                                            }

                                            (KeyCode::Left, KeyModifiers::NONE) => {
                                                if current_char_pos == 0 {
                                                    continue;
                                                };
                                                if let Some(to_be_skipped) =
                                                    unicode_width::UnicodeWidthChar::width(
                                                        new_name[current_char_pos - 1],
                                                    )
                                                {
                                                    current_char_pos -= 1;
                                                    current_pos -= to_be_skipped as u16;
                                                    move_left(to_be_skipped as u16);
                                                }
                                            }

                                            (KeyCode::Right, KeyModifiers::NONE) => {
                                                if current_char_pos == new_name.len() {
                                                    continue;
                                                };
                                                if let Some(to_be_skipped) =
                                                    unicode_width::UnicodeWidthChar::width(
                                                        new_name[current_char_pos],
                                                    )
                                                {
                                                    current_char_pos += 1;
                                                    current_pos += to_be_skipped as u16;
                                                    move_right(to_be_skipped as u16);
                                                }
                                            }

                                            (KeyCode::Backspace, KeyModifiers::NONE)
                                            | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                                                if current_char_pos == 0 {
                                                    continue;
                                                };
                                                let removed = new_name.remove(current_char_pos - 1);
                                                if let Some(to_be_removed) =
                                                    unicode_width::UnicodeWidthChar::width(removed)
                                                {
                                                    current_char_pos -= 1;
                                                    current_pos -= to_be_removed as u16;

                                                    go_to_info_line_and_reset();
                                                    print!(
                                                        " {}",
                                                        &new_name.iter().collect::<String>(),
                                                    );
                                                    move_to(current_pos, 2);
                                                }
                                            }

                                            (KeyCode::Enter, KeyModifiers::NONE) => {
                                                hide_cursor();
                                                //Set the command and argument(s).
                                                let new_name: String = new_name.iter().collect();
                                                if is_dir {
                                                    if let Err(e) = std::fs::create_dir(
                                                        &state.current_dir.join(new_name),
                                                    ) {
                                                        print_warning(e, state.layout.y);
                                                        break 'insert;
                                                    }
                                                } else if let Err(e) = std::fs::File::options()
                                                    .read(true)
                                                    .write(true)
                                                    .create_new(true)
                                                    .open(&state.current_dir.join(new_name))
                                                {
                                                    print_warning(e, state.layout.y);
                                                    break 'insert;
                                                }
                                                state.reload(state.layout.y)?;
                                                break 'insert;
                                            }

                                            (KeyCode::Char(c), _) => {
                                                if let Some(to_be_added) =
                                                    unicode_width::UnicodeWidthChar::width(c)
                                                {
                                                    if current_pos + to_be_added as u16
                                                        > state.layout.terminal_column
                                                    {
                                                        continue;
                                                    }
                                                    new_name.insert(current_char_pos, c);
                                                    current_char_pos += 1;
                                                    current_pos += to_be_added as u16;

                                                    go_to_info_line_and_reset();
                                                    print!(
                                                        " {}",
                                                        &new_name.iter().collect::<String>(),
                                                    );
                                                    move_to(current_pos, 2);
                                                }
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
                                let item = state.get_item_mut()?;
                                item.selected = true;
                                state.redraw(state.layout.y);
                                state.v_start = Some(state.layout.nums.index);
                                continue;
                            }

                            //Toggle sortkey
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

                            //Show or hide hidden items
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

                            //Toggle whether to show preview. Also hide registers.
                            KeyCode::Char('v') => {
                                if state.layout.is_preview() || state.layout.is_reg() {
                                    state.layout.reset_side();
                                } else {
                                    state.layout.show_preview();
                                }
                                let (new_column, new_row) = state.layout.update_column_and_row()?;
                                state.refresh(new_column, new_row, state.layout.y)?;
                            }

                            //Toggle vertical <-> horizontal split
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
                                    if let Err(e) = state.delete_in_visual(None, false, &mut screen)
                                    {
                                        state.reset_selection();
                                        state.redraw(state.layout.y);
                                        print_warning(e, state.layout.y);
                                        continue;
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

                                        if let Event::Key(KeyEvent {
                                            code,
                                            kind: KeyEventKind::Press,
                                            ..
                                        }) = event::read()?
                                        {
                                            match code {
                                                KeyCode::Char('d') => {
                                                    if let Err(e) =
                                                        state.delete(None, false, &mut screen)
                                                    {
                                                        print_warning(e, state.layout.y);
                                                        continue;
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
                                    let item_len = state.registers.yank_item(&items, None, false);
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
                                                    state.registers.yank_item(
                                                        &[ItemBuffer::new(item)],
                                                        None,
                                                        false,
                                                    );
                                                    go_to_info_line_and_reset();
                                                    hide_cursor();
                                                    print_info("1 item yanked.", state.layout.y);
                                                }
                                            }

                                            _ => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                            }
                                        }
                                    }
                                }
                                state.move_cursor(state.layout.y);
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
                                    if let Event::Key(KeyEvent {
                                        code,
                                        modifiers,
                                        kind: KeyEventKind::Press,
                                        ..
                                    }) = event::read()?
                                    {
                                        match (code, modifiers) {
                                            //rename item
                                            (KeyCode::Enter, KeyModifiers::NONE) => {
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

                                            (KeyCode::Esc, KeyModifiers::NONE) => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                                state.move_cursor(state.layout.y);
                                                break;
                                            }

                                            (KeyCode::Left, KeyModifiers::NONE) => {
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

                                            (KeyCode::Right, KeyModifiers::NONE) => {
                                                if current_char_pos == rename.len() {
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

                                            (KeyCode::Backspace, KeyModifiers::NONE)
                                            | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                                                if current_char_pos == 0 {
                                                    continue;
                                                };
                                                let removed = rename.remove(current_char_pos - 1);
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

                                            (KeyCode::Char(c), _) => {
                                                if let Some(to_be_added) =
                                                    unicode_width::UnicodeWidthChar::width(c)
                                                {
                                                    rename.insert(current_char_pos, c);
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

                                            _ => continue,
                                        }
                                        screen.flush()?;
                                    }
                                }
                            }

                            //Search mode
                            KeyCode::Char('/') => {
                                //In visual mode, this is disabled.
                                //TODO! Enable this in visual mode.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                if len == 0 {
                                    continue;
                                }
                                delete_pointer();
                                show_cursor();
                                go_to_info_line_and_reset();
                                print!("/");
                                screen.flush()?;

                                let original_nums = state.layout.nums;
                                let original_y = state.layout.y;
                                let mut keyword: Vec<char> = Vec::new();

                                // express position in terminal
                                let mut current_pos = INITIAL_POS_COMMAND_LINE;
                                // express position in Vec<Char>
                                let mut current_char_pos = 0;
                                loop {
                                    if let Event::Key(KeyEvent {
                                        code,
                                        modifiers,
                                        kind: KeyEventKind::Press,
                                        ..
                                    }) = event::read()?
                                    {
                                        match (code, modifiers) {
                                            (KeyCode::Enter, KeyModifiers::NONE) => {
                                                go_to_info_line_and_reset();
                                                state.keyword = Some(keyword.iter().collect());
                                                state.move_cursor(state.layout.y);
                                                break;
                                            }

                                            (KeyCode::Esc, KeyModifiers::NONE) => {
                                                hide_cursor();
                                                state.redraw(state.layout.y);
                                                break;
                                            }

                                            (KeyCode::Left, KeyModifiers::NONE) => {
                                                if current_char_pos == 0 {
                                                    continue;
                                                };
                                                if let Some(to_be_skipped) =
                                                    unicode_width::UnicodeWidthChar::width(
                                                        keyword[current_char_pos - 1],
                                                    )
                                                {
                                                    current_char_pos -= 1;
                                                    current_pos -= to_be_skipped as u16;
                                                    move_left(to_be_skipped as u16);
                                                }
                                            }

                                            (KeyCode::Right, KeyModifiers::NONE) => {
                                                if current_char_pos == keyword.len() {
                                                    continue;
                                                };
                                                if let Some(to_be_skipped) =
                                                    unicode_width::UnicodeWidthChar::width(
                                                        keyword[current_char_pos],
                                                    )
                                                {
                                                    current_char_pos += 1;
                                                    current_pos += to_be_skipped as u16;
                                                    move_right(to_be_skipped as u16);
                                                }
                                            }

                                            (KeyCode::Backspace, KeyModifiers::NONE)
                                            | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                                                if current_char_pos == 0 {
                                                    continue;
                                                };
                                                let removed = keyword.remove(current_char_pos - 1);
                                                if let Some(to_be_removed) =
                                                    unicode_width::UnicodeWidthChar::width(removed)
                                                {
                                                    current_char_pos -= 1;
                                                    current_pos -= to_be_removed as u16;

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
                                                    move_to(current_pos, 2);
                                                }
                                            }

                                            (KeyCode::Char(c), _) => {
                                                if let Some(to_be_added) =
                                                    unicode_width::UnicodeWidthChar::width(c)
                                                {
                                                    if current_pos + to_be_added as u16
                                                        > state.layout.terminal_column
                                                    {
                                                        continue;
                                                    }
                                                    keyword.insert(current_char_pos, c);
                                                    current_char_pos += 1;
                                                    current_pos += to_be_added as u16;

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
                                                    move_to(current_pos, 2);
                                                }
                                            }

                                            _ => continue,
                                        }
                                        screen.flush()?;
                                    }
                                }
                                hide_cursor();
                            }

                            //Search forward
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

                            //Search backward
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

                            //Tinker with registers
                            KeyCode::Char('"') => {
                                go_to_info_line_and_reset();
                                print!("\"");
                                show_cursor();
                                screen.flush()?;

                                let mut command: Vec<char> = Vec::new();

                                let mut current_pos = INITIAL_POS_COMMAND_LINE;
                                'reg: loop {
                                    if let Event::Key(KeyEvent {
                                        code,
                                        kind: KeyEventKind::Press,
                                        ..
                                    }) = event::read()?
                                    {
                                        match code {
                                            KeyCode::Esc => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                                state.move_cursor(state.layout.y);
                                                break 'reg;
                                            }

                                            KeyCode::Left => {
                                                if current_pos == INITIAL_POS_COMMAND_LINE {
                                                    continue;
                                                };
                                                current_pos -= 1;
                                                move_left(1);
                                            }

                                            KeyCode::Right => {
                                                if current_pos as usize
                                                    == command.len()
                                                        + INITIAL_POS_COMMAND_LINE as usize
                                                {
                                                    continue;
                                                };
                                                current_pos += 1;
                                                move_right(1);
                                            }

                                            KeyCode::Backspace => {
                                                if current_pos == INITIAL_POS_COMMAND_LINE {
                                                    go_to_info_line_and_reset();
                                                    hide_cursor();
                                                    state.move_cursor(state.layout.y);
                                                    break 'reg;
                                                } else {
                                                    command.remove(
                                                        (current_pos
                                                            - INITIAL_POS_COMMAND_LINE
                                                            - 1)
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
                                                    (current_pos - INITIAL_POS_COMMAND_LINE).into(),
                                                    c,
                                                );
                                                if ((state.v_start.is_some() || c == 'p')
                                                    && command.len() == 2)
                                                    || (state.v_start.is_none()
                                                        && command.len() == 3)
                                                {
                                                    if !command[0].is_ascii_alphanumeric() {
                                                        print_warning(
                                                            "Input not supported.",
                                                            state.layout.y,
                                                        );
                                                        break 'reg;
                                                    }
                                                    let action: String =
                                                        command[1..].iter().collect();
                                                    match action.as_str() {
                                                        //put
                                                        "p" => {
                                                            //In read-only directory, put disabled
                                                            if state.is_ro {
                                                                go_to_info_line_and_reset();
                                                                print_warning(
                                        "Cannot put item in this directory.",
                                        state.layout.y,
                                    );
                                                                hide_cursor();
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            if state.v_start.is_some() {
                                                                clear_current_line();
                                                                hide_cursor();
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            let target = match command[0] {
                                                                '0' => Some(&state.registers.zero),
                                                                '1'..='9' => {
                                                                    state.registers.numbered.get(
                                                                        command[0]
                                                                            .to_digit(10)
                                                                            .unwrap()
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
                                                                    print_warning(
                                                                        e,
                                                                        state.layout.y,
                                                                    );
                                                                    break 'reg;
                                                                }
                                                            } else {
                                                                print_warning(
                                                                    "Register not found.",
                                                                    state.layout.y,
                                                                );
                                                            }
                                                            state.move_cursor(state.layout.y);
                                                            break 'reg;
                                                        }
                                                        //yank (normal mode)
                                                        "yy" => {
                                                            if state.v_start.is_some() {
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            if command[0].is_ascii_lowercase() {
                                                                if let Ok(item) = state.get_item() {
                                                                    state.registers.yank_item(
                                                                        &[ItemBuffer::new(item)],
                                                                        Some(command[0]),
                                                                        false,
                                                                    );
                                                                }
                                                            } else if command[0]
                                                                .is_ascii_uppercase()
                                                            {
                                                                if let Ok(item) = state.get_item() {
                                                                    state.registers.yank_item(
                                                                        &[ItemBuffer::new(item)],
                                                                        Some(
                                                                            command[0]
                                                                                .to_ascii_lowercase(
                                                                                ),
                                                                        ),
                                                                        true,
                                                                    );
                                                                }
                                                            } else {
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            go_to_info_line_and_reset();
                                                            hide_cursor();
                                                            print_info(
                                                                "1 item yanked.",
                                                                state.layout.y,
                                                            );
                                                            state.move_cursor(state.layout.y);
                                                            break 'reg;
                                                        }
                                                        //yank (visual mode)
                                                        "y" => {
                                                            if state.v_start.is_none() {
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            let items: Vec<ItemBuffer> = state
                                                                .list
                                                                .iter()
                                                                .filter(|item| item.selected)
                                                                .map(ItemBuffer::new)
                                                                .collect();
                                                            let item_len: usize;
                                                            if command[0].is_ascii_lowercase() {
                                                                item_len =
                                                                    state.registers.yank_item(
                                                                        &items,
                                                                        Some(command[0]),
                                                                        false,
                                                                    );
                                                            } else if command[0]
                                                                .is_ascii_uppercase()
                                                            {
                                                                item_len =
                                                                    state.registers.yank_item(
                                                                        &items,
                                                                        Some(
                                                                            command[0]
                                                                                .to_ascii_lowercase(
                                                                                ),
                                                                        ),
                                                                        true,
                                                                    );
                                                            } else {
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            state.reset_selection();
                                                            state.list_up();
                                                            let mut yank_message: String =
                                                                item_len.to_string();
                                                            yank_message.push_str(" items yanked");
                                                            print_info(
                                                                yank_message,
                                                                state.layout.y,
                                                            );
                                                            state.move_cursor(state.layout.y);
                                                            break 'reg;
                                                        }

                                                        //delete (normal mode)
                                                        "dd" => {
                                                            //In read-only directory, delete
                                                            //disabled
                                                            if state.is_ro {
                                                                go_to_info_line_and_reset();
                                                                print_warning(
                                        "Cannot delete item in this directory.",
                                        state.layout.y,
                                    );
                                                                hide_cursor();
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            if state.v_start.is_some() {
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            if command[0].is_ascii_lowercase() {
                                                                if let Err(e) = state.delete(
                                                                    Some(command[0]),
                                                                    false,
                                                                    &mut screen,
                                                                ) {
                                                                    print_warning(
                                                                        e,
                                                                        state.layout.y,
                                                                    );
                                                                    break 'reg;
                                                                }
                                                            } else if command[0]
                                                                .is_ascii_uppercase()
                                                            {
                                                                if let Err(e) = state.delete(
                                                                    Some(
                                                                        command[0]
                                                                            .to_ascii_lowercase(),
                                                                    ),
                                                                    true,
                                                                    &mut screen,
                                                                ) {
                                                                    print_warning(
                                                                        e,
                                                                        state.layout.y,
                                                                    );
                                                                    break 'reg;
                                                                }
                                                            }
                                                            state.move_cursor(state.layout.y);
                                                            break 'reg;
                                                        }
                                                        //delete (visual mode)
                                                        "d" => {
                                                            //In read-only directory, delete
                                                            //disabled
                                                            if state.is_ro {
                                                                go_to_info_line_and_reset();
                                                                print_warning(
                                        "Cannot delete item in this directory.",
                                        state.layout.y,
                                    );
                                                                hide_cursor();
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            if state.v_start.is_none() {
                                                                state.move_cursor(state.layout.y);
                                                                break 'reg;
                                                            }
                                                            if command[0].is_ascii_lowercase() {
                                                                if let Err(e) = state
                                                                    .delete_in_visual(
                                                                        Some(command[0]),
                                                                        false,
                                                                        &mut screen,
                                                                    )
                                                                {
                                                                    state.reset_selection();
                                                                    state.redraw(state.layout.y);
                                                                    print_warning(
                                                                        e,
                                                                        state.layout.y,
                                                                    );
                                                                    break 'reg;
                                                                }
                                                            } else if command[0]
                                                                .is_ascii_uppercase()
                                                            {
                                                                if let Err(e) = state
                                                                    .delete_in_visual(
                                                                        Some(
                                                                            command[0]
                                                                                .to_ascii_lowercase(
                                                                                ),
                                                                        ),
                                                                        true,
                                                                        &mut screen,
                                                                    )
                                                                {
                                                                    state.reset_selection();
                                                                    state.redraw(state.layout.y);
                                                                    print_warning(
                                                                        e,
                                                                        state.layout.y,
                                                                    );
                                                                    break 'reg;
                                                                }
                                                            }
                                                            state.move_cursor(state.layout.y);
                                                            break 'reg;
                                                        }
                                                        _ => {
                                                            clear_current_line();
                                                            hide_cursor();
                                                            state.move_cursor(state.layout.y);
                                                            break 'reg;
                                                        }
                                                    }
                                                } else {
                                                    current_pos += 1;
                                                    clear_current_line();
                                                    to_info_line();
                                                    print!(
                                                        "\"{}",
                                                        &command.iter().collect::<String>(),
                                                    );
                                                    move_to(current_pos, 2);
                                                }
                                            }

                                            _ => continue,
                                        }
                                        screen.flush()?;
                                    }
                                }
                            }

                            //command line
                            KeyCode::Char(':') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                delete_pointer();
                                go_to_info_line_and_reset();
                                print!(":");
                                show_cursor();
                                screen.flush()?;

                                let mut command: Vec<char> = Vec::new();

                                // express position in terminal
                                let mut current_pos = INITIAL_POS_COMMAND_LINE;
                                // express position in Vec<Char>
                                let mut current_char_pos = 0;
                                'command: loop {
                                    if let Event::Key(KeyEvent {
                                        code,
                                        modifiers,
                                        kind: KeyEventKind::Press,
                                        ..
                                    }) = event::read()?
                                    {
                                        match (code, modifiers) {
                                            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                                                if let Event::Key(KeyEvent {
                                                    code,
                                                    kind: KeyEventKind::Press,
                                                    ..
                                                }) = event::read()?
                                                {
                                                    if let Some(reg) =
                                                        state.registers.check_reg(&code)
                                                    {
                                                        if !reg.is_empty() {
                                                            let to_be_inserted = reg
                                                                .iter()
                                                                .map(|x| x.file_name.clone())
                                                                .collect::<Vec<String>>()
                                                                .join(" ");
                                                            for c in to_be_inserted.chars() {
                                                                if let Some(to_be_added) =
                                                        unicode_width::UnicodeWidthChar::width(c)
                                                    {
                                                        if current_pos + to_be_added as u16
                                                            > state.layout.terminal_column
                                                        {
                                                            continue;
                                                        }
                                                        command.insert(current_char_pos, c);
                                                        current_char_pos += 1;
                                                        current_pos += to_be_added as u16;
                                                    }
                                                            }
                                                            go_to_info_line_and_reset();
                                                            print!(
                                                                ":{}",
                                                                &command.iter().collect::<String>(),
                                                            );
                                                            move_to(current_pos, 2);
                                                            screen.flush()?;
                                                            continue;
                                                        } else {
                                                            continue;
                                                        }
                                                    } else {
                                                        continue;
                                                    }
                                                }
                                            }

                                            (KeyCode::Esc, KeyModifiers::NONE) => {
                                                go_to_info_line_and_reset();
                                                hide_cursor();
                                                state.move_cursor(state.layout.y);
                                                break 'command;
                                            }

                                            (KeyCode::Left, KeyModifiers::NONE) => {
                                                if current_char_pos == 0 {
                                                    continue;
                                                };
                                                if let Some(to_be_skipped) =
                                                    unicode_width::UnicodeWidthChar::width(
                                                        command[current_char_pos - 1],
                                                    )
                                                {
                                                    current_char_pos -= 1;
                                                    current_pos -= to_be_skipped as u16;
                                                    move_left(to_be_skipped as u16);
                                                }
                                            }

                                            (KeyCode::Right, KeyModifiers::NONE) => {
                                                if current_char_pos == command.len() {
                                                    continue;
                                                };
                                                if let Some(to_be_skipped) =
                                                    unicode_width::UnicodeWidthChar::width(
                                                        command[current_char_pos],
                                                    )
                                                {
                                                    current_char_pos += 1;
                                                    current_pos += to_be_skipped as u16;
                                                    move_right(to_be_skipped as u16);
                                                }
                                            }

                                            (KeyCode::Backspace, KeyModifiers::NONE)
                                            | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                                                if current_char_pos == 0 {
                                                    continue;
                                                };
                                                let removed = command.remove(current_char_pos - 1);
                                                if let Some(to_be_removed) =
                                                    unicode_width::UnicodeWidthChar::width(removed)
                                                {
                                                    current_char_pos -= 1;
                                                    current_pos -= to_be_removed as u16;

                                                    go_to_info_line_and_reset();
                                                    print!(
                                                        ":{}",
                                                        &command.iter().collect::<String>(),
                                                    );
                                                    move_to(current_pos, 2);
                                                }
                                            }

                                            (KeyCode::Enter, KeyModifiers::NONE) => {
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
                                                        //:reg - Show registers
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
                                                } else if commands.len() == 2 && command == "cd" {
                                                    if let Ok(target) =
                                                        std::path::Path::new(commands[1])
                                                            .canonicalize()
                                                    {
                                                        if target.exists() {
                                                            if let Err(e) =
                                                                state.chdir(&target, Move::Jump)
                                                            {
                                                                print_warning(e, state.layout.y);
                                                            }
                                                            break 'command;
                                                        } else {
                                                            print_warning(
                                                                "Path does not exist.",
                                                                state.layout.y,
                                                            );
                                                            break 'command;
                                                        }
                                                    } else {
                                                        print_warning(
                                                            "Path does not exist.",
                                                            state.layout.y,
                                                        );
                                                        break 'command;
                                                    }
                                                }

                                                //Execute command as is
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
                                                if let Ok(sh) = std::env::var("SHELL") {
                                                    if std::process::Command::new(&sh)
                                                        .arg("-c")
                                                        .arg(&commands.join(" "))
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
                                                } else if std::process::Command::new(command)
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

                                            (KeyCode::Char(c), _) => {
                                                if let Some(to_be_added) =
                                                    unicode_width::UnicodeWidthChar::width(c)
                                                {
                                                    if current_pos + to_be_added as u16
                                                        > state.layout.terminal_column
                                                    {
                                                        continue;
                                                    }
                                                    command.insert(current_char_pos, c);
                                                    current_char_pos += 1;
                                                    current_pos += to_be_added as u16;

                                                    go_to_info_line_and_reset();
                                                    print!(
                                                        ":{}",
                                                        &command.iter().collect::<String>(),
                                                    );
                                                    move_to(current_pos, 2);
                                                }
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

                            //exit by ZZ
                            KeyCode::Char('Z') => {
                                //In visual mode, this is disabled.
                                if state.v_start.is_some() {
                                    continue;
                                }
                                delete_pointer();
                                go_to_info_line_and_reset();
                                print!("Z");
                                show_cursor();
                                screen.flush()?;

                                let mut next_key: Event = event::read()?;
                                // ignore exactly one keypress Release after a Z is entered
                                if let Event::Key(KeyEvent {
                                    kind: KeyEventKind::Release,
                                    ..
                                }) = next_key
                                {
                                    next_key = event::read()?;
                                }

                                if let Event::Key(KeyEvent {
                                    code,
                                    kind: KeyEventKind::Press,
                                    ..
                                }) = next_key
                                {
                                    match code {
                                        KeyCode::Char('Q') => {
                                            if state.match_vim_exit_behavior
                                                || state.export_lwd().is_ok()
                                            {
                                                break 'main;
                                            }
                                        }

                                        KeyCode::Char('Z') => {
                                            if !state.match_vim_exit_behavior
                                                || state.export_lwd().is_ok()
                                            {
                                                break 'main;
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
                //If you use kitty, clear the screen by the escape sequence or the previewed image remains.
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

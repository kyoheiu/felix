use crossterm::{
    cursor::{Hide, MoveLeft, MoveTo},
    terminal::Clear,
};

pub fn move_to(x: u16, y: u16) {
    print!("{}", MoveTo(x - 1, y - 1));
}

pub fn clear_current_line() {
    print!("{}", Clear(crossterm::terminal::ClearType::CurrentLine));
}

pub fn move_left(x: u16) {
    print!("{}", MoveLeft(x));
}

pub fn hide_cursor() {
    print!("{}", Hide);
}

pub fn print_cursor() {
    print!(">");
}

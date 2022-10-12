use crossterm::{
    cursor::{Hide, MoveLeft, MoveTo},
    style::{Color, Colors, ResetColor, SetColors},
    terminal::Clear,
};

pub fn move_to(x: u16, y: u16) {
    print!("{}", MoveTo(x - 1, y - 1));
}

pub fn clear_current_line() {
    print!("{}", Clear(crossterm::terminal::ClearType::CurrentLine));
}

pub fn clear_until_newline() {
    print!("{}", Clear(crossterm::terminal::ClearType::UntilNewLine));
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

pub fn set_color(foreground: Option<Color>, background: Option<Color>) {
    let colors = Colors {
        foreground,
        background,
    };
    print!("{}", SetColors(colors));
}

pub fn reset_color() {
    print!("{}", ResetColor);
}

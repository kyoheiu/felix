use super::config::Colorname;

use crossterm::{
    cursor::{Hide, MoveLeft, MoveRight, MoveTo, Show},
    style::{Color, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::Clear,
};

pub enum TermColor<'a> {
    ForeGround(&'a Colorname),
    BackGround(&'a Colorname),
}

pub fn move_to(x: u16, y: u16) {
    print!("{}", MoveTo(x - 1, y - 1));
}

pub fn to_info_bar() {
    move_to(2, 2);
}

pub fn clear_current_line() {
    print!("{}", Clear(crossterm::terminal::ClearType::CurrentLine));
}

pub fn clear_until_newline() {
    print!("{}", Clear(crossterm::terminal::ClearType::UntilNewLine));
}

pub fn clear_all() {
    print!("{}", Clear(crossterm::terminal::ClearType::All));
}

pub fn move_left(x: u16) {
    print!("{}", MoveLeft(x));
}

pub fn move_right(x: u16) {
    print!("{}", MoveRight(x));
}

pub fn hide_cursor() {
    print!("{}", Hide);
}

pub fn show_cursor() {
    print!("{}", Show);
}

pub fn print_pointer() {
    print!(">");
}

pub fn set_color(c: &TermColor) {
    match c {
        TermColor::ForeGround(c) => match c {
            Colorname::Black => print!("{}", SetForegroundColor(Color::Black)),
            Colorname::Red => print!("{}", SetForegroundColor(Color::DarkRed)),
            Colorname::Green => print!("{}", SetForegroundColor(Color::DarkGreen)),
            Colorname::Yellow => print!("{}", SetForegroundColor(Color::DarkYellow)),
            Colorname::Blue => print!("{}", SetForegroundColor(Color::DarkBlue)),
            Colorname::Magenta => print!("{}", SetForegroundColor(Color::DarkMagenta)),
            Colorname::Cyan => print!("{}", SetForegroundColor(Color::DarkCyan)),
            Colorname::White => print!("{}", SetForegroundColor(Color::Grey)),
            Colorname::LightBlack => print!("{}", SetForegroundColor(Color::DarkGrey)),
            Colorname::LightRed => print!("{}", SetForegroundColor(Color::Red)),
            Colorname::LightGreen => print!("{}", SetForegroundColor(Color::Green)),
            Colorname::LightYellow => print!("{}", SetForegroundColor(Color::Yellow)),
            Colorname::LightBlue => print!("{}", SetForegroundColor(Color::Blue)),
            Colorname::LightMagenta => print!("{}", SetForegroundColor(Color::Magenta)),
            Colorname::LightCyan => print!("{}", SetForegroundColor(Color::Cyan)),
            Colorname::LightWhite => print!("{}", SetForegroundColor(Color::White)),
            Colorname::Rgb(r, g, b) => print!(
                "{}",
                SetForegroundColor(Color::Rgb {
                    r: *r,
                    g: *g,
                    b: *b
                })
            ),
            Colorname::AnsiValue(n) => print!("{}", SetForegroundColor(Color::AnsiValue(*n))),
        },
        TermColor::BackGround(c) => match c {
            Colorname::Black => print!("{}", SetBackgroundColor(Color::Black)),
            Colorname::Red => print!("{}", SetBackgroundColor(Color::DarkRed)),
            Colorname::Green => print!("{}", SetBackgroundColor(Color::DarkGreen)),
            Colorname::Yellow => print!("{}", SetBackgroundColor(Color::DarkYellow)),
            Colorname::Blue => print!("{}", SetBackgroundColor(Color::DarkBlue)),
            Colorname::Magenta => print!("{}", SetBackgroundColor(Color::DarkMagenta)),
            Colorname::Cyan => print!("{}", SetBackgroundColor(Color::DarkCyan)),
            Colorname::White => print!("{}", SetBackgroundColor(Color::Grey)),
            Colorname::LightBlack => print!("{}", SetBackgroundColor(Color::DarkGrey)),
            Colorname::LightRed => print!("{}", SetBackgroundColor(Color::Red)),
            Colorname::LightGreen => print!("{}", SetBackgroundColor(Color::Green)),
            Colorname::LightYellow => print!("{}", SetBackgroundColor(Color::Yellow)),
            Colorname::LightBlue => print!("{}", SetBackgroundColor(Color::Blue)),
            Colorname::LightMagenta => print!("{}", SetBackgroundColor(Color::Magenta)),
            Colorname::LightCyan => print!("{}", SetBackgroundColor(Color::Cyan)),
            Colorname::LightWhite => print!("{}", SetBackgroundColor(Color::White)),
            Colorname::Rgb(r, g, b) => print!(
                "{}",
                SetBackgroundColor(Color::Rgb {
                    r: *r,
                    g: *g,
                    b: *b
                })
            ),
            Colorname::AnsiValue(n) => print!("{}", SetBackgroundColor(Color::AnsiValue(*n))),
        },
    }
}

pub fn reset_color() {
    print!("{}", ResetColor);
}

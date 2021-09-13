use pancurses::{endwin, initscr};

fn main() {
    let w = initscr();

    let path = std::env::current_dir();

    if let Ok(p) = path {
        let s = p.to_str().unwrap();
        w.printw(s);
        w.refresh();
        w.getch();
        endwin();
    }
}

use std::convert::TryInto;
use std::env::current_dir;
use std::fs;
use std::io::{stdin, stdout, Error, Read, Write};
use termion::cursor::DetectCursorPos;
use termion::{async_stdin, clear, color, cursor, raw::IntoRawMode, AsyncReader};

struct EntryInfo {
    line_number: usize,
    file_path: std::path::PathBuf,
    file_name: String,
}

fn make_parent_dir(p: std::path::PathBuf) -> EntryInfo {
    return EntryInfo {
        line_number: 0,
        file_path: p.to_path_buf(),
        file_name: "../".to_string(),
    };
}

fn make_entry(i: usize, dir: std::fs::DirEntry) -> EntryInfo {
    return EntryInfo {
        line_number: i,
        file_path: dir.path(),
        file_name: dir.file_name().into_string().unwrap(),
    };
}

fn push_entries(p: &std::path::PathBuf) -> Result<Vec<EntryInfo>, Error> {
    let mut v = vec![];
    let mut i = 1;

    match p.parent() {
        Some(parent_p) => {
            let parent_dir = make_parent_dir(parent_p.to_path_buf());
            v.push(parent_dir);
        }
        None => {}
    }
    for entry in fs::read_dir(p)? {
        let entry = entry?;
        v.push(make_entry(i, entry));
        i = i + 1;
    }
    Ok(v)
}

pub fn start() {
    let mut stdout = stdout().into_raw_mode().unwrap();
    let mut stdin = stdin().bytes();

    println!("{}", clear::All);
    println!("{}", cursor::Goto(1, 1));

    let path_buf = current_dir().unwrap();

    println!(
        " {red}{}{reset}",
        path_buf.display(),
        red = color::Bg(color::Magenta),
        reset = color::Bg(color::Reset)
    );

    println!("{}", cursor::Goto(1, 3));

    let entry_v = push_entries(&path_buf).unwrap();
    for (i, entry) in entry_v.iter().enumerate() {
        print!("{}", cursor::Goto(3, (i + 3).try_into().unwrap()));
        println!("{}", entry.file_name);
    }

    let len = &entry_v.len();

    write!(
        stdout,
        "{}{}>{}",
        cursor::Hide,
        cursor::Goto(1, 4),
        cursor::Left(1)
    );
    stdout.flush().unwrap();

    loop {
        let (x, y) = stdout.cursor_pos().unwrap();
        let input = stdin.next();

        if let Some(Ok(key)) = input {
            match key as char {
                'j' => {
                    if y > *len as u16 + 1 {
                        continue;
                    };
                    write!(stdout, " {}\n>{}", cursor::Left(1), cursor::Left(1)).unwrap();
                    stdout.flush().unwrap();
                }

                'k' => {
                    if y <= 2 {
                        continue;
                    }
                    write!(
                        stdout,
                        " {}{}>{}",
                        cursor::Up(1),
                        cursor::Left(1),
                        cursor::Left(2)
                    )
                    .unwrap();
                    stdout.flush().unwrap();
                }
                _ => {
                    print!("{}{}", cursor::Goto(1, 1), clear::All);
                    print!("{}", cursor::Show);
                    break;
                }
            }
        }
    }
}

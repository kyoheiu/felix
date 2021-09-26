use super::config::Colorname;
use super::config::Config;
use dirs;
use std::fs;
use std::io::Error;
use std::path::PathBuf;
use std::process::Command;
use termion::{color, cursor, style};

pub const STARTING_POINT: u16 = 3;
pub const SEARCH_EMOJI: char = '\u{1F50D}';
pub const CONFIG_FILE: &str = "fm/config.toml";
pub const TRUSH: &str = "fm/trash";

#[derive(Copy, Clone, Debug)]
pub enum FileType {
    Directory,
    File,
}

pub struct EntryInfo {
    pub file_path: std::path::PathBuf,
    pub file_name: String,
    pub file_type: FileType,
}

impl EntryInfo {
    pub fn open_file(&self, config: &Config) {
        let path = &self.file_path;
        //todo: have to deal with files like `.gitignore`
        let ext_map = &config.exec;
        let extention = path.extension();
        let default = ext_map.get("default").unwrap();
        match extention {
            Some(extention) => {
                let ext = extention.to_os_string().into_string().unwrap();
                match ext_map.get(&ext) {
                    Some(exec) => {
                        let mut ex = Command::new(exec);
                        ex.arg(path).status().expect("failed");
                    }
                    None => {
                        let mut ex = Command::new(default);
                        ex.arg(path).status().expect("failed");
                    }
                }
            }

            None => {
                let mut ex = Command::new(default);
                ex.arg(path).status().expect("failed");
            }
        }
    }

    fn print(&self, config: &Config) {
        match self.file_type {
            FileType::File => match config.color.file_fg {
                Colorname::AnsiValue(n) => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::AnsiValue(n)),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Black => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Black),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Blue => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Blue),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Cyan => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Cyan),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Green => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Green),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightBlack => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightBlack),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightBlue => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightBlue),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightCyan => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightCyan),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightGreen => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightGreen),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightMagenta => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightMagenta),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightRed => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightRed),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightWhite => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightWhite),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightYellow => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightYellow),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Magenta => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Magenta),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Red => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Red),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Rgb(x, y, z) => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Rgb(x, y, z)),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::White => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::White),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Yellow => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Yellow),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
            },
            FileType::Directory => match config.color.dir_fg {
                Colorname::AnsiValue(n) => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::AnsiValue(n)),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Black => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Black),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Blue => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Blue),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Cyan => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Cyan),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Green => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Green),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightBlack => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightBlack),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightBlue => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightBlue),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightCyan => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightCyan),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightGreen => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightGreen),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightMagenta => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightMagenta),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightRed => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightRed),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightWhite => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightWhite),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::LightYellow => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::LightYellow),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Magenta => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Magenta),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Red => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Red),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Rgb(x, y, z) => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Rgb(x, y, z)),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::White => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::White),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
                Colorname::Yellow => {
                    print!(
                        "{}{}{}",
                        color::Fg(color::Yellow),
                        &self.file_name,
                        color::Fg(color::Reset)
                    );
                }
            },
        }
    }
}

fn make_parent_dir(p: PathBuf) -> EntryInfo {
    return EntryInfo {
        file_path: p.to_path_buf(),
        file_name: String::from("../"),
        file_type: FileType::Directory,
    };
}

fn make_entry(dir: fs::DirEntry) -> EntryInfo {
    return EntryInfo {
        file_path: dir.path(),
        //todo: I have no idea what I'm doing
        file_name: dir
            .path()
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap(),
        file_type: if dir.path().is_file() {
            FileType::File
        } else {
            FileType::Directory
        },
    };
}

pub fn push_entries(p: &PathBuf) -> Result<Vec<EntryInfo>, Error> {
    let mut dir_v = vec![];
    let mut file_v = vec![];

    match p.parent() {
        Some(parent_p) => {
            let parent_dir = make_parent_dir(parent_p.to_path_buf());
            dir_v.push(parent_dir);
        }
        None => {}
    }
    for entry in fs::read_dir(p)? {
        let e = entry?;
        let entry = make_entry(e);
        match entry.file_type {
            FileType::File => file_v.push(entry),
            FileType::Directory => dir_v.push(entry),
        }
    }
    dir_v.sort_by_key(|entry| entry.file_name.clone());
    file_v.sort_by_key(|entry| entry.file_name.clone());
    dir_v.append(&mut file_v);
    Ok(dir_v)
}

pub fn make_config() -> std::io::Result<()> {
    let config_dir = dirs::config_dir().unwrap();
    let config_file = config_dir.join(PathBuf::from(CONFIG_FILE));
    let trush_dir = config_dir.join(PathBuf::from(TRUSH));

    if !trush_dir.exists() {
        fs::create_dir_all(trush_dir)?;
    }

    if !config_file.exists() {
        fs::File::create(config_file)?;
    }

    Ok(())
}

pub fn list_up(config: &Config, p: &PathBuf, v: &std::vec::Vec<EntryInfo>, skip_number: u16) {
    //Show current directory path
    println!(
        " {}{}{}{}{}{}{}",
        style::Bold,
        color::Bg(color::Cyan),
        color::Fg(color::Black),
        p.display(),
        style::Reset,
        color::Bg(color::Reset),
        color::Fg(color::Reset)
    );

    //Show filter emoji and space
    print!("{}{}", cursor::Goto(2, 2), SEARCH_EMOJI);

    let (_, row) = termion::terminal_size().unwrap();
    let len = v.len();

    //if lists exceeds max-row
    if row > STARTING_POINT - 1 && v.len() > (row - STARTING_POINT) as usize - 1 {
        let mut row_count = 0;
        for (i, entry) in v.iter().enumerate() {
            let i = i as u16;

            if i < skip_number {
                continue;
            }

            print!("{}", cursor::Goto(3, i + STARTING_POINT - skip_number));

            if row_count == row - STARTING_POINT {
                print!(
                    "  {}{}{}lines {}-{}({}){}{}",
                    cursor::Left(2),
                    color::Bg(color::LightWhite),
                    color::Fg(color::Black),
                    skip_number,
                    row - STARTING_POINT + skip_number,
                    len,
                    color::Bg(color::Reset),
                    color::Fg(color::Reset)
                );
                break;
            } else {
                entry.print(config);
                row_count += 1;
            }
        }
    } else {
        for (i, entry) in v.iter().enumerate() {
            let i = i as u16;
            print!("{}", cursor::Goto(3, i + STARTING_POINT));
            entry.print(config);
        }
    }
}

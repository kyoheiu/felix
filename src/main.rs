mod config;
mod errors;
mod functions;
mod help;
mod nums;
mod op;
mod run;
mod session;
mod state;

use std::path::PathBuf;

fn main() -> Result<(), errors::FxError> {
    let args: Vec<String> = std::env::args().collect();
    let len = args.len();
    match len {
        1 => {
            if run::run(
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                false,
            )
            .is_err()
            {
                eprintln!("Cannot read the current directory.");
            }
        }

        2 => match args[1].as_str() {
            "-h" | "--help" => {
                print!("{}", help::HELP);
            }
            "-v" | "--version" => {
                functions::check_version()?;
            }
            "-l" | "--log" => {
                if run::run(
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    true,
                )
                .is_err()
                {
                    eprintln!("Cannot read the current directory.");
                }
            }
            _ => {
                if run::run(PathBuf::from(&args[1]), false).is_err() {
                    eprintln!("Cannot read the target directory.");
                }
            }
        },
        3 => {
            if (args[1] == "-l" || args[1] == "--log")
                && run::run(PathBuf::from(&args[2]), true).is_err()
            {
                eprintln!("Cannot read the current directory.");
            }
        }
        _ => print!("{}", help::HELP),
    }
    Ok(())
}

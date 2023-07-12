mod config;
mod errors;
mod functions;
mod help;
mod shell;
mod layout;
mod magic_image;
mod magic_packed;
mod nums;
mod op;
mod run;
mod session;
mod state;
mod term;

use std::path::PathBuf;

fn main() -> Result<(), errors::FxError> {
    let args: Vec<String> = std::env::args().collect();
    let len = args.len();
    match len {
        1 => {
            if let Err(e) = run::run(
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                false,
            ) {
                eprintln!("{}", e);
            }
        }

        2 => match args[1].as_str() {
            "-h" | "--help" => {
                print!("{}", help::HELP);
            }
            "-l" | "--log" => {
                if let Err(e) = run::run(
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    true,
                ) {
                    eprintln!("{}", e);
                }
            }
            "--init" => {
                print!("{}", shell::INTEGRATION_CODE);
            }
            _ => {
                if let Err(e) = run::run(PathBuf::from(&args[1]), false) {
                    eprintln!("{}", e);
                }
            }
        },
        3 => {
            if args[1] == "-l" || args[1] == "--log" {
                if let Err(e) = run::run(PathBuf::from(&args[2]), true) {
                    eprintln!("{}", e);
                }
            } else {
                print!("{}", help::HELP);
            }
        }
        _ => print!("{}", help::HELP),
    }
    Ok(())
}

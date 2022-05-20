mod config;
mod errors;
mod functions;
mod help;
mod nums;
mod run;
mod session;
mod state;

fn main() -> Result<(), errors::FxError> {
    let args: Vec<String> = std::env::args().collect();
    let len = args.len();
    match len {
        1 => {
            if let Err(e) = run::run(
                std::env::current_dir()
                    .unwrap_or_else(|_| panic!("Cannot access current directoy.")),
                false,
            ) {
                eprintln!("{}", e);
            }
        }

        2 => match args[1].as_str() {
            "-h" | "--help" => {
                print!("{}", help::HELP);
            }
            "-v" | "--version" => {
                let output = std::process::Command::new("cargo")
                    .args(["search", "felix", "--limit", "1"])
                    .output()?
                    .stdout;
                if !output.is_empty() {
                    if let Ok(ver) = std::str::from_utf8(&output) {
                        let latest: String =
                            ver.chars().skip(9).take_while(|x| *x != '\"').collect();
                        let current = env!("CARGO_PKG_VERSION");
                        if latest != current {
                            println!("felix v{current}: Latest version is {latest}.");
                        } else {
                            println!("felix v{current}: Up to date.");
                        }
                    }
                } else {
                    println!("Cannot fetch the latest version: Check your internet connection.");
                }
            }
            // "-l" | "--log" => {
            //     if let Err(e) = run::run(
            //         std::env::current_dir()
            //             .unwrap_or_else(|_| panic!("Cannot access current directoy.")),
            //         true,
            //     ) {
            //         eprintln!("{}", e);
            //     }
            // }
            _ => {
                if let Err(e) = run::run(std::path::PathBuf::from(&args[1]), false) {
                    eprintln!("{}", e);
                }
            }
        },
        _ => print!("{}", help::HELP),
    }
    Ok(())
}

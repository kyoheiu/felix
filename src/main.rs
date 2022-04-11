mod config;
mod errors;
mod functions;
mod help;
mod nums;
mod run;
mod session;
mod state;

fn main() -> Result<(), errors::MyError> {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    let len = args.len();
    match len {
        1 => {
            if let Err(e) = run::run(
                std::env::current_dir()
                    .unwrap_or_else(|_| panic!("Cannot access current directoy.")),
            ) {
                println!("{}", e);
            }
        }

        2 => match args[1].as_str() {
            "-h" | "--help" => {
                print!("{}", help::HELP);
            }
            _ => {
                if let Err(e) = run::run(std::path::PathBuf::from(&args[1])) {
                    println!("{}", e);
                }
            }
        },
        _ => print!("{}", help::HELP),
    }
    Ok(())
}

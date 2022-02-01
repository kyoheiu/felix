mod config;
mod functions;
mod help;
mod nums;
mod run;
mod state;
mod session;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let len = args.len();
    match len {
        1 => run::run(
            std::env::current_dir().unwrap_or_else(|_| panic!("cannot access current directory.")),
        ),
        2 => run::run(std::path::PathBuf::from(&args[1])),
        _ => print!("{}", help::HELP),
    }
}

mod config;
mod entry;
mod functions;
mod help;
mod nums;
mod run;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        run::run();
    } else {
        print!("{}", help::HELP);
    }
}

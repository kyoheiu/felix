mod config;
mod functions;
mod help;
mod items;
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

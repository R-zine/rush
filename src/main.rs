mod executor;
mod history;
mod parser;
mod shell;

fn main() {
    if let Err(error) = shell::run() {
        eprintln!("rush: {error}");
        std::process::exit(1);
    }
}

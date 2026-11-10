//! Standalone engine process; all entry-point policy lives outside analyzers.
mod entrypoints;

fn main() {
    match entrypoints::run() {
        Ok(status) => std::process::exit(status),
        Err(error) => {
            eprintln!("{error:#}");
            std::process::exit(2);
        }
    }
}

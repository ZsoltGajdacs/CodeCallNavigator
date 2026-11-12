//! Standalone engine process; all entry-point policy lives outside analyzers.
mod entrypoints;

fn main() {
    entrypoints::main();
}

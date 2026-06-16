mod action;
mod app;
mod cli;
mod config;
mod error;
mod executor;
mod mode;
mod models;
mod storage;
mod tui;
mod ui;

use app::App;
use std::io;
use tui::{init_terminal, restore_terminal};

fn main() -> io::Result<()> {
    // CLI mode: if a subcommand is given, handle it and exit
    if let Some(exit_code) = cli::run_cli() {
        std::process::exit(exit_code);
    }

    let mut terminal = init_terminal()?;
    let mut app = App::new();

    let result = app.run(&mut terminal);

    restore_terminal()?;

    if let Err(e) = &result {
        eprintln!("Error: {}", e);
    }

    result
}

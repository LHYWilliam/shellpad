mod app;
mod config;
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
    let mut terminal = init_terminal()?;
    let mut app = App::new();

    let result = app.run(&mut terminal);

    restore_terminal()?;

    if let Err(e) = &result {
        eprintln!("Error: {}", e);
    }

    result
}

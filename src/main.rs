use shellpad::app::App;
use shellpad::tui::{init_terminal, restore_terminal};
use std::io;

fn main() -> io::Result<()> {
    // CLI mode
    if let Some(exit_code) = shellpad::cli::run_cli() {
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

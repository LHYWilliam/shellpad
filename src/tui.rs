use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

/// The TUI terminal instance.
pub type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

/// Initialize the terminal into raw mode + alternate screen.
pub fn init_terminal() -> io::Result<TuiTerminal> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

/// Restore the terminal to normal mode.
pub fn restore_terminal() -> io::Result<()> {
    execute!(io::stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

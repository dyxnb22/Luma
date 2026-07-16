use crossterm::event::{
    DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};

static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);

/// RAII guard: always restore terminal on drop (normal exit, Ctrl-C path, drop on unwind).
pub struct TerminalGuard {
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
}

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        Self::activate_terminal()?;
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal: Some(terminal),
        })
    }

    fn activate_terminal() -> io::Result<()> {
        if TERMINAL_ACTIVE.load(Ordering::SeqCst) {
            return Ok(());
        }
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange
        )?;
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn suspend(&mut self) {
        if self.terminal.is_some() {
            restore_terminal();
            self.terminal = None;
        }
    }

    pub fn resume(&mut self) -> io::Result<()> {
        if self.terminal.is_some() {
            return Ok(());
        }
        Self::activate_terminal()?;
        let backend = CrosstermBackend::new(io::stdout());
        self.terminal = Some(Terminal::new(backend)?);
        Ok(())
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        self.terminal
            .as_mut()
            .expect("terminal not active — call resume() after suspend()")
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

pub fn restore_terminal() {
    if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(
            stdout,
            LeaveAlternateScreen,
            DisableMouseCapture,
            DisableFocusChange
        );
    }
}

pub fn install_panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_is_idempotent_when_inactive() {
        TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
        restore_terminal();
        restore_terminal();
    }
}

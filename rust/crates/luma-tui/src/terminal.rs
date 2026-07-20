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
    terminal: Terminal<CrosstermBackend<Stdout>>,
    suspended: bool,
}

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(err) = execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange
        ) {
            cleanup_terminal_after_partial_enter();
            return Err(err);
        }
        let backend = CrosstermBackend::new(stdout);
        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(err) => {
                cleanup_terminal_after_partial_enter();
                return Err(err);
            }
        };
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);
        Ok(Self {
            terminal,
            suspended: false,
        })
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }

    /// Leave raw mode / alternate screen so an interactive child can use the terminal.
    pub fn suspend(&mut self) -> io::Result<()> {
        if self.suspended {
            return Ok(());
        }
        restore_terminal()?;
        self.suspended = true;
        Ok(())
    }

    /// Re-enter TUI terminal state after an interactive child exits.
    pub fn resume(&mut self) -> io::Result<()> {
        if !self.suspended {
            return Ok(());
        }
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(err) = execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableFocusChange
        ) {
            cleanup_terminal_after_partial_enter();
            return Err(err);
        }
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);
        if let Err(err) = self.terminal.clear() {
            let _ = restore_terminal();
            return Err(err);
        }
        self.suspended = false;
        Ok(())
    }

    pub fn is_suspended(&self) -> bool {
        self.suspended
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if !self.suspended {
            let _ = restore_terminal();
        } else {
            // A child normally restored the terminal, but resume can fail after partially
            // re-entering it. Retry cleanup when the active flag says there is still work.
            if TERMINAL_ACTIVE.load(Ordering::SeqCst) {
                let _ = restore_terminal();
            } else {
                TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
            }
        }
    }
}

pub fn restore_terminal() -> io::Result<()> {
    if !TERMINAL_ACTIVE.load(Ordering::SeqCst) {
        return Ok(());
    }
    let raw_result = disable_raw_mode();
    let mut stdout = io::stdout();
    let screen_result = execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableFocusChange
    );
    if raw_result.is_ok() && screen_result.is_ok() {
        TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
    }
    raw_result.and(screen_result)
}

/// Best-effort cleanup for a partially completed terminal enter. The active flag is not set
/// until the whole enter sequence succeeds, so this path cannot rely on `restore_terminal`.
fn cleanup_terminal_after_partial_enter() {
    let _ = disable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableFocusChange
    );
}

pub fn install_panic_hook() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_is_idempotent_when_inactive() {
        TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
        restore_terminal().unwrap();
        restore_terminal().unwrap();
    }
}

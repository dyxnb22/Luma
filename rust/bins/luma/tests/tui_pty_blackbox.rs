//! Real PTY regression for the crossterm input path.
//!
//! This intentionally sits above reducer/render unit tests: `/usr/bin/script`
//! allocates a real macOS PTY, the production `luma tui` binary enters raw mode,
//! and `vt100` reconstructs the resulting ANSI screen. All persistence and HOME
//! paths point at one test-only temporary directory.

#![cfg(target_os = "macos")]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tempfile::tempdir;

const TERMINAL_ROWS: u16 = 30;
const TERMINAL_COLS: u16 = 100;
const WAIT_LIMIT: Duration = Duration::from_secs(10);

fn luma_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_luma"))
}

#[derive(Default)]
struct CapturedOutput {
    bytes: Vec<u8>,
    closed: bool,
}

struct OutputFeed {
    state: Mutex<CapturedOutput>,
    changed: Condvar,
}

impl OutputFeed {
    fn new() -> Self {
        Self {
            state: Mutex::new(CapturedOutput::default()),
            changed: Condvar::new(),
        }
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct PtyTui {
    child: Child,
    stdin: ChildStdin,
    stdout: Arc<OutputFeed>,
    stderr: Arc<Mutex<Vec<u8>>>,
    stdout_reader: Option<JoinHandle<()>>,
    stderr_reader: Option<JoinHandle<()>>,
}

impl PtyTui {
    fn spawn(root: &Path) -> Self {
        Self::spawn_with_fake(root, false)
    }

    fn spawn_with_fake(root: &Path, fake_enabled: bool) -> Self {
        let support = root.join("support");
        let logs = root.join("logs");
        let home = root.join("home");
        let tmp = root.join("tmp");
        for directory in [&support, &logs, &home, &tmp] {
            fs::create_dir_all(directory).expect("create isolated PTY test directory");
        }
        write_isolated_settings(&support, fake_enabled);

        // `script` owns the PTY. `stty` gives both crossterm and the ANSI parser
        // deterministic geometry even though the Rust test runner itself has no TTY.
        let mut child = Command::new("/usr/bin/script")
            .args([
                "-q",
                "/dev/null",
                "/bin/sh",
                "-c",
                "stty rows 30 cols 100; exec \"$1\" tui",
                "luma-pty-test",
            ])
            .arg(luma_bin())
            .env_clear()
            .env("HOME", &home)
            .env("TMPDIR", &tmp)
            .env("LUMA_NEXT_SUPPORT_DIR", &support)
            .env("LUMA_NEXT_LOGS_DIR", &logs)
            .env("LUMA_TUI_ASCII", "1")
            .env("HOMEBREW_NO_AUTO_UPDATE", "1")
            .env("TERM", "xterm-256color")
            .env("LANG", "en_US.UTF-8")
            .env("LC_ALL", "en_US.UTF-8")
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("SHELL", "/bin/sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn luma inside a real PTY");

        let stdin = child.stdin.take().expect("PTY stdin");
        let mut stdout = child.stdout.take().expect("PTY stdout");
        let mut stderr = child.stderr.take().expect("PTY stderr");

        let stdout_feed = Arc::new(OutputFeed::new());
        let stdout_for_thread = stdout_feed.clone();
        let stdout_reader = std::thread::spawn(move || {
            let mut chunk = [0_u8; 8192];
            loop {
                match stdout.read(&mut chunk) {
                    Ok(0) | Err(_) => {
                        let mut output = lock_unpoisoned(&stdout_for_thread.state);
                        output.closed = true;
                        stdout_for_thread.changed.notify_all();
                        break;
                    }
                    Ok(read) => {
                        let mut output = lock_unpoisoned(&stdout_for_thread.state);
                        output.bytes.extend_from_slice(&chunk[..read]);
                        stdout_for_thread.changed.notify_all();
                    }
                }
            }
        });

        let stderr_bytes = Arc::new(Mutex::new(Vec::new()));
        let stderr_for_thread = stderr_bytes.clone();
        let stderr_reader = std::thread::spawn(move || {
            let mut chunk = [0_u8; 4096];
            while let Ok(read) = stderr.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                lock_unpoisoned(&stderr_for_thread).extend_from_slice(&chunk[..read]);
            }
        });

        Self {
            child,
            stdin,
            stdout: stdout_feed,
            stderr: stderr_bytes,
            stdout_reader: Some(stdout_reader),
            stderr_reader: Some(stderr_reader),
        }
    }

    fn write(&mut self, bytes: &[u8]) {
        self.stdin.write_all(bytes).expect("write PTY input");
        self.stdin.flush().expect("flush PTY input");
    }

    fn mark(&self) -> usize {
        lock_unpoisoned(&self.stdout.state).bytes.len()
    }

    fn wait_for_screen(
        &self,
        after: usize,
        description: &str,
        predicate: impl Fn(&vt100::Screen) -> bool,
    ) -> vt100::Screen {
        let deadline = Instant::now() + WAIT_LIMIT;
        let mut output = lock_unpoisoned(&self.stdout.state);
        loop {
            let screen = parse_screen(&output.bytes);
            if output.bytes.len() > after && predicate(&screen) {
                return screen;
            }
            let now = Instant::now();
            if now >= deadline || output.closed {
                let stderr = String::from_utf8_lossy(&lock_unpoisoned(&self.stderr)).into_owned();
                let contents = screen.contents();
                let captured = output.bytes.len();
                drop(output);
                panic!(
                    "timed out waiting for {description} ({captured} PTY bytes captured)\n\
                     --- screen ---\n{contents}\n--- stderr ---\n{stderr}"
                );
            }
            let remaining = deadline.saturating_duration_since(now);
            let (next, _) = self
                .stdout
                .changed
                .wait_timeout(output, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            output = next;
        }
    }

    fn wait_for_exit(&mut self) {
        let deadline = Instant::now() + WAIT_LIMIT;
        let mut output = lock_unpoisoned(&self.stdout.state);
        while !output.closed {
            let now = Instant::now();
            if now >= deadline {
                let contents = parse_screen(&output.bytes).contents();
                drop(output);
                panic!("timed out waiting for TUI exit\n--- screen ---\n{contents}");
            }
            let remaining = deadline.saturating_duration_since(now);
            let (next, _) = self
                .stdout
                .changed
                .wait_timeout(output, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            output = next;
        }
        drop(output);

        let status = self.child.wait().expect("wait for PTY child");
        assert!(status.success(), "script wrapper exited with {status}");
        self.join_readers();
    }

    fn join_readers(&mut self) {
        if let Some(reader) = self.stdout_reader.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.stderr_reader.take() {
            let _ = reader.join();
        }
    }
}

impl Drop for PtyTui {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            // This is only the isolated test wrapper and its child. Never target
            // a pre-existing Luma process.
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        self.join_readers();
    }
}

fn parse_screen(bytes: &[u8]) -> vt100::Screen {
    let mut parser = vt100::Parser::new(TERMINAL_ROWS, TERMINAL_COLS, 0);
    parser.process(bytes);
    parser.screen().clone()
}

fn write_isolated_settings(support: &Path, fake_enabled: bool) {
    let settings = format!(
        r#"schema_version = 1
settings_version = 1
projects_roots = []
imported_projects = []
clipboard_retention_days = 30
secrets_idle_lock_secs = 300
hub_windows_max = 7

[enabled_modules]
"luma.apps" = false
"luma.calculator" = false
"luma.clipboard" = false
"luma.command_recipes" = false
"luma.databases" = false
"luma.downloads" = false
"luma.fake" = {fake_enabled}
"luma.git" = false
"luma.ocr" = false
"luma.packages" = false
"luma.projects" = false
"luma.proxy" = false
"luma.quicklinks" = false
"luma.records" = false
"luma.renewals" = false
"luma.runtime" = false
"luma.secrets" = false
"luma.shell_history" = false
"luma.shortcuts" = false
"luma.snippets" = false
"luma.ssh" = false
"luma.timers" = false
"luma.windows" = false
"luma.wordbook" = false
"#
    );
    fs::write(support.join("settings.toml"), settings).expect("write isolated settings");
}

#[test]
fn real_pty_preserves_keyboard_first_prompt_and_navigation() {
    let root = tempdir().expect("temporary PTY test root");
    let mut tui = PtyTui::spawn(root.path());

    let ready = tui.wait_for_screen(0, "initial TUI frame", |screen| {
        let contents = screen.contents();
        contents.contains("LUMA") && contents.contains("GLOBAL SEARCH")
    });
    assert_eq!(
        ready.cell(0, 1).map(vt100::Cell::fgcolor),
        Some(vt100::Color::Rgb(112, 222, 194)),
        "prompt border should start focused"
    );

    // One write exercises crossterm's real raw-mode decoding under rapid input.
    let marker = tui.mark();
    tui.write("1abc02快速中文🙂e\u{301}".as_bytes());
    let mut expected = "1abc02快速中文🙂e\u{301}".to_string();
    tui.wait_for_screen(marker, "rapid ASCII/CJK/emoji/combining input", |screen| {
        screen.contents().contains(&expected)
    });

    // Backspace is grapheme-aware: one key removes `e` and its combining mark.
    let marker = tui.mark();
    tui.write(b"\x7f");
    let without_combining = "1abc02快速中文🙂";
    tui.wait_for_screen(marker, "grapheme-aware Backspace", |screen| {
        let contents = screen.contents();
        contents.contains(without_combining) && !contents.contains(&expected)
    });
    let marker = tui.mark();
    tui.write("e\u{301}".as_bytes());
    tui.wait_for_screen(marker, "retyping a combining grapheme", |screen| {
        screen.contents().contains(&expected)
    });

    // Home/End and both horizontal arrows are validated by where a new glyph lands.
    let marker = tui.mark();
    tui.write(b"\x1b[H^");
    tui.write(b"\x1b[F$");
    expected.insert(0, '^');
    expected.push('$');
    tui.wait_for_screen(marker, "Home and End cursor placement", |screen| {
        screen.contents().contains(&expected)
    });

    let marker = tui.mark();
    tui.write(b"\x1b[DX\x1b[F");
    expected.insert(expected.len() - 1, 'X');
    tui.wait_for_screen(marker, "left-arrow cursor placement", |screen| {
        screen.contents().contains(&expected)
    });

    let marker = tui.mark();
    tui.write(b"\x1b[H\x1b[CY\x1b[F");
    expected.insert(1, 'Y');
    tui.wait_for_screen(marker, "right-arrow cursor placement", |screen| {
        screen.contents().contains(&expected)
    });

    let marker = tui.mark();
    tui.write(b"\t");
    tui.wait_for_screen(marker, "Tab focus change", |screen| {
        screen.cell(0, 1).map(vt100::Cell::fgcolor) == Some(vt100::Color::Rgb(50, 55, 75))
    });
    let marker = tui.mark();
    tui.write(b"Z");
    expected.push('Z');
    let prompt = tui.wait_for_screen(marker, "typing after Tab restores prompt focus", |screen| {
        screen.contents().contains(&expected)
    });
    assert_eq!(
        prompt.cell(0, 1).map(vt100::Cell::fgcolor),
        Some(vt100::Color::Rgb(112, 222, 194))
    );

    // Bracketed paste must arrive as one Paste event and retain normal Unicode.
    let marker = tui.mark();
    tui.write(b"\x15");
    tui.wait_for_screen(marker, "Ctrl-U clear", |screen| {
        screen
            .contents()
            .contains("Search everything or type / for commands")
    });
    let marker = tui.mark();
    tui.write("\x1b[200~粘贴🙂e\u{301}\x1b[201~".as_bytes());
    tui.wait_for_screen(marker, "bracketed Unicode paste", |screen| {
        screen.contents().contains("粘贴🙂e\u{301}")
    });

    // Enter opens a local overlay. PageDown/PageUp must move it and Esc must
    // restore the exact prompt without causing an external action.
    let marker = tui.mark();
    tui.write(b"\x15/help\r");
    let help_top = tui.wait_for_screen(marker, "Enter opening /help", |screen| {
        let contents = screen.contents();
        contents.contains(" HELP ") && contents.contains("Fn+")
    });
    let help_top_contents = help_top.contents();

    let marker = tui.mark();
    tui.write(b"\x1b[B");
    tui.wait_for_screen(marker, "down-arrow scrolling help", |screen| {
        let contents = screen.contents();
        contents.contains(" HELP ") && contents != help_top_contents
    });
    let marker = tui.mark();
    tui.write(b"\x1b[A");
    tui.wait_for_screen(marker, "up-arrow restoring help", |screen| {
        let contents = screen.contents();
        contents.contains("Enter opens a bare trigger") && contents.contains("Workbench commands:")
    });

    let marker = tui.mark();
    tui.write(b"\x1b[6~");
    let help_down = tui.wait_for_screen(marker, "PageDown scrolling help", |screen| {
        let contents = screen.contents();
        contents.contains(" HELP ") && contents != help_top_contents
    });
    assert_ne!(help_down.contents(), help_top_contents);

    let marker = tui.mark();
    tui.write(b"\x1b[5~");
    tui.wait_for_screen(marker, "PageUp restoring help", |screen| {
        let contents = screen.contents();
        contents.contains("Enter opens a bare trigger") && contents.contains("Workbench commands:")
    });

    let marker = tui.mark();
    tui.write(b"\x1b");
    tui.wait_for_screen(marker, "Esc restoring the search prompt", |screen| {
        let contents = screen.contents();
        contents.contains("/help") && contents.contains("COMMAND") && !contents.contains("Fn+")
    });

    // With every module disabled, a global query finishes deterministically
    // without touching system integrations. Ctrl-K then has an observable,
    // honest no-selection outcome.
    let marker = tui.mark();
    tui.write(b"\x15ctrlk-probe");
    tui.wait_for_screen(marker, "isolated empty search completion", |screen| {
        screen.contents().contains("No results")
    });
    let marker = tui.mark();
    tui.write(b"\x0b");
    tui.wait_for_screen(marker, "Ctrl-K action picker request", |screen| {
        screen.contents().contains("no result selected")
    });

    // Ctrl-C is intentionally two-stage: first shows the quit confirmation,
    // second confirms it. This also proves raw-mode teardown reaches script.
    let marker = tui.mark();
    tui.write(b"\x03");
    tui.wait_for_screen(marker, "first Ctrl-C quit confirmation", |screen| {
        let contents = screen.contents();
        contents.contains("Quit Luma?") && contents.contains("Enter confirm")
    });
    tui.write(b"\x03");
    tui.wait_for_exit();
}

#[test]
fn real_pty_runs_the_test_only_result_action_matrix_without_external_effects() {
    let root = tempdir().expect("temporary PTY test root");
    let mut tui = PtyTui::spawn_with_fake(root.path(), true);
    tui.wait_for_screen(0, "focused initial input", |screen| {
        screen.contents().contains("GLOBAL SEARCH · INPUT")
    });

    let marker = tui.mark();
    tui.write(b"/fake keyboard");
    tui.wait_for_screen(marker, "test-only result", |screen| {
        screen.contents().contains("Echo: keyboard")
    });

    let marker = tui.mark();
    tui.write(b"\x0b");
    tui.wait_for_screen(marker, "numbered action picker", |screen| {
        let contents = screen.contents();
        contents.contains(" ACTIONS ") && contents.contains("[1] Open")
    });

    let marker = tui.mark();
    tui.write(b"1");
    tui.wait_for_screen(marker, "digit action execution", |screen| {
        screen.contents().contains("performed open")
    });

    let marker = tui.mark();
    tui.write(b"\x03");
    tui.wait_for_screen(marker, "quit confirmation", |screen| {
        screen.contents().contains("Quit Luma?")
    });
    tui.write(b"\x03");
    tui.wait_for_exit();
}

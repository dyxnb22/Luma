//! OS PTY sessions via `portable-pty` for embedded in-TUI terminals.

use luma_application::{
    EmbeddedPtyError, EmbeddedPtyEvent, EmbeddedPtyPort, EmbeddedPtySession, EmbeddedPtySize,
    EmbeddedPtySpawnRequest,
};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const OUTPUT_CHANNEL_CAPACITY: usize = 256;
const READ_BUF: usize = 8192;

/// Cross-platform embedded PTY factory (named `Mac*` to match other adapters).
pub struct MacEmbeddedPty;

impl Default for MacEmbeddedPty {
    fn default() -> Self {
        Self
    }
}

impl MacEmbeddedPty {
    pub fn new() -> Self {
        Self
    }
}

struct LiveSession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    killed: Arc<AtomicBool>,
}

struct SharedWriter {
    inner: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| std::io::Error::other("embedded pty writer lock poisoned"))?;
        guard.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| std::io::Error::other("embedded pty writer lock poisoned"))?;
        guard.flush()
    }
}

impl EmbeddedPtySession for LiveSession {
    fn try_clone_writer(&self) -> Result<Box<dyn Write + Send>, EmbeddedPtyError> {
        Ok(Box::new(SharedWriter {
            inner: Arc::clone(&self.writer),
        }))
    }

    fn resize(&self, size: EmbeddedPtySize) -> Result<(), EmbeddedPtyError> {
        let master = self.master.lock().map_err(|_| {
            EmbeddedPtyError::Unavailable("embedded pty master lock poisoned".into())
        })?;
        master
            .resize(PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| EmbeddedPtyError::Unavailable(format!("pty resize: {err}")))
    }

    fn kill(&self) -> Result<(), EmbeddedPtyError> {
        self.killed.store(true, Ordering::SeqCst);
        let mut child = self.child.lock().map_err(|_| {
            EmbeddedPtyError::Unavailable("embedded pty child lock poisoned".into())
        })?;
        #[cfg(unix)]
        {
            if let Some(pid) = child.process_id() {
                // Kill the whole process group when the child is a group leader.
                // Fall through to child.kill() for the direct process as well.
                let _ = kill_process_group(pid);
            }
        }
        let _ = child.kill();
        Ok(())
    }
}

impl EmbeddedPtyPort for MacEmbeddedPty {
    fn spawn(
        &self,
        request: EmbeddedPtySpawnRequest,
    ) -> Result<(Box<dyn EmbeddedPtySession>, Receiver<EmbeddedPtyEvent>), EmbeddedPtyError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: request.size.rows,
                cols: request.size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| EmbeddedPtyError::Unavailable(format!("openpty: {err}")))?;

        let mut cmd = CommandBuilder::new(&request.program);
        for arg in &request.args {
            cmd.arg(arg);
        }
        for (key, value) in &request.environment {
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd).map_err(|err| {
            EmbeddedPtyError::Unavailable(format!("spawn {}: {err}", request.program))
        })?;

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| EmbeddedPtyError::Unavailable(format!("clone pty reader: {err}")))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| EmbeddedPtyError::Unavailable(format!("take pty writer: {err}")))?;

        let (tx, rx) = mpsc::sync_channel::<EmbeddedPtyEvent>(OUTPUT_CHANNEL_CAPACITY);
        let killed = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(child));
        let child_for_wait = Arc::clone(&child);
        let tx_out = tx.clone();
        let tx_exit = tx;
        let killed_reader = Arc::clone(&killed);

        // Drop the slave after spawn so only the child retains the slave FD.
        drop(pair.slave);

        thread::Builder::new()
            .name("luma-embedded-pty-reader".into())
            .spawn(move || {
                let mut buf = [0u8; READ_BUF];
                loop {
                    if killed_reader.load(Ordering::SeqCst) {
                        break;
                    }
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if !send_output(&tx_out, buf[..n].to_vec()) {
                                break;
                            }
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
            })
            .map_err(|err| EmbeddedPtyError::Unavailable(format!("spawn reader thread: {err}")))?;

        thread::Builder::new()
            .name("luma-embedded-pty-wait".into())
            .spawn(move || {
                let code = wait_for_exit(&child_for_wait);
                let _ = tx_exit.send(EmbeddedPtyEvent::Exited { code });
            })
            .map_err(|err| EmbeddedPtyError::Unavailable(format!("spawn wait thread: {err}")))?;

        let session = LiveSession {
            master: Mutex::new(pair.master),
            writer: Arc::new(Mutex::new(writer)),
            child,
            killed,
        };
        Ok((Box::new(session), rx))
    }
}

fn send_output(tx: &SyncSender<EmbeddedPtyEvent>, bytes: Vec<u8>) -> bool {
    tx.send(EmbeddedPtyEvent::Output(bytes)).is_ok()
}

#[cfg(unix)]
fn kill_process_group(pid: u32) -> std::io::Result<()> {
    // Negative PID targets the process group. Best-effort; child.kill() follows.
    let rc = unsafe { libc_kill(-(pid as i32), 9) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
unsafe fn libc_kill(pid: i32, sig: i32) -> i32 {
    extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    kill(pid, sig)
}

fn wait_for_exit(child: &Mutex<Box<dyn Child + Send + Sync>>) -> Option<i32> {
    loop {
        let mut guard = match child.lock() {
            Ok(guard) => guard,
            Err(_) => return None,
        };
        match guard.try_wait() {
            Ok(Some(status)) => return Some(status.exit_code() as i32),
            Ok(None) => {
                drop(guard);
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::recv_event_timeout;
    use std::time::Duration;

    #[test]
    fn spawns_shell_and_captures_output() {
        let port = MacEmbeddedPty::new();
        let (session, rx) = match port.spawn(EmbeddedPtySpawnRequest {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), "printf 'hello-pty\\n'; sleep 0.05".into()],
            environment: vec![],
            size: EmbeddedPtySize::new(80, 24),
        }) {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("skipping pty integration test: {err}");
                return;
            }
        };

        let mut collected = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut exit_code = None;
        while std::time::Instant::now() < deadline {
            match recv_event_timeout(&rx, Duration::from_millis(50)) {
                Ok(EmbeddedPtyEvent::Output(bytes)) => collected.extend(bytes),
                Ok(EmbeddedPtyEvent::Exited { code }) => {
                    exit_code = code;
                    // Drain trailing output after exit notification.
                    while let Ok(EmbeddedPtyEvent::Output(bytes)) =
                        recv_event_timeout(&rx, Duration::from_millis(50))
                    {
                        collected.extend(bytes);
                    }
                    break;
                }
                Err(_) => {
                    if exit_code.is_some() {
                        break;
                    }
                }
            }
            if String::from_utf8_lossy(&collected).contains("hello-pty") && exit_code.is_some() {
                break;
            }
        }
        let _ = session.kill();
        let text = String::from_utf8_lossy(&collected);
        assert!(
            text.contains("hello-pty"),
            "expected hello-pty in {text:?}, exit={exit_code:?}"
        );
        assert_eq!(exit_code, Some(0));
    }

    #[test]
    fn kill_terminates_child_process() {
        let port = MacEmbeddedPty::new();
        let (session, rx) = match port.spawn(EmbeddedPtySpawnRequest {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), "sleep 30".into()],
            environment: vec![],
            size: EmbeddedPtySize::new(40, 12),
        }) {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("skipping pty kill test: {err}");
                return;
            }
        };
        session.kill().expect("kill");
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut exited = false;
        while std::time::Instant::now() < deadline {
            match recv_event_timeout(&rx, Duration::from_millis(50)) {
                Ok(EmbeddedPtyEvent::Exited { .. }) => {
                    exited = true;
                    break;
                }
                Ok(EmbeddedPtyEvent::Output(_)) => {}
                Err(_) => {}
            }
        }
        assert!(exited, "expected Exited after kill");
    }
}

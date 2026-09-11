//! Embedded child PTY sessions for in-TUI terminals (SSH Workspace).

use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;

/// Terminal geometry for spawn and resize.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmbeddedPtySize {
    pub cols: u16,
    pub rows: u16,
}

impl EmbeddedPtySize {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
        }
    }
}

/// Spawn request for an embedded interactive program (no shell wrapper).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbeddedPtySpawnRequest {
    pub program: String,
    pub args: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub size: EmbeddedPtySize,
}

/// Events delivered from the PTY reader / child waiter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmbeddedPtyEvent {
    Output(Vec<u8>),
    Exited { code: Option<i32> },
}

#[derive(Debug, Error)]
pub enum EmbeddedPtyError {
    #[error("embedded pty unavailable: {0}")]
    Unavailable(String),
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

/// One live child PTY session.
pub trait EmbeddedPtySession: Send {
    /// Writer for keystrokes / injected command text (no automatic Enter).
    fn try_clone_writer(&self) -> Result<Box<dyn Write + Send>, EmbeddedPtyError>;
    fn resize(&self, size: EmbeddedPtySize) -> Result<(), EmbeddedPtyError>;
    fn kill(&self) -> Result<(), EmbeddedPtyError>;
}

/// Port for creating embedded PTY sessions.
pub trait EmbeddedPtyPort: Send + Sync {
    fn spawn(
        &self,
        request: EmbeddedPtySpawnRequest,
    ) -> Result<(Box<dyn EmbeddedPtySession>, Receiver<EmbeddedPtyEvent>), EmbeddedPtyError>;
}

const FAKE_EVENT_CAPACITY: usize = 64;

/// In-memory PTY double for reducer / unit tests (no OS PTY).
pub struct FakeEmbeddedPty {
    pub spawned: Mutex<Vec<EmbeddedPtySpawnRequest>>,
    /// When set, `spawn` returns this error.
    pub fail_spawn: Mutex<Option<String>>,
    /// Bytes written through session writers, in order.
    pub writes: Arc<Mutex<Vec<u8>>>,
    /// Recorded resize requests.
    pub resizes: Arc<Mutex<Vec<EmbeddedPtySize>>>,
    /// Event injectors handed to tests (one per spawn).
    event_txs: Mutex<Vec<SyncSender<EmbeddedPtyEvent>>>,
    /// Kill calls.
    pub killed: Arc<Mutex<usize>>,
}

impl Default for FakeEmbeddedPty {
    fn default() -> Self {
        Self {
            spawned: Mutex::new(Vec::new()),
            fail_spawn: Mutex::new(None),
            writes: Arc::new(Mutex::new(Vec::new())),
            resizes: Arc::new(Mutex::new(Vec::new())),
            event_txs: Mutex::new(Vec::new()),
            killed: Arc::new(Mutex::new(0)),
        }
    }
}

impl FakeEmbeddedPty {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn push_output(&self, bytes: impl AsRef<[u8]>) {
        let txs = self.event_txs.lock().expect("event_txs");
        if let Some(tx) = txs.last() {
            let _ = tx.send(EmbeddedPtyEvent::Output(bytes.as_ref().to_vec()));
        }
    }

    pub fn push_exit(&self, code: Option<i32>) {
        let txs = self.event_txs.lock().expect("event_txs");
        if let Some(tx) = txs.last() {
            let _ = tx.send(EmbeddedPtyEvent::Exited { code });
        }
    }

    pub fn written_bytes(&self) -> Vec<u8> {
        self.writes.lock().expect("writes").clone()
    }
}

struct FakeSession {
    writes: Arc<Mutex<Vec<u8>>>,
    resizes: Arc<Mutex<Vec<EmbeddedPtySize>>>,
    killed: Arc<Mutex<usize>>,
    exit_tx: SyncSender<EmbeddedPtyEvent>,
}

impl EmbeddedPtySession for FakeSession {
    fn try_clone_writer(&self) -> Result<Box<dyn Write + Send>, EmbeddedPtyError> {
        Ok(Box::new(FakeWriter {
            writes: Arc::clone(&self.writes),
        }))
    }

    fn resize(&self, size: EmbeddedPtySize) -> Result<(), EmbeddedPtyError> {
        self.resizes.lock().expect("resizes").push(size);
        Ok(())
    }

    fn kill(&self) -> Result<(), EmbeddedPtyError> {
        *self.killed.lock().expect("killed") += 1;
        let _ = self.exit_tx.send(EmbeddedPtyEvent::Exited { code: None });
        Ok(())
    }
}

struct FakeWriter {
    writes: Arc<Mutex<Vec<u8>>>,
}

impl Write for FakeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writes.lock().expect("writes").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl EmbeddedPtyPort for FakeEmbeddedPty {
    fn spawn(
        &self,
        request: EmbeddedPtySpawnRequest,
    ) -> Result<(Box<dyn EmbeddedPtySession>, Receiver<EmbeddedPtyEvent>), EmbeddedPtyError> {
        if let Some(message) = self.fail_spawn.lock().expect("fail_spawn").clone() {
            return Err(EmbeddedPtyError::Unavailable(message));
        }
        self.spawned.lock().expect("spawned").push(request);
        let (tx, rx) = mpsc::sync_channel(FAKE_EVENT_CAPACITY);
        self.event_txs.lock().expect("event_txs").push(tx.clone());
        let session = FakeSession {
            writes: Arc::clone(&self.writes),
            resizes: Arc::clone(&self.resizes),
            killed: Arc::clone(&self.killed),
            exit_tx: tx,
        };
        Ok((Box::new(session), rx))
    }
}

/// Drain events with a short timeout (test helper).
pub fn recv_event_timeout(
    rx: &Receiver<EmbeddedPtyEvent>,
    timeout: Duration,
) -> Result<EmbeddedPtyEvent, RecvTimeoutError> {
    rx.recv_timeout(timeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_spawn_records_request_and_accepts_writes() {
        let port = FakeEmbeddedPty::new();
        let request = EmbeddedPtySpawnRequest {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), "true".into()],
            environment: vec![("A".into(), "1".into())],
            size: EmbeddedPtySize::new(80, 24),
        };
        let (session, rx) = port.spawn(request.clone()).expect("spawn");
        assert_eq!(port.spawned.lock().unwrap().len(), 1);
        assert_eq!(port.spawned.lock().unwrap()[0], request);

        let mut writer = session.try_clone_writer().expect("writer");
        writer.write_all(b"ls\n").expect("write");
        assert_eq!(port.written_bytes(), b"ls\n");

        session
            .resize(EmbeddedPtySize::new(120, 40))
            .expect("resize");
        assert_eq!(
            *port.resizes.lock().unwrap(),
            vec![EmbeddedPtySize::new(120, 40)]
        );

        port.push_output(b"hello");
        let event = recv_event_timeout(&rx, Duration::from_millis(200)).expect("output");
        assert_eq!(event, EmbeddedPtyEvent::Output(b"hello".to_vec()));

        session.kill().expect("kill");
        assert_eq!(*port.killed.lock().unwrap(), 1);
        let event = recv_event_timeout(&rx, Duration::from_millis(200)).expect("exit");
        assert!(matches!(event, EmbeddedPtyEvent::Exited { .. }));
    }

    #[test]
    fn fake_spawn_failure_is_structured() {
        let port = FakeEmbeddedPty::new();
        *port.fail_spawn.lock().unwrap() = Some("no pty".into());
        let err = port
            .spawn(EmbeddedPtySpawnRequest {
                program: "/bin/sh".into(),
                args: vec![],
                environment: vec![],
                size: EmbeddedPtySize::new(80, 24),
            })
            .err()
            .expect("error");
        assert!(matches!(err, EmbeddedPtyError::Unavailable(_)));
    }

    #[test]
    fn size_clamps_zero_to_one() {
        assert_eq!(
            EmbeddedPtySize::new(0, 0),
            EmbeddedPtySize { cols: 1, rows: 1 }
        );
    }
}

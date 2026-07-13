use async_trait::async_trait;
use tokio::process::Command;

pub use luma_application::{ProcessCatalogPort as ProcessCatalog, ProcessEntry, ProcessError};

pub struct MacProcessCatalog;

#[cfg(target_os = "macos")]
fn process_start_unix(pid: u32) -> Option<i64> {
    // proc_bsdinfo.pbi_start_tvsec — stable birth time, not (now − etimes).
    #[repr(C)]
    struct ProcBsdInfo {
        _pbi_flags: u32,
        _pbi_status: u32,
        _pbi_xstatus: u32,
        _pbi_pid: u32,
        _pbi_ppid: u32,
        _pbi_uid: u32,
        _pbi_gid: u32,
        _pbi_ruid: u32,
        _pbi_rgid: u32,
        _pbi_svuid: u32,
        _pbi_svgid: u32,
        _rfu_1: u32,
        _pbi_comm: [u8; 16],
        _pbi_name: [u8; 32],
        _pbi_nfiles: u32,
        _pbi_pgid: u32,
        _pbi_pjobc: u32,
        _e_tdev: u32,
        _e_tpgid: u32,
        _pbi_nice: i32,
        pbi_start_tvsec: u64,
        _pbi_start_tvusec: u64,
    }

    const PROC_PIDTBSDINFO: i32 = 3;

    extern "C" {
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut core::ffi::c_void,
            buffersize: i32,
        ) -> i32;
    }

    let mut info = std::mem::MaybeUninit::<ProcBsdInfo>::uninit();
    let size = std::mem::size_of::<ProcBsdInfo>() as i32;
    let got = unsafe {
        proc_pidinfo(
            pid as i32,
            PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if got != size {
        return None;
    }
    let info = unsafe { info.assume_init() };
    Some(info.pbi_start_tvsec as i64)
}

#[cfg(not(target_os = "macos"))]
fn process_start_unix(_pid: u32) -> Option<i64> {
    None
}

#[async_trait]
impl ProcessCatalog for MacProcessCatalog {
    async fn list_gui_ish(&self) -> Result<Vec<ProcessEntry>, ProcessError> {
        let out = Command::new("ps")
            .args(["-axo", "pid=,comm="])
            .output()
            .await?;
        if !out.status.success() {
            return Err(ProcessError::Unavailable("ps failed".into()));
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut entries = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(pid_s) = parts.next() else { continue };
            let Ok(pid) = pid_s.parse::<u32>() else {
                continue;
            };
            let executable = parts.collect::<Vec<_>>().join(" ");
            if executable.is_empty() {
                continue;
            }
            if executable.contains(".app/") || !executable.starts_with('/') {
                let Some(start_unix) = process_start_unix(pid) else {
                    continue;
                };
                let name = executable
                    .rsplit('/')
                    .next()
                    .unwrap_or(&executable)
                    .trim()
                    .to_string();
                entries.push(ProcessEntry {
                    pid,
                    name,
                    executable,
                    start_unix,
                });
            }
            if entries.len() >= 200 {
                break;
            }
        }
        Ok(entries)
    }

    async fn quit(&self, pid: u32, force: bool) -> Result<(), ProcessError> {
        let signal = if force { "-9" } else { "-15" };
        let status = Command::new("kill")
            .args([signal, &pid.to_string()])
            .status()
            .await?;
        if status.success() {
            Ok(())
        } else {
            Err(ProcessError::Unavailable(format!("kill exited {status}")))
        }
    }
}

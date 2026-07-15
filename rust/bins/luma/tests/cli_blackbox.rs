//! CLI blackbox tests against an isolated LumaNext root.
//! Never touches real ~/Library/Application Support/Luma.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn luma_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_luma"))
}

fn run_luma(
    support: &std::path::Path,
    logs: &std::path::Path,
    args: &[&str],
) -> (i32, String, String) {
    let out = Command::new(luma_bin())
        .args(args)
        .env("LUMA_NEXT_SUPPORT_DIR", support)
        .env("LUMA_NEXT_LOGS_DIR", logs)
        .output()
        .expect("spawn luma");
    let code = out.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn doctor_json_ok_on_isolated_root() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, stdout, stderr) = run_luma(&support, &logs, &["doctor", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(v["doctor"], true, "{stdout}");
    assert!(v.get("modules").is_some(), "{stdout}");
    assert!(v.get("probes").is_some(), "{stdout}");
    assert!(v.get("accessibility").is_some(), "{stdout}");
}

#[test]
fn modules_list_json() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, stdout, stderr) = run_luma(&support, &logs, &["modules", "list", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["modules"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["id"] == "luma.apps"));
    assert!(
        v["modules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "luma.windows"),
        "expected luma.windows in modules list: {stdout}"
    );
}

#[test]
fn config_get_and_set_round_trip() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &[
            "config",
            "set",
            "--notes-root",
            "/tmp/luma-notes-fixture",
            "--notes-exclude",
            "private/*",
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    let (code, stdout, stderr) = run_luma(&support, &logs, &["config", "get", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["notes_root"], "/tmp/luma-notes-fixture");
    assert_eq!(v["notes_exclude_patterns"][0], "private/*");
}

#[test]
fn doctor_includes_config_commands_and_skipped_modules_array() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let notes = dir.path().join("notes");
    fs::create_dir_all(&notes).unwrap();
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &[
            "config",
            "set",
            "--notes-root",
            notes.to_str().unwrap(),
            "--notes-exclude",
            "private/*",
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    let (code, stdout, stderr) = run_luma(&support, &logs, &["doctor", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert!(v.get("skipped_modules").is_some(), "{stdout}");
    assert!(
        v["config_commands"]["notes_root"]
            .as_str()
            .unwrap_or("")
            .contains("notes-root"),
        "{stdout}"
    );
    assert_eq!(v["settings"]["configured"], true, "{stdout}");
    assert_eq!(
        v["settings"]["notes_root"].as_str(),
        Some(notes.to_str().unwrap()),
        "{stdout}"
    );
    assert_eq!(
        v["settings"]["notes_exclude_patterns"][0], "private/*",
        "{stdout}"
    );
    assert_eq!(v["stores"]["settings"], "ok", "{stdout}");
    assert!(
        v.get("accessibility").is_some(),
        "missing accessibility: {stdout}"
    );
    assert!(
        v["accessibility"]["trusted"].is_boolean(),
        "accessibility.trusted shape: {stdout}"
    );
    assert!(v.get("probes").is_some(), "missing probes: {stdout}");
    assert!(
        v["probes"]["windows.list"].get("ok").is_some(),
        "probes.windows.list.ok: {stdout}"
    );
    assert!(
        v["probes"]["ax.trusted"].is_boolean(),
        "probes.ax.trusted shape: {stdout}"
    );
    assert_eq!(
        v["ax_trusted"], v["accessibility"]["trusted"],
        "ax_trusted should mirror accessibility.trusted: {stdout}"
    );
}

#[test]
fn notes_query_against_fixture_workspace() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let notes_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/notes-workspaces/basic");
    assert!(
        notes_root.exists(),
        "missing fixture {}",
        notes_root.display()
    );
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &[
            "config",
            "set",
            "--notes-root",
            notes_root.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    let (code, stdout, stderr) = run_luma(&support, &logs, &["query", "n alpha", "--json"]);
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let results = v["results"].as_array().expect("results array");
    assert!(
        results.iter().any(|r| {
            r["title"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .contains("alpha")
                || r["id"].as_str().unwrap_or("").contains("alpha")
        }),
        "expected alpha note hit: {stdout}"
    );
}

#[test]
fn migrate_clipboard_fixture_dry_run_then_commit_rollback() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/legacy/clipboard-history.sample.json");
    assert!(fixture.exists(), "missing fixture {}", fixture.display());

    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &[
            "migrate",
            "clipboard-fixture",
            "--path",
            fixture.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    let dry: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(dry["ledger"]["dry_run"], true);

    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &[
            "migrate",
            "clipboard-fixture",
            "--path",
            fixture.to_str().unwrap(),
            "--commit",
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    let committed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(committed["ledger"]["dry_run"], false);
    let mig = committed["ledger"]["migration_id"]
        .as_str()
        .expect("migration_id")
        .to_string();

    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &["migrate", "rollback", "--migration-id", &mig, "--json"],
    );
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    let rolled: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        rolled["status"] == "rolled_back" || rolled["status"].as_str() == Some("RolledBack"),
        "{rolled}"
    );
}

#[test]
fn action_run_fake_query() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    // Enable fake module then run action
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &["config", "set", "--enable-module", "luma.fake", "--json"],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &[
            "action",
            "run",
            "--query",
            "fake hello",
            "--action-id",
            "open",
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    assert!(
        stdout.contains("success") || stdout.contains("Success") || stdout.contains("ok"),
        "{stdout}"
    );
}

#[test]
fn corrupt_config_blocks_query() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    // First create valid config
    let _ = run_luma(&support, &logs, &["config", "get", "--json"]);
    fs::write(support.join("settings.toml"), "not = toml [[[").unwrap();
    let (code, stdout, stderr) = run_luma(&support, &logs, &["query", "app", "--json"]);
    assert_ne!(
        code, 0,
        "corrupt config must fail query; stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn query_bare_fake_trigger_returns_results_in_cli() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &["config", "set", "--enable-module", "luma.fake", "--json"],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    let (code, stdout, stderr) = run_luma(&support, &logs, &["query", "fake", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    let n = v["results"].as_array().map(|a| a.len()).unwrap_or(0);
    assert!(n >= 1, "bare fake should target module in CLI: {stdout}");
}

#[test]
fn action_failure_exits_nonzero() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &["config", "set", "--enable-module", "luma.fake", "--json"],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &[
            "action",
            "run",
            "--query",
            "fake hello",
            "--action-id",
            "nonexistent_action",
            "--json",
        ],
    );
    assert_ne!(
        code, 0,
        "failed action must exit nonzero; stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn query_json_redact_flag() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, stdout, stderr) =
        run_luma(&support, &logs, &["query", "clip", "--json", "--redact"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("json");
    assert_eq!(v["redacted"], true);
}

#[test]
fn secrets_set_smoke_or_skip_without_keychain() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let out = std::process::Command::new(luma_bin())
        .args(["secrets", "set", "phase6-smoke-label"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .env("LUMA_NEXT_SUPPORT_DIR", &support)
        .env("LUMA_NEXT_LOGS_DIR", &logs)
        .spawn()
        .expect("spawn luma secrets set");
    use std::io::Write;
    let mut child = out;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"phase6-test-value\n");
    }
    let out = child.wait_with_output().expect("wait");
    let code = out.status.code().unwrap_or(1);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("keychain")
        || stderr.contains("Keychain")
        || stderr.contains("unavailable")
        || stderr.contains("secrets set:")
    {
        eprintln!("skip secrets_set_smoke: {stderr}");
        return;
    }
    assert_eq!(code, 0, "stderr={stderr}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("phase6-smoke-label"), "{stdout}");
}

#[test]
fn concurrent_config_set_one_wins() {
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::sync::Barrier;
    use std::thread;
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, _, stderr) = run_luma(&support, &logs, &["config", "get", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let barrier = Arc::new(Barrier::new(2));
    let run_set = |notes: &'static str, barrier: Arc<Barrier>| {
        let support = support.clone();
        let logs = logs.clone();
        thread::spawn(move || {
            barrier.wait();
            Command::new(luma_bin())
                .args([
                    "config",
                    "set",
                    "--notes-root",
                    notes,
                    "--expected-version",
                    "1",
                    "--json",
                ])
                .env("LUMA_NEXT_SUPPORT_DIR", support)
                .env("LUMA_NEXT_LOGS_DIR", logs)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
                .expect("config set")
        })
    };
    let a = run_set("/tmp/luma-lock-a", Arc::clone(&barrier));
    let b = run_set("/tmp/luma-lock-b", barrier);
    let a_out = a.join().unwrap();
    let b_out = b.join().unwrap();
    let codes = [
        a_out.status.code().unwrap_or(1),
        b_out.status.code().unwrap_or(1),
    ];
    let successes = codes.iter().filter(|&&c| c == 0).count();
    assert_eq!(
        successes,
        1,
        "exactly one config set should win; codes={codes:?} a_err={} b_err={}",
        String::from_utf8_lossy(&a_out.stderr),
        String::from_utf8_lossy(&b_out.stderr)
    );
    let (code, stdout, stderr) = run_luma(&support, &logs, &["config", "get", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["settings_version"].as_u64(), Some(2), "{stdout}");
}

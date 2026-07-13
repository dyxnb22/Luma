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
    assert!(
        v.get("modules").is_some() || v.get("doctor").is_some() || v.get("ok").is_some(),
        "{stdout}"
    );
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
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    let (code, stdout, stderr) = run_luma(&support, &logs, &["config", "get", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["notes_root"], "/tmp/luma-notes-fixture");
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

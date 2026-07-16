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
    assert!(
        v["modules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "luma.wordbook"),
        "expected luma.wordbook in modules list: {stdout}"
    );
    assert!(
        v["modules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "luma.records"),
        "expected luma.records in modules list: {stdout}"
    );
    assert!(
        v["modules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "luma.command_recipes"),
        "expected luma.command_recipes in modules list: {stdout}"
    );
}

#[test]
fn cmd_list_json() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, stdout, stderr) = run_luma(&support, &logs, &["cmd", "list", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["recipes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == "git-status"));
}

#[test]
fn query_cmd_test_json() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, stdout, stderr) = run_luma(&support, &logs, &["query", "cmd test", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["query"], "cmd test");
    assert!(v["results"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == "cmd:test"));
}

#[test]
fn cmd_show_missing_recipe_errors() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &["cmd", "show", "no-such-recipe", "--json"],
    );
    assert_ne!(code, 0);
    assert!(stderr.contains("not found") || stderr.contains("no-such-recipe"));
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

#[test]
fn config_import_project_round_trip() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let project = dir.path().join("myapp");
    fs::create_dir(&project).unwrap();
    let path = project.display().to_string();
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &["config", "set", "--import-project", &path, "--json"],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    let (code, stdout, stderr) = run_luma(&support, &logs, &["config", "get", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let imports = v["imported_projects"].as_array().expect("imports");
    assert_eq!(imports.len(), 1);
    assert!(imports[0]["path"].as_str().unwrap().contains("myapp"));
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &["config", "set", "--import-project", &path, "--json"],
    );
    assert_ne!(code, 0);
    assert!(stderr.contains("already imported"), "stderr={stderr}");
    assert!(!stderr.contains("panicked"), "stderr={stderr}");
    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &[
            "config",
            "set",
            "--remove-project",
            "myapp",
            "--expected-version",
            &v["settings_version"].to_string(),
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(project.exists(), "remove must not delete directory");
    let (code, stdout, stderr) = run_luma(&support, &logs, &["config", "get", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["imported_projects"].as_array().unwrap().is_empty());
}

#[test]
fn wordbook_import_wordpet_dry_run_then_commit() {
    use luma_storage::{WordContent, WordbookStore};

    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();

    let src = dir.path().join("wordpet.sqlite3");
    {
        let store = WordbookStore::with_path(src.clone()).unwrap();
        store
            .upsert_content(&WordContent {
                term: "throughput".into(),
                phonetic: "".into(),
                meaning: "吞吐量".into(),
                example: "Improved throughput".into(),
                category: "sys".into(),
            })
            .unwrap();
        let id = store.get_by_term("throughput").unwrap().unwrap().id;
        store.review(id, "known").unwrap();
        store.set_daily_goal(25).unwrap();
    }

    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &[
            "wordbook",
            "import-wordpet",
            "--from",
            src.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    let dry: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(dry["committed"], false);
    assert_eq!(dry["would_insert"], 1);
    assert!(
        !support.join("wordbook.sqlite").exists(),
        "dry-run must not create wordbook.sqlite"
    );

    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &[
            "wordbook",
            "import-wordpet",
            "--from",
            src.to_str().unwrap(),
            "--commit",
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    let committed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(committed["committed"], true);

    let (code, stdout, stderr) = run_luma(&support, &logs, &["query", "wb status", "--json"]);
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    assert!(
        stdout.contains("Today") || stdout.contains("wb:status") || stdout.contains("due"),
        "{stdout}"
    );

    let (code, stdout, stderr) = run_luma(&support, &logs, &["query", "wb throughput", "--json"]);
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    assert!(stdout.contains("throughput"), "{stdout}");
}

#[test]
fn records_import_dry_run_then_apply_and_query() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();

    let root = dir.path().join("records-src");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("电影.md"),
        "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| 沙丘 | 8 | 史诗 |\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &[
            "record",
            "import",
            "--root",
            root.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    let dry: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(dry["preview"]["records"], 1);
    assert!(
        !support.join("records.sqlite").exists(),
        "dry-run must not create records.sqlite"
    );

    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &[
            "record",
            "import",
            "--root",
            root.to_str().unwrap(),
            "--apply",
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    assert!(support.join("records.sqlite").exists());

    let (code, stdout, stderr) = run_luma(&support, &logs, &["query", "rec 沙丘", "--json"]);
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    assert!(stdout.contains("沙丘"), "{stdout}");
    assert!(stdout.contains("luma.records"), "{stdout}");

    let (code, stdout, stderr) = run_luma(&support, &logs, &["record", "rate", "1", "9", "--json"]);
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    let rated: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(rated["rating"], 9);

    let (code, _, stderr) = run_luma(&support, &logs, &["record", "rate", "1"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("provide SCORE") || stderr.contains("--clear"),
        "{stderr}"
    );
}

#[test]
fn records_rollback_does_not_touch_unrelated_support_files() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    fs::write(support.join("settings.toml"), "settings sentinel\n").unwrap();
    fs::write(support.join("clipboard.sqlite"), b"clipboard sentinel").unwrap();

    let root = dir.path().join("records-src");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("电影.md"),
        "| 名字 | 评分 | 备注 |\n|---|---:|---|\n| 沙丘 | 8 | 史诗 |\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run_luma(
        &support,
        &logs,
        &[
            "record",
            "import",
            "--root",
            root.to_str().unwrap(),
            "--apply",
            "--json",
        ],
    );
    assert_eq!(code, 0, "stderr={stderr} stdout={stdout}");
    let applied: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let migration_id = applied["migration"]["migration_id"]
        .as_str()
        .expect("records migration id")
        .to_string();

    let (code, _, stderr) = run_luma(
        &support,
        &logs,
        &["migrate", "rollback", "--migration-id", &migration_id],
    );
    assert_eq!(code, 0, "stderr={stderr}");
    assert_eq!(
        fs::read(support.join("settings.toml")).unwrap(),
        b"settings sentinel\n"
    );
    assert_eq!(
        fs::read(support.join("clipboard.sqlite")).unwrap(),
        b"clipboard sentinel"
    );
}

#[test]
fn ssh_query_not_configured_without_ssh_config() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, stdout, stderr) = run_luma(&support, &logs, &["query", "ssh", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let results = v["results"].as_array().expect("results");
    assert!(
        results.iter().any(|r| r["kind"] == "not_configured"),
        "expected not_configured row: {stdout}"
    );
    let blob = stdout.to_lowercase();
    assert!(!blob.contains("-----begin"));
    assert!(!blob.contains("private-key"));
}

#[test]
fn modules_list_includes_ssh() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, stdout, stderr) = run_luma(&support, &logs, &["modules", "list", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        v["modules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "luma.ssh"),
        "expected luma.ssh in modules list: {stdout}"
    );
}

#[test]
fn modules_list_includes_ports() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, stdout, stderr) = run_luma(&support, &logs, &["modules", "list", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        v["modules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "luma.ports"),
        "expected luma.ports in modules list: {stdout}"
    );
}

#[test]
fn query_port_bare_trigger_json() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, stdout, stderr) = run_luma(&support, &logs, &["query", "port ", "--json"]);
    assert_eq!(code, 0, "stderr={stderr}");
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let results = v["results"].as_array().cloned().unwrap_or_default();
    assert!(
        !results.is_empty(),
        "expected ports results (status or endpoints): {stdout}"
    );
    assert!(
        results.iter().all(|r| {
            let kind = r["kind"].as_str().unwrap_or("");
            matches!(
                kind,
                "port" | "status" | "unavailable" | "permission_required"
            )
        }),
        "unexpected kinds: {stdout}"
    );
}

#[test]
fn ports_kill_requires_yes() {
    let dir = tempdir().unwrap();
    let support = dir.path().join("support");
    let logs = dir.path().join("logs");
    fs::create_dir_all(&support).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let (code, _stdout, stderr) = run_luma(&support, &logs, &["ports", "kill", "65535"]);
    assert_ne!(code, 0, "expected failure without --yes");
    assert!(
        stderr.contains("yes") || stderr.contains("refusing"),
        "stderr={stderr}"
    );
}

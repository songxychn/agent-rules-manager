use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Sandbox(PathBuf);
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn cli(root: &Path, state: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agent-rules"))
        .arg("--root")
        .arg(root)
        .arg("--state-root")
        .arg(state)
        .args(args)
        .output()
        .unwrap()
}
fn json(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn cli_requires_the_preview_token_and_reuses_the_saved_connection() {
    let path = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "arm-cli-sync-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&path).unwrap();
    let sandbox = Sandbox(path);
    let root = sandbox.0.join("rules");
    let state = sandbox.0.join("state");
    let remote = sandbox.0.join("remote.git");
    let result = Command::new("git")
        .args([
            "init",
            "--bare",
            "--initial-branch=master",
            "--object-format=sha1",
            "--template=",
        ])
        .arg(&remote)
        .output()
        .unwrap();
    assert!(result.status.success());
    json(cli(&root, &state, &["init", "--json"]));
    json(cli(&root, &state, &["init", "--apply", "--json"]));
    let preview = json(cli(
        &root,
        &state,
        &["sync", "--remote", remote.to_str().unwrap(), "--json"],
    ));
    assert_eq!(preview["blocked"], false);
    assert!(!cli(
        &root,
        &state,
        &[
            "sync",
            "--remote",
            remote.to_str().unwrap(),
            "--apply",
            "--json"
        ]
    )
    .status
    .success());
    let result = json(cli(
        &root,
        &state,
        &[
            "sync",
            "--remote",
            remote.to_str().unwrap(),
            "--apply",
            "--token",
            preview["token"].as_str().unwrap(),
            "--json",
        ],
    ));
    assert_eq!(result["pushed"], true);
    let again = json(cli(&root, &state, &["sync", "--json"]));
    assert_eq!(again["changes"].as_array().unwrap().len(), 0);
    assert_eq!(again["config"]["branch"], "master");
}

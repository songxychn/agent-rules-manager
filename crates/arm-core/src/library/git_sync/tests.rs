use super::*;
use crate::RulesManager;

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    remote: PathBuf,
    a: RulesManager,
    b: RulesManager,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let remote = root.join("remote.git");
        fs::create_dir(&remote).unwrap();
        git(
            &remote,
            &["init", "--bare", "--initial-branch=master", "--template="],
            None,
        )
        .unwrap();
        let a = RulesManager::with_adapters(root.join("a"), root.join("a-state"), vec![]);
        let b = RulesManager::with_adapters(root.join("b"), root.join("b-state"), vec![]);
        a.plan_initialize().unwrap();
        a.initialize().unwrap();
        b.plan_initialize().unwrap();
        b.initialize().unwrap();
        Self {
            _temp: temp,
            root,
            remote,
            a,
            b,
        }
    }
    fn request(&self) -> GitSyncRequest {
        GitSyncRequest {
            remote: self.remote.to_str().unwrap().into(),
            branch: "master".into(),
            resolutions: vec![],
        }
    }
    fn sync(&self, manager: &RulesManager) -> GitSyncOutcome {
        let request = self.request();
        let plan = manager.preview_git_sync(&request).unwrap();
        assert!(!plan.blocked, "{:?}", plan);
        let result = manager.apply_git_sync(&request, &plan.token).unwrap();
        assert!(result.push_error.is_none(), "{:?}", result);
        result
    }
    fn paired(&self) {
        self.sync(&self.a);
        self.sync(&self.b);
    }
    fn source(&self, machine: &str, id: &str) -> PathBuf {
        self.root.join(format!("{machine}/profiles/{id}/AGENTS.md"))
    }
    fn remote_file(&self, path: &str) -> String {
        git_text(
            &self.remote,
            &["show", &format!("refs/heads/master:{path}")],
        )
        .unwrap()
    }
    fn inject_remote(&self, path: &str, mode: &str, content: &str) {
        git(&self.remote, &["read-tree", "refs/heads/master"], None).unwrap();
        let blob = String::from_utf8(
            git(
                &self.remote,
                &["hash-object", "-w", "--stdin"],
                Some(content.as_bytes()),
            )
            .unwrap(),
        )
        .unwrap();
        git(
            &self.remote,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("{mode},{},{path}", blob.trim()),
            ],
            None,
        )
        .unwrap();
        let tree = git_text(&self.remote, &["write-tree"]).unwrap();
        let commit = git_text(
            &self.remote,
            &[
                "commit-tree",
                &tree,
                "-p",
                "refs/heads/master",
                "-m",
                "external update",
            ],
        )
        .unwrap();
        git(
            &self.remote,
            &["update-ref", "refs/heads/master", &commit],
            None,
        )
        .unwrap();
    }
}

#[test]
fn preview_is_read_only_and_upload_obeys_allowlist_without_touching_library_git() {
    let f = Fixture::new();
    let library = f.root.join("a");
    fs::write(library.join("private.txt"), "do not upload").unwrap();
    git(
        &library,
        &["init", "--initial-branch=master", "--template="],
        None,
    )
    .unwrap();
    git(&library, &["add", "private.txt"], None).unwrap();
    let index = fs::read(library.join(".git/index")).unwrap();
    let history = f.a.operation_history().unwrap().len();
    let preview = f.a.preview_git_sync(&f.request()).unwrap();
    assert!(!preview.blocked);
    assert!(f.a.git_sync_config().unwrap().is_none());
    assert_eq!(history, f.a.operation_history().unwrap().len());
    assert!(git_text(&f.remote, &["show-ref"]).is_err());
    let outcome = f.a.apply_git_sync(&f.request(), &preview.token).unwrap();
    assert!(outcome.pushed);
    assert_eq!(fs::read(library.join(".git/index")).unwrap(), index);
    let names = git_text(
        &f.remote,
        &["ls-tree", "-r", "--name-only", "refs/heads/master"],
    )
    .unwrap();
    assert_eq!(
        names.lines().collect::<Vec<_>>(),
        [
            "profiles/default/AGENTS.md",
            "profiles/default/profile.json",
            "schema.json"
        ]
    );
    assert_eq!(f.a.git_sync_config().unwrap().unwrap().branch, "master");
}

#[test]
fn remote_update_does_not_activate_or_change_current_runtime() {
    let f = Fixture::new();
    f.paired();
    f.b.activate_profile("default").unwrap();
    let runtime = fs::read_to_string(f.root.join("b/current/AGENTS.md")).unwrap();
    let current = fs::read_link(f.root.join("b/current")).unwrap();
    let machine = fs::read(f.root.join("b-state/machine.json")).unwrap();
    fs::write(f.source("a", "default"), "# Rules from A\n").unwrap();
    f.sync(&f.a);
    let plan = f.b.preview_git_sync(&f.request()).unwrap();
    assert_eq!(f.b.snapshot().unwrap().runtime_state, RuntimeState::Current);
    assert_ne!(
        fs::read_to_string(f.source("b", "default")).unwrap(),
        "# Rules from A\n"
    );
    f.b.apply_git_sync(&f.request(), &plan.token).unwrap();
    assert_eq!(
        fs::read_to_string(f.source("b", "default")).unwrap(),
        "# Rules from A\n"
    );
    assert_eq!(
        fs::read_to_string(f.root.join("b/current/AGENTS.md")).unwrap(),
        runtime
    );
    assert_eq!(fs::read_link(f.root.join("b/current")).unwrap(), current);
    assert_eq!(
        fs::read(f.root.join("b-state/machine.json")).unwrap(),
        machine
    );
    assert_eq!(f.b.snapshot().unwrap().runtime_state, RuntimeState::Stale);
    f.b.activate_profile("default").unwrap();
    assert_eq!(f.b.snapshot().unwrap().runtime_state, RuntimeState::Current);
}

#[test]
fn metadata_and_content_merge_independently_using_the_common_base() {
    let f = Fixture::new();
    f.paired();
    let metadata = f.root.join("a/profiles/default/profile.json");
    let mut doc: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&metadata).unwrap()).unwrap();
    doc["name"] = "Renamed on A".into();
    fs::write(metadata, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
    f.sync(&f.a);
    fs::write(f.source("b", "default"), "# Edited on B\n").unwrap();
    let plan = f.b.preview_git_sync(&f.request()).unwrap();
    assert!(!plan.blocked);
    assert!(plan.conflicts.is_empty());
    f.b.apply_git_sync(&f.request(), &plan.token).unwrap();
    assert!(f
        .remote_file("profiles/default/profile.json")
        .contains("Renamed on A"));
    assert_eq!(f.remote_file("profiles/default/AGENTS.md"), "# Edited on B");
    assert_eq!(f.b.snapshot().unwrap().profiles[0].name, "Renamed on A");
}

#[test]
fn true_conflicts_require_a_review_and_support_all_three_choices() {
    for choice in [
        GitSyncChoice::KeepLocal,
        GitSyncChoice::UseRemote,
        GitSyncChoice::KeepBoth,
    ] {
        let f = Fixture::new();
        f.paired();
        fs::write(f.source("a", "default"), "# Remote version\n").unwrap();
        f.sync(&f.a);
        fs::write(f.source("b", "default"), "# Local version\n").unwrap();
        let mut request = f.request();
        let initial = f.b.preview_git_sync(&request).unwrap();
        assert!(initial.blocked);
        assert_eq!(initial.conflicts.len(), 1);
        assert!(!initial.conflicts[0].base.is_empty());
        assert!(f.b.apply_git_sync(&request, &initial.token).is_err());
        request.resolutions.push(GitSyncResolution {
            id: "profile:default".into(),
            choice: choice.clone(),
            new_profile_id: Some("remote-copy".into()),
        });
        assert!(f.b.apply_git_sync(&request, &initial.token).is_err());
        let plan = f.b.preview_git_sync(&request).unwrap();
        assert!(!plan.blocked);
        let outcome = f.b.apply_git_sync(&request, &plan.token).unwrap();
        assert!(outcome.push_error.is_none());
        let expected = if choice == GitSyncChoice::UseRemote {
            "# Remote version\n"
        } else {
            "# Local version\n"
        };
        assert_eq!(
            fs::read_to_string(f.source("b", "default")).unwrap(),
            expected
        );
        if choice == GitSyncChoice::KeepBoth {
            assert_eq!(
                fs::read_to_string(f.source("b", "remote-copy")).unwrap(),
                "# Remote version\n"
            );
            assert!(f
                .remote_file("profiles/remote-copy/profile.json")
                .contains("\"id\": \"remote-copy\""));
        }
    }
}

#[test]
fn changing_either_side_or_the_local_selection_invalidates_preview() {
    let f = Fixture::new();
    f.paired();
    let request = f.request();
    let plan = f.b.preview_git_sync(&request).unwrap();
    fs::write(f.source("b", "default"), "# concurrent local save\n").unwrap();
    assert!(f.b.apply_git_sync(&request, &plan.token).is_err());
    let plan = f.b.preview_git_sync(&request).unwrap();
    fs::write(f.source("a", "default"), "# concurrent remote save\n").unwrap();
    f.sync(&f.a);
    assert!(f.b.apply_git_sync(&request, &plan.token).is_err());
    f.a.create_profile("other", "Other", "").unwrap();
    let plan = f.a.preview_git_sync(&request).unwrap();
    f.a.activate_profile("other").unwrap();
    assert!(f.a.apply_git_sync(&request, &plan.token).is_err());
}

#[test]
fn deleting_an_active_profile_is_blocked_then_safe_after_switch_and_is_restorable() {
    let f = Fixture::new();
    f.a.create_profile("work", "Work", "").unwrap();
    f.paired();
    f.b.activate_profile("work").unwrap();
    f.a.delete_profile("work").unwrap();
    f.sync(&f.a);
    let plan = f.b.preview_git_sync(&f.request()).unwrap();
    assert!(plan.blocked);
    assert!(plan.blockers.iter().any(|b| b.contains("active Profile")));
    f.b.activate_profile("default").unwrap();
    let before = f.b.operation_history().unwrap()[0].id.clone();
    f.sync(&f.b);
    assert!(!f.root.join("b/profiles/work").exists());
    assert_eq!(f.b.snapshot().unwrap().profiles.len(), 1);
    let after = f.b.operation_history().unwrap()[0].id.clone();
    let restore = f.b.preview_restore(&before).unwrap();
    assert!(!restore.blocked);
    f.b.restore_history(&before, &restore.token).unwrap();
    assert!(f.source("b", "work").exists());
    let redo = f.b.preview_restore(&after).unwrap();
    assert!(!redo.blocked);
    f.b.restore_history(&after, &redo.token).unwrap();
    assert!(!f.root.join("b/profiles/work").exists());
    assert_eq!(f.b.snapshot().unwrap().profiles.len(), 1);
}

#[test]
fn deletion_conflicts_and_unmanaged_entries_are_preserved() {
    let f = Fixture::new();
    f.a.create_profile("work", "Work", "").unwrap();
    f.paired();
    f.a.delete_profile("work").unwrap();
    f.sync(&f.a);
    fs::write(f.source("b", "work"), "# local edit\n").unwrap();
    let mut request = f.request();
    let plan = f.b.preview_git_sync(&request).unwrap();
    assert!(plan.blocked);
    assert_eq!(plan.conflicts[0].id, "profile:work");
    assert!(!plan.conflicts[0].can_keep_both);
    request.resolutions.push(GitSyncResolution {
        id: "profile:work".into(),
        choice: GitSyncChoice::UseRemote,
        new_profile_id: None,
    });
    fs::write(f.root.join("b/profiles/work/notes.txt"), "unmanaged").unwrap();
    let plan = f.b.preview_git_sync(&request).unwrap();
    assert!(plan.blockers.iter().any(|b| b.contains("unmanaged")));
    assert!(f.b.apply_git_sync(&request, &plan.token).is_err());
    assert_eq!(
        fs::read_to_string(f.root.join("b/profiles/work/notes.txt")).unwrap(),
        "unmanaged"
    );
}

#[test]
fn local_recovery_refuses_source_drift_and_does_not_rewind_remote() {
    let f = Fixture::new();
    f.paired();
    fs::write(f.source("a", "default"), "# remote updated\n").unwrap();
    f.sync(&f.a);
    f.sync(&f.b);
    fs::write(f.source("b", "default"), "# subsequent local edit\n").unwrap();
    assert!(matches!(
        f.b.rollback_library_latest(),
        Err(ArmError::RollbackDrift(_))
    ));
    assert_eq!(
        f.remote_file("profiles/default/AGENTS.md"),
        "# remote updated"
    );
}

#[test]
fn a_failed_push_preserves_local_snapshot_and_can_be_retried() {
    let f = Fixture::new();
    f.paired();
    fs::write(f.source("b", "default"), "# offline edit\n").unwrap();
    git(&f.remote, &["config", "receive.maxInputSize", "1"], None).unwrap();
    let request = f.request();
    let plan = f.b.preview_git_sync(&request).unwrap();
    let outcome = f.b.apply_git_sync(&request, &plan.token).unwrap();
    assert!(outcome.push_error.is_some());
    assert!(!outcome.pushed);
    assert!(outcome.backup_id.is_some());
    assert_eq!(
        fs::read_to_string(f.source("b", "default")).unwrap(),
        "# offline edit\n"
    );
    git(
        &f.remote,
        &["config", "--unset", "receive.maxInputSize"],
        None,
    )
    .unwrap();
    f.sync(&f.b);
    assert_eq!(
        f.remote_file("profiles/default/AGENTS.md"),
        "# offline edit"
    );
}

#[test]
fn hostile_remote_paths_symlinks_and_invalid_manifests_are_rejected() {
    for (path, mode, content) in [
        (".runtime/leak.md", "100644", "unsafe"),
        ("profiles/default/AGENTS.md", "120000", "/etc/passwd"),
        (
            "profiles/default/profile.json",
            "100644",
            "{\"schemaVersion\":99}",
        ),
        ("profiles/default/extra.md", "100644", "undeclared"),
    ] {
        let f = Fixture::new();
        f.paired();
        let before = fs::read(f.source("b", "default")).unwrap();
        f.inject_remote(path, mode, content);
        assert!(f.b.preview_git_sync(&f.request()).is_err(), "{path}");
        assert_eq!(fs::read(f.source("b", "default")).unwrap(), before);
    }
}

#[cfg(unix)]
#[test]
fn swapped_parent_symlinks_are_rejected_before_any_local_write() {
    let f = Fixture::new();
    f.paired();
    let plan = f.b.preview_git_sync(&f.request()).unwrap();
    let original = f.root.join("b/profiles");
    let renamed = f.root.join("preserved-profiles");
    fs::rename(&original, &renamed).unwrap();
    std::os::unix::fs::symlink(&renamed, &original).unwrap();
    assert!(f.b.apply_git_sync(&f.request(), &plan.token).is_err());
    assert!(renamed.join("default/AGENTS.md").is_file());
}

#[test]
fn credentials_options_and_unsupported_transports_are_rejected() {
    for remote in [
        "ssh://host:2222/rules.git",
        "ssh://git@host:2222/rules.git",
        "https://host/rules.git",
        "git@host:rules.git",
    ] {
        assert!(validate_config(&GitSyncConfig {
            remote: remote.into(),
            branch: "release/rules".into()
        })
        .is_ok());
    }
    for remote in [
        "https://token@github.com/user/rules.git",
        "https://github.com/rules?token=secret",
        "ext::touch /tmp/no",
        "--upload-pack=bad",
        "http://host/repo",
        "git@host -oProxyCommand=bad:repo",
    ] {
        assert!(validate_config(&GitSyncConfig {
            remote: remote.into(),
            branch: "master".into()
        })
        .is_err());
    }
    for branch in ["-bad", "foo/../bar", "foo.lock", "a//b", ".hidden", "a@{b"] {
        assert!(validate_config(&GitSyncConfig {
            remote: "git@host:rules.git".into(),
            branch: branch.into()
        })
        .is_err());
    }
}

#[test]
fn a_remote_with_an_empty_initial_commit_accepts_the_first_source_sync() {
    let f = Fixture::new();
    let tree = String::from_utf8(git(&f.remote, &["mktree"], Some(b"")).unwrap()).unwrap();
    let initial = git_text(
        &f.remote,
        &["commit-tree", tree.trim(), "-m", "Initialize rule sync"],
    )
    .unwrap();
    git(
        &f.remote,
        &["update-ref", "refs/heads/master", &initial],
        None,
    )
    .unwrap();
    let plan = f.a.preview_git_sync(&f.request()).unwrap();
    assert_eq!(plan.remote_commit.as_deref(), Some(initial.as_str()));
    assert!(!plan.blocked);
    let outcome = f.a.apply_git_sync(&f.request(), &plan.token).unwrap();
    assert!(outcome.pushed);
    assert!(outcome.push_error.is_none());
    assert_eq!(
        git_text(&f.remote, &["rev-parse", "refs/heads/master^"]).unwrap(),
        initial
    );
    assert!(f.remote_file("schema.json").contains("agent-rules-library"));
}

#[test]
fn importing_an_existing_remote_into_an_empty_library_leaves_selection_local() {
    let f = Fixture::new();
    f.sync(&f.a);
    f.inject_remote(".gitignore", "100644", ".runtime/\ncurrent\n");
    let root = f.root.join("imported");
    let manager = RulesManager::with_adapters(root.clone(), f.root.join("imported-state"), vec![]);
    let request = f.request();
    let plan = manager.preview_git_sync(&request).unwrap();
    assert!(!root.exists());
    assert!(!plan.blocked);
    assert!(plan
        .directories
        .iter()
        .any(|d| d.path == "profiles" && d.action == "create"));
    manager.apply_git_sync(&request, &plan.token).unwrap();
    assert_eq!(manager.snapshot().unwrap().active_profile_id, None);
    assert!(!root.join("current").exists());
    assert!(!root.join(".runtime").exists());
    assert!(!root.join(".gitignore").exists());
    assert_eq!(f.remote_file(".gitignore"), ".runtime/\ncurrent");
    manager.rollback_library_latest().unwrap();
    assert!(!root.exists());
}

#[test]
fn a_schema_only_library_can_be_imported_without_creating_an_invalid_layout() {
    let f = Fixture::new();
    f.sync(&f.a);
    // A library with no selected Profile can legitimately remove its last one.
    let import_root = f.root.join("unselected");
    let m =
        RulesManager::with_adapters(import_root.clone(), f.root.join("unselected-state"), vec![]);
    f.sync(&m);
    m.delete_profile("default").unwrap();
    f.sync(&m);
    let empty =
        RulesManager::with_adapters(f.root.join("empty"), f.root.join("empty-state"), vec![]);
    f.sync(&empty);
    assert!(empty.snapshot().unwrap().profiles.is_empty());
    assert!(f.root.join("empty/profiles").is_dir());
}

#[test]
fn publication_rejects_a_remote_rewind_after_the_final_local_check() {
    let f = Fixture::new();
    f.paired();
    let old = git_text(&f.remote, &["rev-parse", "refs/heads/master"]).unwrap();
    fs::write(f.source("a", "default"), "# revision 2\n").unwrap();
    f.sync(&f.a);
    fs::write(f.source("b", "default"), "# revision 3\n").unwrap();
    let mut request = f.request();
    request.resolutions.push(GitSyncResolution {
        id: "profile:default".into(),
        choice: GitSyncChoice::KeepLocal,
        new_profile_id: None,
    });
    let prepared = prepare(&f.root.join("b"), &f.root.join("b-state"), &request).unwrap();
    let commit = commit(&prepared).unwrap().unwrap();
    git(&f.remote, &["update-ref", "refs/heads/master", &old], None).unwrap();
    assert!(publish(&prepared, &commit).is_err());
    assert_eq!(
        git_text(&f.remote, &["rev-parse", "refs/heads/master"]).unwrap(),
        old
    );
}

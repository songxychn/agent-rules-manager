//! Manual source synchronization. Git never checks out into the user's library.
//! A preview binds both source snapshots, the remote commit, local selection and
//! conflict decisions. Applying sources never changes runtime or machine.json.
use super::*;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

type Files = BTreeMap<String, String>;
const MAX_FILE: usize = 1024 * 1024;
const MAX_TOTAL: usize = 16 * MAX_FILE;
const MAX_PROFILES: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncConfig {
    pub remote: String,
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GitSyncChoice {
    KeepLocal,
    UseRemote,
    KeepBoth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncResolution {
    pub id: String,
    pub choice: GitSyncChoice,
    #[serde(default)]
    pub new_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncRequest {
    pub remote: String,
    pub branch: String,
    #[serde(default)]
    pub resolutions: Vec<GitSyncResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncChange {
    pub path: String,
    pub local_before: Option<String>,
    pub local_after: Option<String>,
    pub remote_before: Option<String>,
    pub remote_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncVersion {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncConflict {
    pub id: String,
    pub base: Vec<GitSyncVersion>,
    pub local: Vec<GitSyncVersion>,
    pub remote: Vec<GitSyncVersion>,
    pub can_keep_both: bool,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncPlan {
    pub token: String,
    pub config: GitSyncConfig,
    pub remote_commit: Option<String>,
    pub changes: Vec<GitSyncChange>,
    /// Empty directory creation/deletion is also part of the reviewed plan.
    pub directories: Vec<GitSyncDirectoryChange>,
    pub conflicts: Vec<GitSyncConflict>,
    pub blockers: Vec<String>,
    pub blocked: bool,
    pub connection_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncDirectoryChange {
    pub path: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSyncOutcome {
    pub changed: Vec<String>,
    pub backup_id: Option<String>,
    pub pushed: bool,
    /// Local sources are already backed up/applied when a publication fails.
    /// The next preview reconciles that state; it never claims a failed push synced.
    pub push_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSync {
    version: u32,
    config: GitSyncConfig,
    published_commit: Option<String>,
    baseline: Files,
    previous_remote: Files,
}

struct PreparedSync {
    plan: GitSyncPlan,
    repo: tempfile::TempDir,
    local: Files,
    remote: Files,
    merged: Files,
    state_path: PathBuf,
    state_before: FileState,
    machine_before: FileState,
    deleted_directories: Vec<PathBuf>,
}

fn fail(message: impl Into<String>) -> ArmError {
    ArmError::Blocked(message.into())
}

// Do not allow a URL to become a git option, remote helper, shell command or
// credential store. Authentication belongs to system Git/SSH credential helpers.
fn validate_config(config: &GitSyncConfig) -> Result<(), ArmError> {
    let remote = &config.remote;
    if remote.is_empty() || remote.trim() != remote || remote.chars().any(char::is_control) {
        return Err(fail("Enter a Git remote without embedded credentials"));
    }
    let url = remote
        .strip_prefix("https://")
        .or_else(|| remote.strip_prefix("ssh://"));
    let valid = if let Some(rest) = url {
        let host = rest.split('/').next().unwrap_or_default();
        !host.is_empty()
            && !rest.contains(['?', '#', ' ', '\\'])
            && if remote.starts_with("https://") {
                !host.contains('@')
            } else {
                !host
                    .split_once('@')
                    .is_some_and(|(user, _)| user.contains(':'))
            }
    } else if Path::new(remote).is_absolute() {
        true
    } else if let Some((host, path)) = remote.split_once(':') {
        !path.is_empty()
            && host.contains('@')
            && !host.starts_with('-')
            && host
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"@._-".contains(&c))
    } else {
        false
    };
    if !valid {
        return Err(fail(
            "Use HTTPS without credentials, SSH, or an absolute local Git path",
        ));
    }
    if config.branch.starts_with('-')
        || config.branch.is_empty()
        || !config
            .branch
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/_-.".contains(&c))
        || config
            .branch
            .split('/')
            .any(|p| p.is_empty() || p.starts_with('.') || p.ends_with('.') || p.ends_with(".lock"))
        || config.branch.contains("..")
    {
        return Err(fail("Enter a valid branch name"));
    }
    Ok(())
}

fn validate_path(path: &Path) -> Result<(), ArmError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(fail(
            "Synchronization requires absolute paths without .. components",
        ));
    }
    for parent in path.ancestors() {
        match fs::symlink_metadata(parent) {
            Ok(m) if m.file_type().is_symlink() || (parent != path && !m.is_dir()) => {
                return Err(ArmError::UnsupportedEntry(parent.display().to_string()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(ArmError::io(parent.display().to_string(), e)),
        }
    }
    Ok(())
}

fn state_path(root: &Path, state: &Path) -> Result<PathBuf, ArmError> {
    validate_path(root)?;
    let path = state.join("git-sync").join(format!(
        "{}.json",
        full_digest(root.to_string_lossy().as_bytes())
    ));
    validate_path(&path)?;
    Ok(path)
}

fn read_stored(
    root: &Path,
    state: &Path,
) -> Result<(PathBuf, FileState, Option<StoredSync>), ArmError> {
    let path = state_path(root, state)?;
    let before = read_file_state(&path)?;
    let stored = match &before {
        FileState::Missing => None,
        FileState::File { content } => {
            let stored: StoredSync = serde_json::from_str(content)?;
            if stored.version != 1 {
                return Err(fail("Unsupported Git sync state"));
            }
            validate_config(&stored.config)?;
            validate_sources(&stored.baseline)?;
            validate_sources(&stored.previous_remote)?;
            Some(stored)
        }
        _ => return Err(ArmError::UnsupportedEntry(path.display().to_string())),
    };
    Ok((path, before, stored))
}

pub(crate) fn config(root: &Path, state: &Path) -> Result<Option<GitSyncConfig>, ArmError> {
    Ok(read_stored(root, state)?.2.map(|s| s.config))
}

fn allowed_source(path: &str) -> bool {
    if path == SCHEMA_FILE {
        return true;
    }
    let parts: Vec<_> = path.split('/').collect();
    parts.len() == 3
        && parts[0] == PROFILES_DIR
        && validate_id(parts[1]).is_ok()
        && matches!(parts[2], PROFILE_MANIFEST_FILE | ENTRYPOINT_FILE)
}

fn validate_sources(files: &Files) -> Result<(), ArmError> {
    if files.is_empty() {
        return Ok(());
    }
    if files.len() > MAX_PROFILES * 2 + 1
        || files.values().map(String::len).sum::<usize>() > MAX_TOTAL
    {
        return Err(fail(
            "Rule library exceeds the Git sync limit (256 Profiles / 16 MiB)",
        ));
    }
    for (path, content) in files {
        if !allowed_source(path) || content.len() > MAX_FILE || content.contains('\0') {
            return Err(fail(format!(
                "Unsupported sync source or file over 1 MiB: {path}"
            )));
        }
    }
    let schema: SchemaDocument = serde_json::from_str(
        files
            .get(SCHEMA_FILE)
            .ok_or_else(|| fail("Remote is missing schema.json"))?,
    )?;
    if schema.schema_version != LIBRARY_SCHEMA_VERSION
        || schema.format != "agent-rules-library"
        || schema.sync.machine_selection != "local"
        || schema
            .legacy_source
            .as_deref()
            .is_some_and(|s| s != LEGACY_SOURCE_FILE)
    {
        return Err(fail(
            "Git sync requires a v3 rule library with machine-local selection",
        ));
    }
    for id in profile_ids(files) {
        let prefix = format!("profiles/{id}");
        let doc: ProfileDocument = serde_json::from_str(
            files
                .get(&format!("{prefix}/profile.json"))
                .ok_or_else(|| fail(format!("Missing manifest for {id}")))?,
        )?;
        validate_profile_document(&doc, &id)?;
        if !files.contains_key(&format!("{prefix}/AGENTS.md")) {
            return Err(fail(format!("Missing AGENTS.md for {id}")));
        }
    }
    Ok(())
}

fn profile_ids(files: &Files) -> BTreeSet<String> {
    files
        .keys()
        .filter_map(|p| {
            p.strip_prefix("profiles/")?
                .split('/')
                .next()
                .map(String::from)
        })
        .collect()
}

fn local_sources(root: &Path) -> Result<Files, ArmError> {
    validate_path(root)?;
    validate_path(&root.join(SCHEMA_FILE))?;
    validate_path(&root.join(PROFILES_DIR))?;
    let (state, detail) = detect_library_state(root)?;
    if state == LibraryState::Empty {
        return Ok(Files::new());
    }
    if state != LibraryState::Ready {
        return Err(fail(detail));
    }
    let loaded = load_library(root)?;
    let mut paths = vec![SCHEMA_FILE.to_string()];
    for id in loaded.profiles.keys() {
        paths.push(format!("profiles/{id}/profile.json"));
        paths.push(format!("profiles/{id}/AGENTS.md"));
    }
    let mut files = Files::new();
    for relative in paths {
        let path = root.join(&relative);
        validate_path(&path)?;
        let m =
            fs::symlink_metadata(&path).map_err(|e| ArmError::io(path.display().to_string(), e))?;
        if !m.is_file() || m.len() > MAX_FILE as u64 {
            return Err(fail(format!("Unsupported source: {relative}")));
        }
        files.insert(
            relative,
            fs::read_to_string(&path).map_err(|e| ArmError::io(path.display().to_string(), e))?,
        );
    }
    validate_sources(&files)?;
    Ok(files)
}

fn git_command(repo: &Path) -> Command {
    let mut cmd = Command::new("git");
    // Ambient repository/index overrides must not redirect operations into a
    // developer checkout. Keep system authentication and SSH agent settings.
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.arg("-C")
        .arg(repo)
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "protocol.ext.allow=never",
            "-c",
            "credential.interactive=false",
            "-c",
            "http.lowSpeedLimit=1",
            "-c",
            "http.lowSpeedTime=30",
            "-c",
            "commit.gpgsign=false",
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .env(
            "GIT_SSH_COMMAND",
            "ssh -o BatchMode=yes -o ConnectTimeout=15 -o ConnectionAttempts=1",
        )
        .env("GIT_AUTHOR_NAME", "Agent Rules Manager")
        .env("GIT_AUTHOR_EMAIL", "agent-rules-manager@local")
        .env("GIT_COMMITTER_NAME", "Agent Rules Manager")
        .env("GIT_COMMITTER_EMAIL", "agent-rules-manager@local");
    cmd
}

fn git(repo: &Path, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>, ArmError> {
    let mut cmd = git_command(repo);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    if input.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| ArmError::io("system Git (install Git and make it available in PATH)", e))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| fail("Git stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| fail("Git stderr unavailable"))?;
    let read = |pipe: Box<dyn Read + Send>, limit: u64| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.take(limit + 1).read_to_end(&mut bytes).map(|_| bytes)
        })
    };
    let out = read(Box::new(stdout), (MAX_TOTAL * 2) as u64);
    let err = read(Box::new(stderr), 65536);
    if let Some(bytes) = input {
        if let Err(error) = child
            .stdin
            .take()
            .ok_or_else(|| fail("Git stdin unavailable"))?
            .write_all(bytes)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ArmError::io("Git input", error));
        }
    }
    let start = Instant::now();
    let status = loop {
        match child
            .try_wait()
            .map_err(|e| ArmError::io("Git process", e))?
        {
            Some(status) => break status,
            None if start.elapsed() >= Duration::from_secs(45) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(fail("Git operation timed out after 45 seconds. Check the remote and system Git/SSH authentication, then preview again."));
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    let stdout = out
        .join()
        .map_err(|_| fail("Git output reader failed"))?
        .map_err(|e| ArmError::io("Git output", e))?;
    let stderr = err
        .join()
        .map_err(|_| fail("Git error reader failed"))?
        .map_err(|e| ArmError::io("Git error output", e))?;
    if !status.success() {
        return Err(fail(format!(
            "Git {} failed: {}",
            args.first().unwrap_or(&"operation"),
            String::from_utf8_lossy(&stderr)
                .chars()
                .take(2000)
                .collect::<String>()
        )));
    }
    if stdout.len() > MAX_TOTAL * 2 || stderr.len() > 65536 {
        return Err(fail("Git response exceeds the sync limit"));
    }
    Ok(stdout)
}

fn git_text(repo: &Path, args: &[&str]) -> Result<String, ArmError> {
    String::from_utf8(git(repo, args, None)?)
        .map(|s| s.trim_end().to_string())
        .map_err(|_| fail("Git returned non-UTF-8 data"))
}

fn remote_head(repo: &Path, config: &GitSyncConfig) -> Result<Option<String>, ArmError> {
    let reference = format!("refs/heads/{}", config.branch);
    let output = git_text(
        repo,
        &["ls-remote", "--refs", "--", &config.remote, &reference],
    )?;
    if output.is_empty() {
        return Ok(None);
    }
    let mut lines = output.lines();
    let (oid, name) = lines
        .next()
        .and_then(|l| l.split_once('\t'))
        .ok_or_else(|| fail("Invalid remote branch response"))?;
    if name != reference
        || lines.next().is_some()
        || oid.len() != 40
        || !oid.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(fail("Invalid remote branch response"));
    }
    Ok(Some(oid.to_string()))
}

fn fetch(config: &GitSyncConfig) -> Result<(tempfile::TempDir, Option<String>, Files), ArmError> {
    let repo = tempfile::tempdir().map_err(|e| ArmError::io("temporary Git repository", e))?;
    git(
        repo.path(),
        &[
            "init",
            "--bare",
            "--initial-branch=master",
            "--object-format=sha1",
            "--template=",
        ],
        None,
    )?;
    let mut head = remote_head(repo.path(), config)?;
    let mut files = Files::new();
    if head.is_some() {
        let reference = format!("refs/heads/{}", config.branch);
        git(
            repo.path(),
            &[
                "fetch",
                "--no-tags",
                "--no-recurse-submodules",
                "--",
                &config.remote,
                &reference,
            ],
            None,
        )?;
        // FETCH_HEAD, rather than the preceding ls-remote, is the snapshot we
        // actually inspected. A later move is detected before publication.
        let fetched = git_text(repo.path(), &["rev-parse", "FETCH_HEAD^{commit}"])?;
        let listing = git(repo.path(), &["ls-tree", "-rz", "--long", &fetched], None)?;
        let mut count = 0;
        for entry in listing.split(|b| *b == 0).filter(|e| !e.is_empty()) {
            count += 1;
            if count > MAX_PROFILES * 2 + 2 {
                return Err(fail("Remote contains too many files"));
            }
            let line =
                std::str::from_utf8(entry).map_err(|_| fail("Remote has a non-UTF-8 path"))?;
            let (meta, path) = line
                .split_once('\t')
                .ok_or_else(|| fail("Invalid remote tree"))?;
            let columns: Vec<_> = meta.split_whitespace().collect();
            if columns.len() != 4
                || columns[0] != "100644"
                || columns[1] != "blob"
                || columns[3].parse::<usize>().unwrap_or(usize::MAX) > MAX_FILE
            {
                return Err(fail(format!(
                    "Remote entry must be a regular file under 1 MiB: {path}"
                )));
            }
            // Preserve an existing remote .gitignore verbatim, but never import
            // it locally or use it to decide what can be uploaded.
            if path == ".gitignore" {
                continue;
            }
            if !allowed_source(path) {
                return Err(fail(format!(
                    "Remote contains a path outside the source allowlist: {path}"
                )));
            }
            let content =
                String::from_utf8(git(repo.path(), &["cat-file", "blob", columns[2]], None)?)
                    .map_err(|_| fail(format!("Remote source is not UTF-8: {path}")))?;
            files.insert(path.to_string(), content);
        }
        validate_sources(&files)?;
        // A fileless initial commit (or only the permitted .gitignore) is an
        // empty sync target too. Preserve its history when publishing sources.
        head = Some(fetched);
    }
    Ok((repo, head, files))
}

fn group(files: &Files, id: &str) -> Files {
    if id == "schema" {
        return files
            .iter()
            .filter(|(p, _)| p.as_str() == SCHEMA_FILE)
            .map(|(p, c)| (p.clone(), c.clone()))
            .collect();
    }
    let prefix = format!(
        "profiles/{}/",
        id.strip_prefix("profile:").unwrap_or_default()
    );
    files
        .iter()
        .filter(|(p, _)| p.starts_with(&prefix))
        .map(|(p, c)| (p.clone(), c.clone()))
        .collect()
}

fn pick<T: PartialEq + Clone>(base: &T, local: &T, remote: &T) -> Option<T> {
    if local == remote || remote == base {
        Some(local.clone())
    } else if local == base {
        Some(remote.clone())
    } else {
        None
    }
}

fn combine(base: &Files, local: &Files, remote: &Files, id: &str) -> Option<Files> {
    if let Some(result) = pick(base, local, remote) {
        return Some(result);
    }
    if id == "schema" || base.is_empty() || local.is_empty() || remote.is_empty() {
        return None;
    }
    let profile = id.strip_prefix("profile:")?;
    let manifest = format!("profiles/{profile}/profile.json");
    let rules = format!("profiles/{profile}/AGENTS.md");
    let b: serde_json::Value = serde_json::from_str(base.get(&manifest)?).ok()?;
    let l: serde_json::Value = serde_json::from_str(local.get(&manifest)?).ok()?;
    let r: serde_json::Value = serde_json::from_str(remote.get(&manifest)?).ok()?;
    let mut merged = serde_json::Map::new();
    let keys: BTreeSet<_> = b
        .as_object()?
        .keys()
        .chain(l.as_object()?.keys())
        .chain(r.as_object()?.keys())
        .collect();
    for key in keys {
        if let Some(value) = pick(&b.get(key), &l.get(key), &r.get(key))? {
            merged.insert(key.clone(), value.clone());
        }
    }
    let merged = serde_json::Value::Object(merged);
    let metadata = if merged == l {
        local.get(&manifest)?.clone()
    } else if merged == r {
        remote.get(&manifest)?.clone()
    } else {
        format!("{}\n", serde_json::to_string_pretty(&merged).ok()?)
    };
    let content = pick(&base.get(&rules), &local.get(&rules), &remote.get(&rules))??;
    Some(
        [(manifest, metadata), (rules, content.clone())]
            .into_iter()
            .collect(),
    )
}

fn versions(files: &Files) -> Vec<GitSyncVersion> {
    files
        .iter()
        .map(|(p, c)| GitSyncVersion {
            path: p.clone(),
            content: c.clone(),
        })
        .collect()
}

fn duplicate(remote: &Files, id: &str, new_id: &str) -> Result<Files, ArmError> {
    validate_id(new_id)?;
    let old_prefix = format!(
        "profiles/{}/",
        id.strip_prefix("profile:").unwrap_or_default()
    );
    let mut copy = Files::new();
    for (path, content) in remote {
        let filename = path
            .strip_prefix(&old_prefix)
            .ok_or_else(|| fail("Invalid conflict Profile"))?;
        let content = if filename == PROFILE_MANIFEST_FILE {
            let mut value: serde_json::Value = serde_json::from_str(content)?;
            value["id"] = new_id.into();
            format!("{}\n", serde_json::to_string_pretty(&value)?)
        } else {
            content.clone()
        };
        copy.insert(format!("profiles/{new_id}/{filename}"), content);
    }
    Ok(copy)
}

fn prepare(root: &Path, state: &Path, request: &GitSyncRequest) -> Result<PreparedSync, ArmError> {
    let config = GitSyncConfig {
        remote: request.remote.clone(),
        branch: request.branch.clone(),
    };
    validate_config(&config)?;
    let local = local_sources(root)?;
    let (state_path, state_before, stored) = read_stored(root, state)?;
    validate_path(&state.join("library-backups"))?;
    let machine_path = state.join(MACHINE_FILE);
    validate_path(&machine_path)?;
    let machine_before = read_file_state(&machine_path)?;
    let machine = read_machine(state)?;
    let (repo, remote_commit, remote) = fetch(&config)?;
    let connected = stored.as_ref().filter(|s| s.config == config);
    let base = if let Some(s) = connected {
        // State is written before publication. If a push was interrupted or
        // failed, only use its proposed baseline if the remote contains it.
        let published = match (&s.published_commit, &remote_commit) {
            (Some(a), Some(b)) => {
                a == b || git(repo.path(), &["merge-base", "--is-ancestor", a, b], None).is_ok()
            }
            _ => false,
        };
        if published {
            &s.baseline
        } else {
            &s.previous_remote
        }
    } else {
        &Files::new()
    };
    let mut ids = BTreeSet::from(["schema".to_string()]);
    for files in [base, &local, &remote] {
        ids.extend(
            profile_ids(files)
                .into_iter()
                .map(|id| format!("profile:{id}")),
        );
    }
    let mut resolutions = BTreeMap::new();
    for resolution in &request.resolutions {
        if resolutions
            .insert(resolution.id.clone(), resolution)
            .is_some()
        {
            return Err(fail("Duplicate conflict decision"));
        }
    }
    let mut merged = Files::new();
    let mut conflicts = Vec::new();
    let mut blockers = Vec::new();
    let mut reserved = profile_ids(&local);
    reserved.extend(profile_ids(&remote));
    for id in ids {
        let b = group(base, &id);
        let l = group(&local, &id);
        let r = group(&remote, &id);
        if let Some(combined) = combine(&b, &l, &r, &id) {
            if resolutions.contains_key(&id) {
                return Err(fail(
                    "A conflict decision no longer matches this preview; check again",
                ));
            }
            merged.extend(combined);
            continue;
        }
        let can_keep_both = id != "schema" && !l.is_empty() && !r.is_empty();
        let decision = resolutions.remove(&id);
        conflicts.push(GitSyncConflict {
            id: id.clone(),
            base: versions(&b),
            local: versions(&l),
            remote: versions(&r),
            can_keep_both,
            resolved: decision.is_some(),
        });
        match decision {
            None => merged.extend(l),
            Some(d) => match d.choice {
                GitSyncChoice::KeepLocal => merged.extend(l),
                GitSyncChoice::UseRemote => merged.extend(r),
                GitSyncChoice::KeepBoth => {
                    if !can_keep_both {
                        return Err(fail("Keep both requires two existing Profile versions"));
                    }
                    let new_id = d
                        .new_profile_id
                        .as_deref()
                        .ok_or_else(|| fail("Choose a new Profile id for the remote copy"))?;
                    if !reserved.insert(new_id.to_string())
                        || path_entry_exists(&root.join(PROFILES_DIR).join(new_id))?
                    {
                        return Err(fail(format!("Profile id already exists: {new_id}")));
                    }
                    merged.extend(duplicate(&r, &id, new_id)?);
                    merged.extend(l);
                }
            },
        }
    }
    if !resolutions.is_empty() {
        return Err(fail("Unknown conflict decision; check again"));
    }
    validate_sources(&merged)?;
    if merged.is_empty() {
        blockers.push(
            "Initialize a local rule library or choose a remote containing a v3 library".into(),
        );
    }
    let remaining = profile_ids(&merged);
    if let Some(active) = machine.map(|m| m.active_profile_id) {
        if !remaining.contains(&active) {
            blockers.push(format!("Sync would delete active Profile `{active}`. Activate another Profile before syncing."));
        }
    }
    let mut deleted_directories = Vec::new();
    for id in profile_ids(&local).difference(&remaining) {
        let loaded = load_library(root)?;
        let profile = &loaded.profiles[id];
        let directory = root.join(PROFILES_DIR).join(id);
        if let Some(reason) = profile_deletion_blocker(&directory, state, profile)? {
            blockers.push(reason);
        }
        deleted_directories.push(directory);
    }
    let paths: BTreeSet<_> = local
        .keys()
        .chain(remote.keys())
        .chain(merged.keys())
        .cloned()
        .collect();
    let mut changes = Vec::new();
    for path in paths {
        let after = merged.get(&path).cloned();
        if local.get(&path) == after.as_ref() && remote.get(&path) == after.as_ref() {
            continue;
        }
        let target = root.join(&path);
        validate_path(&target)?;
        if !local.contains_key(&path) && path_entry_exists(&target)? {
            blockers.push(format!("Unmanaged target already exists: {path}"));
        }
        changes.push(GitSyncChange {
            local_before: local.get(&path).cloned(),
            remote_before: remote.get(&path).cloned(),
            path,
            local_after: after.clone(),
            remote_after: after,
        });
    }
    let blocked = !blockers.is_empty() || conflicts.iter().any(|c| !c.resolved);
    let mut directories: Vec<_> = deleted_directories
        .iter()
        .map(|path| GitSyncDirectoryChange {
            path: path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            action: "remove".into(),
        })
        .collect();
    if !merged.is_empty() && !path_entry_exists(&root.join(PROFILES_DIR))? {
        directories.push(GitSyncDirectoryChange {
            path: PROFILES_DIR.into(),
            action: "create".into(),
        });
    }
    let mut plan = GitSyncPlan {
        token: String::new(),
        config,
        remote_commit,
        changes,
        directories,
        conflicts,
        blockers,
        blocked,
        connection_changed: connected.is_none(),
    };
    plan.token = full_digest(&serde_json::to_vec(&(
        &plan,
        request,
        &local,
        &remote,
        &merged,
        &state_before,
        &machine_before,
        root,
        state,
    ))?);
    Ok(PreparedSync {
        plan,
        repo,
        local,
        remote,
        merged,
        state_path,
        state_before,
        machine_before,
        deleted_directories,
    })
}

pub(crate) fn preview(
    root: &Path,
    state: &Path,
    request: &GitSyncRequest,
) -> Result<GitSyncPlan, ArmError> {
    Ok(prepare(root, state, request)?.plan)
}

fn commit(prepared: &PreparedSync) -> Result<Option<String>, ArmError> {
    let repo = prepared.repo.path();
    if let Some(head) = &prepared.plan.remote_commit {
        git(repo, &["read-tree", head], None)?;
    } else {
        git(repo, &["read-tree", "--empty"], None)?;
    }
    for path in prepared
        .remote
        .keys()
        .filter(|p| !prepared.merged.contains_key(*p))
    {
        // --force-remove consults a worktree even in a bare repository. The
        // index-info deletion record only changes our isolated temporary index.
        git(
            repo,
            &["update-index", "--index-info"],
            Some(format!("0 {}\t{path}\n", "0".repeat(40)).as_bytes()),
        )?;
    }
    for (path, content) in &prepared.merged {
        let oid = String::from_utf8(git(
            repo,
            &["hash-object", "-w", "--stdin"],
            Some(content.as_bytes()),
        )?)
        .map_err(|_| fail("Invalid Git object id"))?;
        git(
            repo,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("100644,{},{}", oid.trim(), path),
            ],
            None,
        )?;
    }
    let tree = git_text(repo, &["write-tree"])?;
    if let Some(head) = &prepared.plan.remote_commit {
        if git_text(repo, &["rev-parse", &format!("{head}^{{tree}}")])? == tree {
            return Ok(Some(head.clone()));
        }
    }
    let mut args = vec!["commit-tree", &tree, "-m", "Sync rule library sources"];
    if let Some(head) = &prepared.plan.remote_commit {
        args.extend(["-p", head]);
    }
    Ok(Some(git_text(repo, &args)?))
}

pub(crate) fn apply(
    root: &Path,
    state: &Path,
    request: &GitSyncRequest,
    token: &str,
) -> Result<GitSyncOutcome, ArmError> {
    let prepared = prepare(root, state, request)?;
    if prepared.plan.blocked || prepared.plan.token != token {
        return Err(fail(
            "Git sync preview is stale or unresolved; preview again",
        ));
    }
    let published_commit = commit(&prepared)?;
    if local_sources(root)? != prepared.local
        || read_file_state(&prepared.state_path)? != prepared.state_before
        || read_file_state(&state.join(MACHINE_FILE))? != prepared.machine_before
    {
        return Err(fail(
            "Local files or selection changed after preview; preview again",
        ));
    }
    if remote_head(prepared.repo.path(), &prepared.plan.config)? != prepared.plan.remote_commit {
        return Err(fail("Remote changed after preview; preview again"));
    }
    let stored = StoredSync {
        version: 1,
        config: prepared.plan.config.clone(),
        published_commit: published_commit.clone(),
        baseline: prepared.merged.clone(),
        previous_remote: prepared.remote.clone(),
    };
    let mut changes = Vec::new();
    for change in &prepared.plan.changes {
        if change.local_before == change.local_after {
            continue;
        }
        let path = root.join(&change.path);
        validate_path(&path)?;
        changes.push(PreparedChange {
            path,
            original: to_state(&change.local_before),
            desired: to_state(&change.local_after),
        });
    }
    let desired = FileState::File {
        content: format!("{}\n", serde_json::to_string_pretty(&stored)?),
    };
    if desired != prepared.state_before {
        changes.push(PreparedChange {
            path: prepared.state_path.clone(),
            original: prepared.state_before.clone(),
            desired,
        });
    }
    let mut outcome = apply_changes(state, "gitSync", changes)?;
    if let Some(id) = &outcome.backup_id {
        let path = library_backup_path(state, id);
        let update = (|| {
            let mut backup: serde_json::Value = read_json(&path)?;
            backup["removedDirectories"] = serde_json::to_value(&prepared.deleted_directories)?;
            if prepared
                .plan
                .directories
                .iter()
                .any(|d| d.action == "create")
            {
                let directory = root.join(PROFILES_DIR).display().to_string();
                let entries = backup["createdDirectories"]
                    .as_array_mut()
                    .ok_or_else(|| fail("Invalid sync recovery snapshot"))?;
                if !entries.iter().any(|p| p.as_str() == Some(&directory)) {
                    entries.push(directory.into());
                }
            }
            write_atomic(
                &path,
                &format!("{}\n", serde_json::to_string_pretty(&backup)?),
            )
        })();
        if let Err(e) = update {
            rollback_library_backup(state, &path)?;
            return Err(e);
        }
    }
    if prepared
        .plan
        .directories
        .iter()
        .any(|d| d.action == "create")
    {
        let path = root.join(PROFILES_DIR);
        validate_path(&path)?;
        if let Err(error) = fs::create_dir_all(&path) {
            if let Some(id) = &outcome.backup_id {
                rollback_library_backup(state, &library_backup_path(state, id))?;
            }
            return Err(ArmError::io(path.display().to_string(), error));
        }
    }
    if let Err(error) = remove_profile_directories(&prepared.deleted_directories) {
        if let Some(id) = &outcome.backup_id {
            rollback_library_backup(state, &library_backup_path(state, id))?;
        }
        return Err(error);
    }
    outcome.changed.extend(
        prepared
            .deleted_directories
            .iter()
            .map(|p| p.display().to_string()),
    );
    let needs_push = published_commit != prepared.plan.remote_commit;
    let push_error = if needs_push {
        publish(&prepared, published_commit.as_deref().unwrap())
            .err()
            .map(|e| e.to_string())
    } else {
        None
    };
    Ok(GitSyncOutcome {
        changed: outcome.changed,
        backup_id: outcome.backup_id,
        pushed: needs_push && push_error.is_none(),
        push_error,
    })
}

fn publish(prepared: &PreparedSync, commit: &str) -> Result<(), ArmError> {
    let repo = prepared.repo.path();
    // Only a descendant of the reviewed remote commit may be published. The
    // explicit lease is a compare-and-swap guard, never permission to rewrite
    // history: it also rejects deletion/rewind races that normal FF push misses.
    if let Some(parent) = &prepared.plan.remote_commit {
        git(repo, &["merge-base", "--is-ancestor", parent, commit], None)
            .map_err(|_| fail("Refusing a publication that would rewrite remote history"))?;
    }
    let branch = format!("refs/heads/{}", prepared.plan.config.branch);
    let lease = format!(
        "--force-with-lease={}:{}",
        branch,
        prepared.plan.remote_commit.as_deref().unwrap_or_default()
    );
    let reference = format!("{commit}:{branch}");
    git(
        repo,
        &[
            "push",
            "--porcelain",
            &lease,
            "--",
            &prepared.plan.config.remote,
            &reference,
        ],
        None,
    )?;
    Ok(())
}

fn to_state(value: &Option<String>) -> FileState {
    value
        .as_ref()
        .map(|content| FileState::File {
            content: content.clone(),
        })
        .unwrap_or(FileState::Missing)
}

use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::process::Command;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OpenTargetKind {
    Default,
    Editor,
    Reveal,
    Terminal,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenTarget {
    pub id: String,
    pub label: String,
    pub kind: OpenTargetKind,
}

#[derive(Debug, Clone)]
struct ResolvedOpenTarget {
    public: OpenTarget,
    program: Option<String>,
}

impl ResolvedOpenTarget {
    fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        kind: OpenTargetKind,
        program: Option<String>,
    ) -> Self {
        Self {
            public: OpenTarget {
                id: id.into(),
                label: label.into(),
                kind,
            },
            program,
        }
    }
}

pub fn list_open_targets() -> Vec<OpenTarget> {
    resolved_open_targets()
        .into_iter()
        .map(|target| target.public)
        .collect()
}

pub fn open_source(app: &AppHandle, target_id: &str, source: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("cannot open {}: {error}", source.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "rules source is not a regular file: {}",
            source.display()
        ));
    }

    let target = resolved_open_targets()
        .into_iter()
        .find(|target| target.public.id == target_id)
        .ok_or_else(|| format!("open target is unavailable: {target_id}"))?;
    let source_value = source.to_string_lossy().into_owned();

    match target.public.kind {
        OpenTargetKind::Default => app
            .opener()
            .open_path(source_value, None::<String>)
            .map_err(|error| error.to_string()),
        OpenTargetKind::Editor => {
            let program = target
                .program
                .ok_or_else(|| format!("open target has no program: {target_id}"))?;
            app.opener()
                .open_path(source_value, Some(program))
                .map_err(|error| error.to_string())
        }
        OpenTargetKind::Reveal => app
            .opener()
            .reveal_item_in_dir(source)
            .map_err(|error| error.to_string()),
        OpenTargetKind::Terminal => {
            let directory = source
                .parent()
                .ok_or_else(|| format!("rules source has no parent: {}", source.display()))?;
            open_terminal(app, &target, directory)
        }
    }
}

fn resolved_open_targets() -> Vec<ResolvedOpenTarget> {
    let mut targets = vec![ResolvedOpenTarget::new(
        "default",
        "Default app",
        OpenTargetKind::Default,
        None,
    )];
    extend_platform_targets(&mut targets);
    targets
}

#[cfg(target_os = "macos")]
fn extend_platform_targets(targets: &mut Vec<ResolvedOpenTarget>) {
    let home = env::var_os("HOME").map(PathBuf::from);
    let editors = [
        ("vscode", "Visual Studio Code", "Visual Studio Code"),
        ("cursor", "Cursor", "Cursor"),
        ("typora", "Typora", "Typora"),
        ("textedit", "TextEdit", "TextEdit"),
        ("intellij-idea", "IntelliJ IDEA", "IntelliJ IDEA"),
        ("rider", "Rider", "Rider"),
        ("webstorm", "WebStorm", "WebStorm"),
    ];

    for (id, label, app_name) in editors {
        if mac_app_installed(home.as_deref(), app_name) {
            targets.push(ResolvedOpenTarget::new(
                id,
                label,
                OpenTargetKind::Editor,
                Some(app_name.to_string()),
            ));
        }
    }

    targets.push(ResolvedOpenTarget::new(
        "finder",
        "Finder",
        OpenTargetKind::Reveal,
        None,
    ));
    if mac_app_installed(home.as_deref(), "Terminal") {
        targets.push(ResolvedOpenTarget::new(
            "terminal",
            "Terminal",
            OpenTargetKind::Terminal,
            Some("Terminal".into()),
        ));
    }
}

#[cfg(target_os = "macos")]
fn mac_app_installed(home: Option<&Path>, app_name: &str) -> bool {
    let bundle_name = format!("{app_name}.app");
    let mut roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home) = home {
        roots.push(home.join("Applications"));
    }
    roots.iter().any(|root| root.join(&bundle_name).is_dir())
}

#[cfg(target_os = "macos")]
fn open_terminal(
    app: &AppHandle,
    target: &ResolvedOpenTarget,
    directory: &Path,
) -> Result<(), String> {
    let program = target
        .program
        .clone()
        .ok_or_else(|| "terminal target has no application".to_string())?;
    app.opener()
        .open_path(directory.to_string_lossy().into_owned(), Some(program))
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn extend_platform_targets(targets: &mut Vec<ResolvedOpenTarget>) {
    let local_app_data = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let program_files = env::var_os("ProgramFiles").map(PathBuf::from);
    let program_files_x86 = env::var_os("ProgramFiles(x86)").map(PathBuf::from);

    let path_roots: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();
    let mut install_roots = Vec::new();
    if let Some(root) = &local_app_data {
        install_roots.push(root.join("Programs"));
    }
    if let Some(root) = &program_files {
        install_roots.push(root.join("Microsoft VS Code"));
        install_roots.push(root.join("Typora"));
        install_roots.push(root.join("JetBrains"));
    }
    if let Some(root) = &program_files_x86 {
        install_roots.push(root.join("Microsoft VS Code"));
        install_roots.push(root.join("Typora"));
        install_roots.push(root.join("JetBrains"));
    }

    let editors = [
        ("vscode", "Visual Studio Code", &["Code.exe"][..]),
        ("cursor", "Cursor", &["Cursor.exe"][..]),
        ("typora", "Typora", &["Typora.exe"][..]),
        (
            "intellij-idea",
            "IntelliJ IDEA",
            &["idea64.exe", "idea.exe"][..],
        ),
        ("rider", "Rider", &["rider64.exe", "rider.exe"][..]),
        (
            "webstorm",
            "WebStorm",
            &["webstorm64.exe", "webstorm.exe"][..],
        ),
    ];
    for (id, label, names) in editors {
        let program =
            find_program(names, &path_roots, 0).or_else(|| find_program(names, &install_roots, 4));
        if let Some(program) = program {
            targets.push(ResolvedOpenTarget::new(
                id,
                label,
                OpenTargetKind::Editor,
                Some(program),
            ));
        }
    }

    targets.push(ResolvedOpenTarget::new(
        "notepad",
        "Notepad",
        OpenTargetKind::Editor,
        Some("notepad.exe".into()),
    ));
    targets.push(ResolvedOpenTarget::new(
        "explorer",
        "File Explorer",
        OpenTargetKind::Reveal,
        None,
    ));
    if let Some(program) = find_program(&["wt.exe"], &path_roots, 0) {
        targets.push(ResolvedOpenTarget::new(
            "windows-terminal",
            "Windows Terminal",
            OpenTargetKind::Terminal,
            Some(program),
        ));
    }
}

#[cfg(target_os = "windows")]
fn open_terminal(
    _app: &AppHandle,
    target: &ResolvedOpenTarget,
    directory: &Path,
) -> Result<(), String> {
    let program = target
        .program
        .as_deref()
        .ok_or_else(|| "terminal target has no executable".to_string())?;
    Command::new(program)
        .arg("-d")
        .arg(directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("cannot open Windows Terminal: {error}"))
}

#[cfg(target_os = "linux")]
fn extend_platform_targets(targets: &mut Vec<ResolvedOpenTarget>) {
    let roots: Vec<PathBuf> = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default();
    let editors = [
        ("vscode", "Visual Studio Code", &["code"][..]),
        ("cursor", "Cursor", &["cursor"][..]),
        ("typora", "Typora", &["typora"][..]),
        ("intellij-idea", "IntelliJ IDEA", &["idea"][..]),
        ("rider", "Rider", &["rider"][..]),
        ("webstorm", "WebStorm", &["webstorm"][..]),
    ];
    for (id, label, names) in editors {
        if let Some(program) = find_program(names, &roots, 0) {
            targets.push(ResolvedOpenTarget::new(
                id,
                label,
                OpenTargetKind::Editor,
                Some(program),
            ));
        }
    }

    targets.push(ResolvedOpenTarget::new(
        "files",
        "Files",
        OpenTargetKind::Reveal,
        None,
    ));
    let terminals = [
        (
            "terminal",
            "Terminal",
            &["gnome-terminal", "x-terminal-emulator"][..],
        ),
        ("konsole", "Konsole", &["konsole"][..]),
        ("xfce-terminal", "Xfce Terminal", &["xfce4-terminal"][..]),
    ];
    for (id, label, names) in terminals {
        if let Some(program) = find_program(names, &roots, 0) {
            targets.push(ResolvedOpenTarget::new(
                id,
                label,
                OpenTargetKind::Terminal,
                Some(program),
            ));
            break;
        }
    }
}

#[cfg(target_os = "linux")]
fn open_terminal(
    _app: &AppHandle,
    target: &ResolvedOpenTarget,
    directory: &Path,
) -> Result<(), String> {
    let program = target
        .program
        .as_deref()
        .ok_or_else(|| "terminal target has no executable".to_string())?;
    let mut command = Command::new(program);
    command.current_dir(directory);
    match target.public.id.as_str() {
        "konsole" => {
            command.arg("--workdir").arg(directory);
        }
        "xfce-terminal" => {
            command.arg(format!("--working-directory={}", directory.display()));
        }
        _ if program.ends_with("gnome-terminal") => {
            command.arg(format!("--working-directory={}", directory.display()));
        }
        _ => {}
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("cannot open terminal: {error}"))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn find_program(names: &[&str], roots: &[PathBuf], max_depth: usize) -> Option<String> {
    for root in roots {
        for name in names {
            if let Some(path) = find_named_file(root, name, max_depth) {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn find_named_file(root: &Path, name: &str, max_depth: usize) -> Option<PathBuf> {
    let direct = root.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    if max_depth == 0 || !root.is_dir() {
        return None;
    }
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_named_file(&path, name, max_depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn extend_platform_targets(_targets: &mut Vec<ResolvedOpenTarget>) {}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn open_terminal(
    _app: &AppHandle,
    _target: &ResolvedOpenTarget,
    _directory: &Path,
) -> Result<(), String> {
    Err("opening a terminal is not supported on this platform".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn default_target_is_always_first() {
        let targets = list_open_targets();
        assert_eq!(
            targets.first().map(|target| target.id.as_str()),
            Some("default")
        );
    }

    #[test]
    fn open_target_ids_are_unique() {
        let targets = list_open_targets();
        let ids = targets
            .iter()
            .map(|target| target.id.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), targets.len());
    }
}

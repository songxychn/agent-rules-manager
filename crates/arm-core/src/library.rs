use crate::{
    ArmError, LibraryMutationOutcome, LibraryPlan, LibraryPlanStep, LibraryState, ProfileSummary,
    RollbackOutcome, RuleFileSummary, RulePackSummary, RuntimeState,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

const SCHEMA_VERSION: u32 = 1;
const SCHEMA_FILE: &str = "schema.json";
const PACKS_DIR: &str = "packs";
const PROFILES_DIR: &str = "profiles";
const RUNTIME_DIR: &str = ".runtime";
const CURRENT_LINK: &str = "current";
const MACHINE_FILE: &str = "machine.json";
const LEGACY_SOURCE_FILE: &str = "AGENTS.md";
const PACK_MANIFEST_FILE: &str = "pack.json";
const ENTRYPOINT_FILE: &str = "AGENTS.md";

const STARTER_RULES: &str = r#"# Shared agent rules

These instructions apply to every supported coding agent unless a project-level rule is more specific.

## Working style

- Read the relevant code and instructions before editing.
- Keep changes scoped to the requested behavior.
- Verify changes in proportion to risk.
"#;

const LIBRARY_GITIGNORE: &str = ".runtime/\ncurrent\n";

#[derive(Debug, Clone)]
pub(crate) struct LibraryInspection {
    pub state: LibraryState,
    pub detail: String,
    pub runtime_state: RuntimeState,
    pub source_path: PathBuf,
    pub source_exists: bool,
    pub source_digest: Option<String>,
    pub source_modified_at: Option<String>,
    pub active_profile_id: Option<String>,
    pub active_source_path: Option<PathBuf>,
    pub legacy_source_path: Option<PathBuf>,
    pub legacy_adapter_source_path: Option<PathBuf>,
    pub packs: Vec<RulePackSummary>,
    pub profiles: Vec<ProfileSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemaDocument {
    schema_version: u32,
    format: String,
    sync: SyncPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    legacy_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPolicy {
    include: Vec<String>,
    exclude: Vec<String>,
    machine_selection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackDocument {
    schema_version: u32,
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    instructions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileDocument {
    schema_version: u32,
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    packs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MachineDocument {
    schema_version: u32,
    active_profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RuntimeSource {
    pack_id: String,
    path: String,
    digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RuntimeManifest {
    schema_version: u32,
    profile_id: String,
    profile_digest: String,
    rendered_digest: String,
    packs: Vec<String>,
    sources: Vec<RuntimeSource>,
}

#[derive(Debug, Clone)]
struct LoadedPack {
    document: PackDocument,
    files: Vec<LoadedRuleFile>,
}

#[derive(Debug, Clone)]
struct LoadedRuleFile {
    relative_path: String,
    path: PathBuf,
    content: String,
    digest: String,
    modified_at: Option<String>,
}

#[derive(Debug, Clone)]
struct LoadedLibrary {
    packs: BTreeMap<String, LoadedPack>,
    profiles: BTreeMap<String, ProfileDocument>,
    legacy_source_hint: Option<String>,
}

#[derive(Debug, Clone)]
struct RenderedProfile {
    runtime_name: String,
    content: String,
    manifest: RuntimeManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum FileState {
    Missing,
    File { content: String },
    Symlink { target: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupEntry {
    target_path: String,
    original: FileState,
    applied_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupSnapshot {
    id: String,
    created_at: String,
    operation: String,
    entries: Vec<BackupEntry>,
}

#[derive(Debug, Clone)]
struct PreparedChange {
    path: PathBuf,
    original: FileState,
    desired: FileState,
}

pub(crate) fn source_path(library_root: &Path) -> PathBuf {
    library_root.join(CURRENT_LINK).join(ENTRYPOINT_FILE)
}

pub(crate) fn inspect(
    library_root: &Path,
    state_root: &Path,
) -> Result<LibraryInspection, ArmError> {
    let source = source_path(library_root);
    let legacy = regular_file(library_root.join(LEGACY_SOURCE_FILE))?;
    let (state, detail) = detect_library_state(library_root)?;
    if state != LibraryState::Ready {
        return Ok(LibraryInspection {
            state,
            detail,
            runtime_state: RuntimeState::Missing,
            source_path: source,
            source_exists: false,
            source_digest: None,
            source_modified_at: None,
            active_profile_id: None,
            active_source_path: None,
            legacy_source_path: legacy.clone(),
            legacy_adapter_source_path: legacy,
            packs: Vec::new(),
            profiles: Vec::new(),
        });
    }

    let loaded = load_library(library_root)?;
    let legacy_adapter_source_path = legacy.clone().or_else(|| {
        loaded
            .legacy_source_hint
            .as_ref()
            .map(|relative| library_root.join(relative))
    });
    let machine = read_machine(state_root)?;
    let active_profile_id = machine.as_ref().map(|value| value.active_profile_id.clone());
    let mut runtime_state = RuntimeState::Missing;
    let mut active_source_path = None;
    let runtime_detail;

    if let Some(active_id) = &active_profile_id {
        let profile = loaded
            .profiles
            .get(active_id)
            .ok_or_else(|| ArmError::UnknownProfile(active_id.clone()))?;
        let rendered = render_profile(&loaded, profile)?;
        active_source_path = profile
            .packs
            .first()
            .and_then(|pack_id| loaded.packs.get(pack_id))
            .and_then(|pack| pack.files.first())
            .map(|file| file.path.clone());
        runtime_state = inspect_current(library_root, &rendered)?;
        runtime_detail = match runtime_state {
            RuntimeState::Current => format!("Profile `{active_id}` is rendered and selected on this machine."),
            RuntimeState::Missing => format!("Profile `{active_id}` is selected but has not been rendered."),
            RuntimeState::Stale => format!("Profile `{active_id}` changed after the current runtime was rendered."),
            RuntimeState::Conflict => "The `current` entry is not a managed runtime link.".into(),
        };
    } else if path_entry_exists(&library_root.join(CURRENT_LINK))? {
        runtime_state = RuntimeState::Conflict;
        runtime_detail = "A `current` entry exists without a machine-local profile selection.".into();
    } else {
        runtime_detail = "Choose a profile for this machine before connecting agents.".into();
    }

    let (source_exists, source_digest, source_modified_at) = inspect_rendered_source(&source)?;
    let packs = loaded
        .packs
        .values()
        .map(|pack| RulePackSummary {
            id: pack.document.id.clone(),
            name: pack.document.name.clone(),
            description: pack.document.description.clone(),
            files: pack
                .files
                .iter()
                .map(|file| RuleFileSummary {
                    path: file.relative_path.clone(),
                    digest: file.digest.clone(),
                    modified_at: file.modified_at.clone(),
                })
                .collect(),
        })
        .collect();
    let profiles = loaded
        .profiles
        .values()
        .map(|profile| ProfileSummary {
            id: profile.id.clone(),
            name: profile.name.clone(),
            description: profile.description.clone(),
            pack_ids: profile.packs.clone(),
            is_active: active_profile_id.as_deref() == Some(profile.id.as_str()),
        })
        .collect();

    Ok(LibraryInspection {
        state,
        detail: runtime_detail,
        runtime_state,
        source_path: source,
        source_exists,
        source_digest,
        source_modified_at,
        active_profile_id,
        active_source_path,
        legacy_source_path: legacy,
        legacy_adapter_source_path,
        packs,
        profiles,
    })
}

pub(crate) fn plan_initialize(
    library_root: &Path,
    state_root: &Path,
) -> Result<LibraryPlan, ArmError> {
    let (state, detail) = detect_library_state(library_root)?;
    let mut steps = Vec::new();
    match state {
        LibraryState::Ready => {
            return Ok(LibraryPlan {
                operation: "initialize".into(),
                blocked: false,
                change_count: 0,
                summary: "The rule library is already initialized.".into(),
                steps,
            });
        }
        LibraryState::Conflict => {
            return Ok(LibraryPlan {
                operation: "initialize".into(),
                blocked: true,
                change_count: 0,
                summary: detail,
                steps,
            });
        }
        LibraryState::Empty | LibraryState::Legacy => {}
    }

    let imported = state == LibraryState::Legacy;
    let initial_rules = if imported {
        let path = library_root.join(LEGACY_SOURCE_FILE);
        fs::read_to_string(&path)
            .map_err(|error| ArmError::io(path.display().to_string(), error))?
    } else {
        STARTER_RULES.into()
    };
    let initial_runtime = render_default_profile(library_root, &initial_rules)?;
    steps.push(plan_step(
        library_root.join(SCHEMA_FILE),
        "createSchema",
        "Create the versioned, sync-safe library schema.",
    ));
    steps.push(plan_step(
        library_root.join(PACKS_DIR).join("base").join(PACK_MANIFEST_FILE),
        "createPack",
        "Create the base Rule Pack manifest with AGENTS.md as its required entrypoint.",
    ));
    steps.push(plan_step(
        library_root.join(PACKS_DIR).join("base").join(ENTRYPOINT_FILE),
        if imported { "copyLegacy" } else { "createRules" },
        if imported {
            "Copy the exact legacy AGENTS.md content into the base Rule Pack."
        } else {
            "Create the base Rule Pack AGENTS.md."
        },
    ));
    steps.push(plan_step(
        library_root.join(PROFILES_DIR).join("default.json"),
        "createProfile",
        "Create a default Profile that selects the base Rule Pack.",
    ));
    if !path_entry_exists(&library_root.join(".gitignore"))? {
        steps.push(plan_step(
            library_root.join(".gitignore"),
            "createIgnore",
            "Keep .runtime and current out of multi-machine Git sync.",
        ));
    }
    if imported {
        steps.push(plan_step(
            library_root.join(LEGACY_SOURCE_FILE),
            "removeLegacy",
            "Remove the legacy root file only after its exact content is backed up and copied.",
        ));
    }
    steps.push(plan_step(
        library_root
            .join(RUNTIME_DIR)
            .join(&initial_runtime.runtime_name),
        "renderRuntime",
        "Render the immutable default Profile runtime.",
    ));
    steps.push(plan_step(
        library_root.join(CURRENT_LINK),
        "switchCurrent",
        "Point the stable current directory link at the default runtime.",
    ));
    steps.push(plan_step(
        state_root.join(MACHINE_FILE),
        "selectLocalProfile",
        "Record the default Profile only in machine-local application state.",
    ));

    Ok(LibraryPlan {
        operation: "initialize".into(),
        blocked: false,
        change_count: steps.len(),
        summary: if imported {
            "Migrate the legacy AGENTS.md into the base Rule Pack with a rollback snapshot, remove the old root entry, then activate the default Profile."
                .into()
        } else {
            "Create a Rule Pack library and activate its default Profile.".into()
        },
        steps,
    })
}

pub(crate) fn initialize(
    library_root: &Path,
    state_root: &Path,
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_initialize(library_root, state_root)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    if plan.change_count == 0 {
        return Ok(LibraryMutationOutcome {
            changed: Vec::new(),
            backup_id: None,
        });
    }

    let legacy_path = library_root.join(LEGACY_SOURCE_FILE);
    let legacy_original = read_file_state(&legacy_path)?;
    let (rules, imported) = match &legacy_original {
        FileState::Missing => (STARTER_RULES.into(), false),
        FileState::File { content } => (content.clone(), true),
        FileState::Symlink { .. } => {
            return Err(ArmError::Blocked(format!(
                "the legacy source must be a regular file: {}",
                legacy_path.display()
            )))
        }
    };

    let schema = SchemaDocument {
        schema_version: SCHEMA_VERSION,
        format: "agent-rules-library".into(),
        sync: SyncPolicy {
            include: vec!["schema.json".into(), "packs/**".into(), "profiles/**".into()],
            exclude: vec![".runtime/**".into(), "current".into(), "machine state".into()],
            machine_selection: "local".into(),
        },
        legacy_source: imported.then(|| LEGACY_SOURCE_FILE.into()),
    };
    let pack = PackDocument {
        schema_version: SCHEMA_VERSION,
        id: "base".into(),
        name: "Base rules".into(),
        description: "Rules shared by every local profile.".into(),
        instructions: vec![ENTRYPOINT_FILE.into()],
    };
    let profile = ProfileDocument {
        schema_version: SCHEMA_VERSION,
        id: "default".into(),
        name: "Default".into(),
        description: "The initial machine profile.".into(),
        packs: vec!["base".into()],
    };

    let mut changes = vec![
        missing_file_change(library_root.join(SCHEMA_FILE), pretty_json(&schema)? + "\n")?,
        missing_file_change(
            library_root.join(PACKS_DIR).join("base").join(PACK_MANIFEST_FILE),
            pretty_json(&pack)? + "\n",
        )?,
        missing_file_change(
            library_root.join(PACKS_DIR).join("base").join(ENTRYPOINT_FILE),
            rules,
        )?,
        missing_file_change(
            library_root.join(PROFILES_DIR).join("default.json"),
            pretty_json(&profile)? + "\n",
        )?,
    ];
    if !path_entry_exists(&library_root.join(".gitignore"))? {
        changes.push(missing_file_change(
            library_root.join(".gitignore"),
            LIBRARY_GITIGNORE.into(),
        )?);
    }
    if imported {
        changes.push(PreparedChange {
            path: legacy_path,
            original: legacy_original,
            desired: FileState::Missing,
        });
    }

    let created = apply_changes(state_root, "initialize", changes)?;
    match activate_profile(library_root, state_root, "default") {
        Ok(mut activated) => {
            let mut changed = created.changed;
            changed.append(&mut activated.changed);
            Ok(LibraryMutationOutcome {
                changed,
                backup_id: activated.backup_id.or(created.backup_id),
            })
        }
        Err(error) => {
            let _ = rollback_library_latest(state_root);
            Err(error)
        }
    }
}

pub(crate) fn plan_create_pack(
    library_root: &Path,
    id: &str,
    name: &str,
    description: &str,
) -> Result<LibraryPlan, ArmError> {
    validate_id(id)?;
    validate_label("pack name", name, 80)?;
    validate_label("pack description", description, 240)?;
    ensure_ready(library_root)?;
    let directory = library_root.join(PACKS_DIR).join(id);
    let blocked = path_entry_exists(&directory)?;
    let steps = if blocked {
        Vec::new()
    } else {
        vec![
            plan_step(
                directory.join(PACK_MANIFEST_FILE),
                "createPack",
                "Create a versioned Rule Pack manifest.",
            ),
            plan_step(
                directory.join(ENTRYPOINT_FILE),
                "createRules",
                "Create the required AGENTS.md entrypoint.",
            ),
        ]
    };
    Ok(LibraryPlan {
        operation: "createPack".into(),
        blocked,
        change_count: steps.len(),
        summary: if blocked {
            format!("Rule Pack `{id}` already exists; no file will be overwritten.")
        } else {
            format!("Create Rule Pack `{name}` with a required AGENTS.md entrypoint.")
        },
        steps,
    })
}

pub(crate) fn create_pack(
    library_root: &Path,
    state_root: &Path,
    id: &str,
    name: &str,
    description: &str,
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_create_pack(library_root, id, name, description)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    let document = PackDocument {
        schema_version: SCHEMA_VERSION,
        id: id.into(),
        name: name.trim().into(),
        description: description.trim().into(),
        instructions: vec![ENTRYPOINT_FILE.into()],
    };
    let directory = library_root.join(PACKS_DIR).join(id);
    apply_changes(
        state_root,
        "createPack",
        vec![
            missing_file_change(
                directory.join(PACK_MANIFEST_FILE),
                pretty_json(&document)? + "\n",
            )?,
            missing_file_change(directory.join(ENTRYPOINT_FILE), STARTER_RULES.into())?,
        ],
    )
}

pub(crate) fn plan_add_pack_file(
    library_root: &Path,
    pack_id: &str,
    relative_path: &str,
) -> Result<LibraryPlan, ArmError> {
    validate_id(pack_id)?;
    validate_instruction_path(relative_path)?;
    let loaded = load_library(library_root)?;
    let pack = loaded
        .packs
        .get(pack_id)
        .ok_or_else(|| ArmError::UnknownPack(pack_id.into()))?;
    let file_path = library_root.join(PACKS_DIR).join(pack_id).join(relative_path);
    let already_declared = pack
        .document
        .instructions
        .iter()
        .any(|path| path == relative_path);
    let blocked = already_declared || path_entry_exists(&file_path)?;
    let steps = if blocked {
        Vec::new()
    } else {
        vec![
            plan_step(
                library_root
                    .join(PACKS_DIR)
                    .join(pack_id)
                    .join(PACK_MANIFEST_FILE),
                "updatePack",
                "Append the new Markdown path to the ordered instruction manifest.",
            ),
            plan_step(
                file_path,
                "createRuleFile",
                "Create a new user-owned Markdown instruction source.",
            ),
        ]
    };
    Ok(LibraryPlan {
        operation: "addPackFile".into(),
        blocked,
        change_count: steps.len(),
        summary: if blocked {
            format!(
                "`{relative_path}` is already declared or exists in Rule Pack `{pack_id}`; it will not be overwritten."
            )
        } else {
            format!(
                "Add `{relative_path}` after the existing sources in Rule Pack `{pack_id}`."
            )
        },
        steps,
    })
}

pub(crate) fn add_pack_file(
    library_root: &Path,
    state_root: &Path,
    pack_id: &str,
    relative_path: &str,
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_add_pack_file(library_root, pack_id, relative_path)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    let manifest_path = library_root
        .join(PACKS_DIR)
        .join(pack_id)
        .join(PACK_MANIFEST_FILE);
    let manifest_original = read_file_state(&manifest_path)?;
    let mut document: PackDocument = match &manifest_original {
        FileState::File { content } => serde_json::from_str(content)?,
        _ => return Err(ArmError::ApplyDrift(manifest_path.display().to_string())),
    };
    validate_pack_document(&document, pack_id)?;
    if document
        .instructions
        .iter()
        .any(|path| path == relative_path)
    {
        return Err(ArmError::ApplyDrift(manifest_path.display().to_string()));
    }
    document.instructions.push(relative_path.into());
    let file_path = library_root.join(PACKS_DIR).join(pack_id).join(relative_path);
    let title = Path::new(relative_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Supplemental rules")
        .replace('-', " ")
        .replace('_', " ");
    apply_changes(
        state_root,
        "addPackFile",
        vec![
            PreparedChange {
                path: manifest_path,
                original: manifest_original,
                desired: FileState::File {
                    content: pretty_json(&document)? + "\n",
                },
            },
            missing_file_change(file_path, format!("# {title}\n"))?,
        ],
    )
}

pub(crate) fn plan_create_profile(
    library_root: &Path,
    id: &str,
    name: &str,
    description: &str,
    pack_ids: &[String],
) -> Result<LibraryPlan, ArmError> {
    validate_id(id)?;
    validate_label("profile name", name, 80)?;
    validate_label("profile description", description, 240)?;
    let loaded = load_library(library_root)?;
    validate_pack_selection(&loaded, pack_ids)?;
    let path = library_root.join(PROFILES_DIR).join(format!("{id}.json"));
    let blocked = path_entry_exists(&path)?;
    let steps = if blocked {
        Vec::new()
    } else {
        vec![plan_step(
            path,
            "createProfile",
            "Create an ordered, syncable list of Rule Pack ids.",
        )]
    };
    Ok(LibraryPlan {
        operation: "createProfile".into(),
        blocked,
        change_count: steps.len(),
        summary: if blocked {
            format!("Profile `{id}` already exists; no file will be overwritten.")
        } else {
            format!("Create Profile `{name}` from {} ordered Rule Pack(s).", pack_ids.len())
        },
        steps,
    })
}

pub(crate) fn create_profile(
    library_root: &Path,
    state_root: &Path,
    id: &str,
    name: &str,
    description: &str,
    pack_ids: &[String],
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_create_profile(library_root, id, name, description, pack_ids)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    let document = ProfileDocument {
        schema_version: SCHEMA_VERSION,
        id: id.into(),
        name: name.trim().into(),
        description: description.trim().into(),
        packs: pack_ids.to_vec(),
    };
    apply_changes(
        state_root,
        "createProfile",
        vec![missing_file_change(
            library_root.join(PROFILES_DIR).join(format!("{id}.json")),
            pretty_json(&document)? + "\n",
        )?],
    )
}

pub(crate) fn plan_activate_profile(
    library_root: &Path,
    state_root: &Path,
    profile_id: &str,
) -> Result<LibraryPlan, ArmError> {
    validate_id(profile_id)?;
    let loaded = load_library(library_root)?;
    let profile = loaded
        .profiles
        .get(profile_id)
        .ok_or_else(|| ArmError::UnknownProfile(profile_id.into()))?;
    let rendered = render_profile(&loaded, profile)?;
    let runtime_path = library_root.join(RUNTIME_DIR).join(&rendered.runtime_name);
    let current_path = library_root.join(CURRENT_LINK);
    let machine_path = state_root.join(MACHINE_FILE);
    let current = read_file_state(&current_path)?;
    let machine = read_file_state(&machine_path)?;
    let desired_current = FileState::Symlink {
        target: format!("{RUNTIME_DIR}/{}", rendered.runtime_name),
    };
    let desired_machine = FileState::File {
        content: pretty_json(&MachineDocument {
            schema_version: SCHEMA_VERSION,
            active_profile_id: profile_id.into(),
        })? + "\n",
    };
    let current_allowed = match &current {
        FileState::Missing => true,
        FileState::Symlink { target } => managed_runtime_link(library_root, &current_path, target),
        FileState::File { .. } => false,
    };
    let machine_allowed = matches!(machine, FileState::Missing | FileState::File { .. });
    let blocked = !current_allowed || !machine_allowed;
    let mut steps = Vec::new();
    if blocked {
        return Ok(LibraryPlan {
            operation: "activateProfile".into(),
            blocked: true,
            change_count: 0,
            summary: "Activation is blocked by an unmanaged `current` link or machine state entry."
                .into(),
            steps,
        });
    }
    if !runtime_matches(&runtime_path, &rendered)? {
        steps.push(plan_step(
            runtime_path,
            "renderRuntime",
            "Render immutable AGENTS.md output with source markers and a manifest.",
        ));
    }
    if current != desired_current {
        steps.push(plan_step(
            current_path,
            "switchCurrent",
            "Atomically switch the stable current directory link.",
        ));
    }
    if machine != desired_machine {
        steps.push(plan_step(
            machine_path,
            "selectLocalProfile",
            "Record this Profile only in machine-local application state.",
        ));
    }
    Ok(LibraryPlan {
        operation: "activateProfile".into(),
        blocked: false,
        change_count: steps.len(),
        summary: if steps.is_empty() {
            format!("Profile `{profile_id}` is already current on this machine.")
        } else {
            format!("Render and activate Profile `{profile_id}` without changing synced sources.")
        },
        steps,
    })
}

pub(crate) fn activate_profile(
    library_root: &Path,
    state_root: &Path,
    profile_id: &str,
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_activate_profile(library_root, state_root, profile_id)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    let loaded = load_library(library_root)?;
    let profile = loaded
        .profiles
        .get(profile_id)
        .ok_or_else(|| ArmError::UnknownProfile(profile_id.into()))?;
    let rendered = render_profile(&loaded, profile)?;
    let runtime_path = ensure_runtime(library_root, &rendered)?;
    let current_path = library_root.join(CURRENT_LINK);
    let machine_path = state_root.join(MACHINE_FILE);
    let desired_current = FileState::Symlink {
        target: format!("{RUNTIME_DIR}/{}", rendered.runtime_name),
    };
    let desired_machine = FileState::File {
        content: pretty_json(&MachineDocument {
            schema_version: SCHEMA_VERSION,
            active_profile_id: profile_id.into(),
        })? + "\n",
    };
    let mut changes = Vec::new();
    for (path, desired) in [
        (current_path, desired_current),
        (machine_path, desired_machine),
    ] {
        let original = read_file_state(&path)?;
        if original != desired {
            changes.push(PreparedChange {
                path,
                original,
                desired,
            });
        }
    }
    let mut outcome = apply_changes(state_root, "activateProfile", changes)?;
    if plan
        .steps
        .iter()
        .any(|step| step.action == "renderRuntime")
    {
        outcome.changed.insert(0, runtime_path.display().to_string());
    }
    Ok(outcome)
}

pub(crate) fn rule_source_path(
    library_root: &Path,
    pack_id: &str,
    relative_path: &str,
) -> Result<PathBuf, ArmError> {
    let loaded = load_library(library_root)?;
    let pack = loaded
        .packs
        .get(pack_id)
        .ok_or_else(|| ArmError::UnknownPack(pack_id.into()))?;
    pack.files
        .iter()
        .find(|file| file.relative_path == relative_path)
        .map(|file| file.path.clone())
        .ok_or_else(|| {
            ArmError::InvalidLibrary(format!(
                "`{relative_path}` is not declared by Rule Pack `{pack_id}`"
            ))
        })
}

pub(crate) fn latest_library_backup_id(state_root: &Path) -> Result<Option<String>, ArmError> {
    Ok(latest_backup_path(state_root)?.and_then(|path| {
        path.file_stem()
            .map(|value| value.to_string_lossy().to_string())
    }))
}

pub(crate) fn rollback_library_latest(state_root: &Path) -> Result<RollbackOutcome, ArmError> {
    let backup_path = latest_backup_path(state_root)?.ok_or(ArmError::NoBackup)?;
    let data = fs::read_to_string(&backup_path)
        .map_err(|error| ArmError::io(backup_path.display().to_string(), error))?;
    let backup: BackupSnapshot = serde_json::from_str(&data)?;
    for entry in &backup.entries {
        let path = PathBuf::from(&entry.target_path);
        let current = read_file_state(&path)?;
        let current_digest = file_state_digest(&current)?;
        let original_digest = file_state_digest(&entry.original)?;
        if current_digest != entry.applied_digest && current_digest != original_digest {
            return Err(ArmError::RollbackDrift(entry.target_path.clone()));
        }
    }
    let mut restored = Vec::new();
    for entry in backup.entries.iter().rev() {
        let path = PathBuf::from(&entry.target_path);
        restore_file_state(&path, &entry.original)?;
        restored.push(entry.target_path.clone());
    }
    archive_backup(state_root, &backup_path)?;
    Ok(RollbackOutcome {
        restored,
        backup_id: backup.id,
    })
}

fn detect_library_state(library_root: &Path) -> Result<(LibraryState, String), ArmError> {
    let metadata = match fs::symlink_metadata(library_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((LibraryState::Empty, "The rule library has not been created.".into()))
        }
        Err(error) => return Err(ArmError::io(library_root.display().to_string(), error)),
    };
    if !metadata.is_dir() {
        return Ok((
            LibraryState::Conflict,
            "The selected library path is not a directory.".into(),
        ));
    }
    let schema = library_root.join(SCHEMA_FILE);
    if regular_file(schema.clone())?.is_some() {
        return Ok((LibraryState::Ready, "The versioned rule library is ready.".into()));
    }
    if path_entry_exists(&schema)? {
        return Ok((
            LibraryState::Conflict,
            "schema.json exists but is not a regular file.".into(),
        ));
    }

    let mut unexpected = Vec::new();
    for entry in fs::read_dir(library_root)
        .map_err(|error| ArmError::io(library_root.display().to_string(), error))?
    {
        let entry = entry.map_err(|error| ArmError::io(library_root.display().to_string(), error))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !matches!(name.as_str(), LEGACY_SOURCE_FILE | ".git" | ".gitignore") {
            unexpected.push(name);
        }
    }
    if !unexpected.is_empty() {
        return Ok((
            LibraryState::Conflict,
            format!(
                "The directory has no schema.json and contains unmanaged entries: {}.",
                unexpected.join(", ")
            ),
        ));
    }
    if regular_file(library_root.join(LEGACY_SOURCE_FILE))?.is_some() {
        return Ok((
            LibraryState::Legacy,
            "A legacy root AGENTS.md can be migrated into a Rule Pack with rollback protection."
                .into(),
        ));
    }
    if path_entry_exists(&library_root.join(LEGACY_SOURCE_FILE))? {
        return Ok((
            LibraryState::Conflict,
            "The legacy AGENTS.md entry is not a regular file.".into(),
        ));
    }
    Ok((LibraryState::Empty, "The rule library is empty.".into()))
}

fn ensure_ready(library_root: &Path) -> Result<(), ArmError> {
    let (state, detail) = detect_library_state(library_root)?;
    if state == LibraryState::Ready {
        Ok(())
    } else {
        Err(ArmError::InvalidLibrary(detail))
    }
}

fn load_library(library_root: &Path) -> Result<LoadedLibrary, ArmError> {
    ensure_ready(library_root)?;
    let schema_path = library_root.join(SCHEMA_FILE);
    let schema: SchemaDocument = read_json(&schema_path)?;
    if schema.schema_version != SCHEMA_VERSION || schema.format != "agent-rules-library" {
        return Err(ArmError::InvalidLibrary(format!(
            "unsupported schema in {}",
            schema_path.display()
        )));
    }
    if schema
        .legacy_source
        .as_deref()
        .is_some_and(|source| source != LEGACY_SOURCE_FILE)
    {
        return Err(ArmError::InvalidLibrary(format!(
            "unsupported legacy source in {}",
            schema_path.display()
        )));
    }

    let packs_root = library_root.join(PACKS_DIR);
    let mut packs = BTreeMap::new();
    for path in sorted_entries(&packs_root)? {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| ArmError::io(path.display().to_string(), error))?;
        if !metadata.is_dir() {
            return Err(ArmError::InvalidLibrary(format!(
                "Rule Pack entry is not a directory: {}",
                path.display()
            )));
        }
        let directory_id = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| ArmError::InvalidLibrary(path.display().to_string()))?
            .to_string();
        validate_id(&directory_id)?;
        let manifest_path = path.join(PACK_MANIFEST_FILE);
        let manifest_metadata = fs::symlink_metadata(&manifest_path)
            .map_err(|error| ArmError::io(manifest_path.display().to_string(), error))?;
        if !manifest_metadata.is_file() {
            return Err(ArmError::InvalidLibrary(format!(
                "Rule Pack manifest must be a regular file: {}",
                manifest_path.display()
            )));
        }
        let document: PackDocument = read_json(&manifest_path)?;
        validate_pack_document(&document, &directory_id)?;
        let mut files = Vec::new();
        for relative in &document.instructions {
            validate_instruction_path(relative)?;
            let file_path = path.join(relative);
            let metadata = fs::symlink_metadata(&file_path)
                .map_err(|error| ArmError::io(file_path.display().to_string(), error))?;
            if !metadata.is_file() {
                return Err(ArmError::InvalidLibrary(format!(
                    "instruction source must be a regular file: {}",
                    file_path.display()
                )));
            }
            let content = fs::read_to_string(&file_path)
                .map_err(|error| ArmError::io(file_path.display().to_string(), error))?;
            files.push(LoadedRuleFile {
                relative_path: relative.clone(),
                path: file_path,
                digest: short_digest(&content),
                content,
                modified_at: metadata
                    .modified()
                    .ok()
                    .map(DateTime::<Utc>::from)
                    .map(|timestamp| timestamp.to_rfc3339()),
            });
        }
        if packs
            .insert(
                document.id.clone(),
                LoadedPack {
                    document,
                    files,
                },
            )
            .is_some()
        {
            return Err(ArmError::InvalidLibrary(format!(
                "duplicate Rule Pack id `{directory_id}`"
            )));
        }
    }

    let profiles_root = library_root.join(PROFILES_DIR);
    let mut profiles = BTreeMap::new();
    for path in sorted_entries(&profiles_root)? {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| ArmError::io(path.display().to_string(), error))?;
        if !metadata.is_file() || path.extension().and_then(|value| value.to_str()) != Some("json") {
            return Err(ArmError::InvalidLibrary(format!(
                "Profile entry must be a JSON file: {}",
                path.display()
            )));
        }
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| ArmError::InvalidLibrary(path.display().to_string()))?;
        let document: ProfileDocument = read_json(&path)?;
        validate_profile_document(&document, file_id, &packs)?;
        if profiles.insert(document.id.clone(), document).is_some() {
            return Err(ArmError::InvalidLibrary(format!(
                "duplicate Profile id `{file_id}`"
            )));
        }
    }
    Ok(LoadedLibrary {
        packs,
        profiles,
        legacy_source_hint: schema.legacy_source,
    })
}

fn validate_pack_document(document: &PackDocument, directory_id: &str) -> Result<(), ArmError> {
    if document.schema_version != SCHEMA_VERSION {
        return Err(ArmError::InvalidLibrary(format!(
            "Rule Pack `{directory_id}` uses an unsupported schema"
        )));
    }
    validate_id(&document.id)?;
    validate_label("pack name", &document.name, 80)?;
    validate_label("pack description", &document.description, 240)?;
    if document.id != directory_id {
        return Err(ArmError::InvalidLibrary(format!(
            "Rule Pack id `{}` does not match directory `{directory_id}`",
            document.id
        )));
    }
    if document.instructions.first().map(String::as_str) != Some(ENTRYPOINT_FILE) {
        return Err(ArmError::InvalidLibrary(format!(
            "Rule Pack `{directory_id}` must declare AGENTS.md as its first instruction file"
        )));
    }
    let mut seen = BTreeSet::new();
    for path in &document.instructions {
        validate_instruction_path(path)?;
        if !seen.insert(path) {
            return Err(ArmError::InvalidLibrary(format!(
                "Rule Pack `{directory_id}` declares `{path}` more than once"
            )));
        }
    }
    Ok(())
}

fn validate_profile_document(
    document: &ProfileDocument,
    file_id: &str,
    packs: &BTreeMap<String, LoadedPack>,
) -> Result<(), ArmError> {
    if document.schema_version != SCHEMA_VERSION {
        return Err(ArmError::InvalidLibrary(format!(
            "Profile `{file_id}` uses an unsupported schema"
        )));
    }
    validate_id(&document.id)?;
    validate_label("profile name", &document.name, 80)?;
    validate_label("profile description", &document.description, 240)?;
    if document.id != file_id {
        return Err(ArmError::InvalidLibrary(format!(
            "Profile id `{}` does not match file `{file_id}.json`",
            document.id
        )));
    }
    validate_pack_selection_from_map(packs, &document.packs)
}

fn validate_pack_selection(loaded: &LoadedLibrary, pack_ids: &[String]) -> Result<(), ArmError> {
    validate_pack_selection_from_map(&loaded.packs, pack_ids)
}

fn validate_pack_selection_from_map(
    packs: &BTreeMap<String, LoadedPack>,
    pack_ids: &[String],
) -> Result<(), ArmError> {
    if pack_ids.is_empty() {
        return Err(ArmError::InvalidLibrary(
            "a Profile must contain at least one Rule Pack".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for id in pack_ids {
        validate_id(id)?;
        if !packs.contains_key(id) {
            return Err(ArmError::UnknownPack(id.clone()));
        }
        if !seen.insert(id) {
            return Err(ArmError::InvalidLibrary(format!(
                "Profile declares Rule Pack `{id}` more than once"
            )));
        }
    }
    Ok(())
}

fn validate_instruction_path(value: &str) -> Result<(), ArmError> {
    let path = Path::new(value);
    let safe = !value.is_empty()
        && !path.is_absolute()
        && path.extension().and_then(|extension| extension.to_str()) == Some("md")
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if safe {
        Ok(())
    } else {
        Err(ArmError::InvalidLibrary(format!(
            "invalid instruction path `{value}`"
        )))
    }
}

fn validate_id(id: &str) -> Result<(), ArmError> {
    let valid = !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_')
        && id.as_bytes()[0].is_ascii_alphanumeric();
    if valid {
        Ok(())
    } else {
        Err(ArmError::InvalidId(id.into()))
    }
}

fn validate_label(kind: &str, value: &str, max: usize) -> Result<(), ArmError> {
    if value.trim().is_empty() && !kind.ends_with("description") {
        return Err(ArmError::InvalidLibrary(format!("{kind} cannot be empty")));
    }
    if value.chars().count() > max {
        return Err(ArmError::InvalidLibrary(format!(
            "{kind} is longer than {max} characters"
        )));
    }
    Ok(())
}

fn render_default_profile(
    library_root: &Path,
    rules: &str,
) -> Result<RenderedProfile, ArmError> {
    let pack_document = PackDocument {
        schema_version: SCHEMA_VERSION,
        id: "base".into(),
        name: "Base rules".into(),
        description: "Rules shared by every local profile.".into(),
        instructions: vec![ENTRYPOINT_FILE.into()],
    };
    let pack = LoadedPack {
        document: pack_document,
        files: vec![LoadedRuleFile {
            relative_path: ENTRYPOINT_FILE.into(),
            path: library_root
                .join(PACKS_DIR)
                .join("base")
                .join(ENTRYPOINT_FILE),
            content: rules.into(),
            digest: short_digest(rules),
            modified_at: None,
        }],
    };
    let profile = ProfileDocument {
        schema_version: SCHEMA_VERSION,
        id: "default".into(),
        name: "Default".into(),
        description: "The initial machine profile.".into(),
        packs: vec!["base".into()],
    };
    render_profile(
        &LoadedLibrary {
            packs: BTreeMap::from([("base".into(), pack)]),
            profiles: BTreeMap::new(),
            legacy_source_hint: None,
        },
        &profile,
    )
}

fn render_profile(
    loaded: &LoadedLibrary,
    profile: &ProfileDocument,
) -> Result<RenderedProfile, ArmError> {
    validate_pack_selection(loaded, &profile.packs)?;
    let mut source_material = format!("profile:{}\n", profile.id);
    let mut sources = Vec::new();
    let mut content = format!(
        "<!-- Generated by Agent Rules Manager. Edit Rule Pack sources, not this file. -->\n<!-- Profile: {} -->\n",
        profile.id
    );
    for pack_id in &profile.packs {
        let pack = loaded
            .packs
            .get(pack_id)
            .ok_or_else(|| ArmError::UnknownPack(pack_id.clone()))?;
        for file in &pack.files {
            let logical = format!("packs/{pack_id}/{}", file.relative_path);
            source_material.push_str(&logical);
            source_material.push('\0');
            source_material.push_str(&file.content);
            source_material.push('\0');
            content.push_str(&format!("\n<!-- Source: {logical} -->\n"));
            content.push_str(file.content.trim_end_matches('\n'));
            content.push('\n');
            sources.push(RuntimeSource {
                pack_id: pack_id.clone(),
                path: file.relative_path.clone(),
                digest: file.digest.clone(),
            });
        }
    }
    let profile_digest = short_digest(&source_material);
    let rendered_digest = short_digest(&content);
    let runtime_name = format!("{}-{profile_digest}", profile.id);
    let manifest = RuntimeManifest {
        schema_version: SCHEMA_VERSION,
        profile_id: profile.id.clone(),
        profile_digest: profile_digest.clone(),
        rendered_digest: rendered_digest.clone(),
        packs: profile.packs.clone(),
        sources,
    };
    Ok(RenderedProfile {
        runtime_name,
        content,
        manifest,
    })
}

fn inspect_current(library_root: &Path, rendered: &RenderedProfile) -> Result<RuntimeState, ArmError> {
    let current = library_root.join(CURRENT_LINK);
    let state = read_file_state(&current)?;
    match state {
        FileState::Missing => Ok(RuntimeState::Missing),
        FileState::File { .. } => Ok(RuntimeState::Conflict),
        FileState::Symlink { target } => {
            if !managed_runtime_link(library_root, &current, &target) {
                return Ok(RuntimeState::Conflict);
            }
            let linked = resolve_link(&current, &target);
            let expected = library_root.join(RUNTIME_DIR).join(&rendered.runtime_name);
            if normalize_path(&linked) != normalize_path(&expected) {
                return Ok(RuntimeState::Stale);
            }
            if runtime_matches(&expected, rendered)? {
                Ok(RuntimeState::Current)
            } else {
                Ok(RuntimeState::Conflict)
            }
        }
    }
}

fn ensure_runtime(library_root: &Path, rendered: &RenderedProfile) -> Result<PathBuf, ArmError> {
    let runtime_root = library_root.join(RUNTIME_DIR);
    fs::create_dir_all(&runtime_root)
        .map_err(|error| ArmError::io(runtime_root.display().to_string(), error))?;
    let target = runtime_root.join(&rendered.runtime_name);
    if path_entry_exists(&target)? {
        if runtime_matches(&target, rendered)? {
            return Ok(target);
        }
        return Err(ArmError::Blocked(format!(
            "immutable runtime already exists with unexpected content: {}",
            target.display()
        )));
    }
    let temporary = runtime_root.join(format!(
        ".render-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::create_dir(&temporary)
        .map_err(|error| ArmError::io(temporary.display().to_string(), error))?;
    let result = (|| {
        write_atomic(&temporary.join(ENTRYPOINT_FILE), &rendered.content)?;
        write_atomic(
            &temporary.join("manifest.json"),
            &(pretty_json(&rendered.manifest)? + "\n"),
        )?;
        fs::rename(&temporary, &target)
            .map_err(|error| ArmError::io(target.display().to_string(), error))?;
        Ok::<(), ArmError>(())
    })();
    if result.is_err() && temporary.exists() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result?;
    Ok(target)
}

fn runtime_matches(path: &Path, rendered: &RenderedProfile) -> Result<bool, ArmError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(ArmError::io(path.display().to_string(), error)),
    };
    if !metadata.is_dir() {
        return Ok(false);
    }
    let content_path = path.join(ENTRYPOINT_FILE);
    let manifest_path = path.join("manifest.json");
    let content = match fs::read_to_string(&content_path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(ArmError::io(content_path.display().to_string(), error)),
    };
    let manifest: RuntimeManifest = match read_json(&manifest_path) {
        Ok(value) => value,
        Err(ArmError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(false)
        }
        Err(error) => return Err(error),
    };
    Ok(content == rendered.content && manifest == rendered.manifest)
}

fn read_machine(state_root: &Path) -> Result<Option<MachineDocument>, ArmError> {
    let path = state_root.join(MACHINE_FILE);
    if !path_entry_exists(&path)? {
        return Ok(None);
    }
    let document: MachineDocument = read_json(&path)?;
    if document.schema_version != SCHEMA_VERSION {
        return Err(ArmError::InvalidLibrary(
            "machine state uses an unsupported schema".into(),
        ));
    }
    validate_id(&document.active_profile_id)?;
    Ok(Some(document))
}

fn inspect_rendered_source(
    source: &Path,
) -> Result<(bool, Option<String>, Option<String>), ArmError> {
    let metadata = match fs::metadata(source) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((false, None, None))
        }
        Err(error) => return Err(ArmError::io(source.display().to_string(), error)),
    };
    if !metadata.is_file() {
        return Ok((false, None, None));
    }
    let content = fs::read_to_string(source)
        .map_err(|error| ArmError::io(source.display().to_string(), error))?;
    Ok((
        true,
        Some(short_digest(&content)),
        metadata
            .modified()
            .ok()
            .map(DateTime::<Utc>::from)
            .map(|timestamp| timestamp.to_rfc3339()),
    ))
}

fn apply_changes(
    state_root: &Path,
    operation: &str,
    changes: Vec<PreparedChange>,
) -> Result<LibraryMutationOutcome, ArmError> {
    if changes.is_empty() {
        return Ok(LibraryMutationOutcome {
            changed: Vec::new(),
            backup_id: None,
        });
    }
    let backup_id = Utc::now().format("%Y%m%dT%H%M%S%.9fZ").to_string();
    let backup = BackupSnapshot {
        id: backup_id.clone(),
        created_at: Utc::now().to_rfc3339(),
        operation: operation.into(),
        entries: changes
            .iter()
            .map(|change| {
                Ok(BackupEntry {
                    target_path: change.path.display().to_string(),
                    original: change.original.clone(),
                    applied_digest: file_state_digest(&change.desired)?,
                })
            })
            .collect::<Result<Vec<_>, ArmError>>()?,
    };
    let backup_path = write_backup(state_root, &backup)?;
    let mut completed = 0;
    for change in &changes {
        let current = match read_file_state(&change.path) {
            Ok(current) => current,
            Err(error) => {
                restore_completed(&changes[..completed]);
                return Err(error);
            }
        };
        if current != change.original {
            restore_completed(&changes[..completed]);
            return Err(ArmError::ApplyDrift(change.path.display().to_string()));
        }
        if let Err(error) = write_file_state(&change.path, &change.desired) {
            if restore_completed(&changes[..completed]) {
                let _ = archive_backup(state_root, &backup_path);
            }
            return Err(error);
        }
        let written = match read_file_state(&change.path) {
            Ok(written) => written,
            Err(error) => {
                restore_completed(&changes[..=completed]);
                return Err(error);
            }
        };
        if written != change.desired {
            restore_completed(&changes[..=completed]);
            return Err(ArmError::ApplyDrift(change.path.display().to_string()));
        }
        completed += 1;
    }
    Ok(LibraryMutationOutcome {
        changed: changes
            .iter()
            .map(|change| change.path.display().to_string())
            .collect(),
        backup_id: Some(backup_id),
    })
}

fn missing_file_change(path: PathBuf, content: String) -> Result<PreparedChange, ArmError> {
    let original = read_file_state(&path)?;
    if original != FileState::Missing {
        return Err(ArmError::Blocked(path.display().to_string()));
    }
    Ok(PreparedChange {
        path,
        original,
        desired: FileState::File { content },
    })
}

fn write_backup(state_root: &Path, backup: &BackupSnapshot) -> Result<PathBuf, ArmError> {
    let directory = state_root.join("library-backups");
    fs::create_dir_all(&directory)
        .map_err(|error| ArmError::io(directory.display().to_string(), error))?;
    let path = directory.join(format!("{}.json", backup.id));
    write_atomic(&path, &(pretty_json(backup)? + "\n"))?;
    Ok(path)
}

fn latest_backup_path(state_root: &Path) -> Result<Option<PathBuf>, ArmError> {
    let directory = state_root.join("library-backups");
    if !directory.exists() {
        return Ok(None);
    }
    let mut paths = sorted_entries(&directory)?
        .into_iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    Ok(paths.pop())
}

fn archive_backup(state_root: &Path, path: &Path) -> Result<(), ArmError> {
    let directory = state_root.join("library-backups/restored");
    fs::create_dir_all(&directory)
        .map_err(|error| ArmError::io(directory.display().to_string(), error))?;
    let name = path
        .file_name()
        .ok_or_else(|| ArmError::UnsupportedEntry(path.display().to_string()))?;
    fs::rename(path, directory.join(name))
        .map_err(|error| ArmError::io(path.display().to_string(), error))
}

fn restore_completed(changes: &[PreparedChange]) -> bool {
    let mut restored = true;
    for change in changes.iter().rev() {
        match read_file_state(&change.path) {
            Ok(current) if current == change.desired => {
                if restore_file_state(&change.path, &change.original).is_err() {
                    restored = false;
                }
            }
            Ok(current) if current == change.original => {}
            _ => restored = false,
        }
    }
    restored
}

fn read_file_state(path: &Path) -> Result<FileState, ArmError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FileState::Missing)
        }
        Err(error) => return Err(ArmError::io(path.display().to_string(), error)),
    };
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(path)
            .map_err(|error| ArmError::io(path.display().to_string(), error))?;
        return Ok(FileState::Symlink {
            target: target.display().to_string(),
        });
    }
    if metadata.is_file() {
        let content = fs::read_to_string(path)
            .map_err(|error| ArmError::io(path.display().to_string(), error))?;
        return Ok(FileState::File { content });
    }
    Err(ArmError::UnsupportedEntry(path.display().to_string()))
}

fn write_file_state(path: &Path, state: &FileState) -> Result<(), ArmError> {
    match state {
        FileState::Missing => restore_file_state(path, state),
        FileState::File { content } => write_atomic(path, content),
        FileState::Symlink { target } => write_atomic_symlink(path, Path::new(target)),
    }
}

fn restore_file_state(path: &Path, state: &FileState) -> Result<(), ArmError> {
    if path_entry_exists(path)? {
        fs::remove_file(path).map_err(|error| ArmError::io(path.display().to_string(), error))?;
    }
    match state {
        FileState::Missing => Ok(()),
        FileState::File { content } => write_atomic(path, content),
        FileState::Symlink { target } => write_atomic_symlink(path, Path::new(target)),
    }
}

fn write_atomic(path: &Path, content: &str) -> Result<(), ArmError> {
    let parent = path
        .parent()
        .ok_or_else(|| ArmError::UnsupportedEntry(path.display().to_string()))?;
    fs::create_dir_all(parent)
        .map_err(|error| ArmError::io(parent.display().to_string(), error))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| ArmError::UnsupportedEntry(path.display().to_string()))?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{file_name}.arm-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    fs::write(&temporary, content)
        .map_err(|error| ArmError::io(temporary.display().to_string(), error))?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(ArmError::io(path.display().to_string(), error));
    }
    Ok(())
}

#[cfg(unix)]
fn write_atomic_symlink(path: &Path, target: &Path) -> Result<(), ArmError> {
    let parent = path
        .parent()
        .ok_or_else(|| ArmError::UnsupportedEntry(path.display().to_string()))?;
    fs::create_dir_all(parent)
        .map_err(|error| ArmError::io(parent.display().to_string(), error))?;
    let temporary = parent.join(format!(
        ".current.arm-{}-{}",
        std::process::id(),
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    std::os::unix::fs::symlink(target, &temporary)
        .map_err(|error| ArmError::io(temporary.display().to_string(), error))?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(ArmError::io(path.display().to_string(), error));
    }
    Ok(())
}

#[cfg(windows)]
fn write_atomic_symlink(path: &Path, target: &Path) -> Result<(), ArmError> {
    let parent = path
        .parent()
        .ok_or_else(|| ArmError::UnsupportedEntry(path.display().to_string()))?;
    fs::create_dir_all(parent)
        .map_err(|error| ArmError::io(parent.display().to_string(), error))?;
    let temporary = parent.join(format!(".current.arm-{}", std::process::id()));
    std::os::windows::fs::symlink_dir(target, &temporary)
        .map_err(|error| ArmError::io(temporary.display().to_string(), error))?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_dir(&temporary);
        return Err(ArmError::io(path.display().to_string(), error));
    }
    Ok(())
}

fn file_state_digest(state: &FileState) -> Result<String, ArmError> {
    Ok(full_digest(&serde_json::to_vec(state)?))
}

fn short_digest(content: &str) -> String {
    full_digest(content.as_bytes())[..12].to_string()
}

fn full_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn pretty_json<T: Serialize>(value: &T) -> Result<String, ArmError> {
    Ok(serde_json::to_string_pretty(value)?)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ArmError> {
    let data = fs::read_to_string(path)
        .map_err(|error| ArmError::io(path.display().to_string(), error))?;
    Ok(serde_json::from_str(&data)?)
}

fn sorted_entries(path: &Path) -> Result<Vec<PathBuf>, ArmError> {
    let mut entries = fs::read_dir(path)
        .map_err(|error| ArmError::io(path.display().to_string(), error))?
        .map(|entry| {
            entry
                .map(|value| value.path())
                .map_err(|error| ArmError::io(path.display().to_string(), error))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    Ok(entries)
}

fn plan_step(path: PathBuf, action: &str, summary: &str) -> LibraryPlanStep {
    LibraryPlanStep {
        path: path.display().to_string(),
        action: action.into(),
        summary: summary.into(),
    }
}

fn path_entry_exists(path: &Path) -> Result<bool, ArmError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(ArmError::io(path.display().to_string(), error)),
    }
}

fn regular_file(path: PathBuf) -> Result<Option<PathBuf>, ArmError> {
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => Ok(Some(path)),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ArmError::io(path.display().to_string(), error)),
    }
}

fn managed_runtime_link(library_root: &Path, current_path: &Path, target: &str) -> bool {
    let linked = normalize_path(&resolve_link(current_path, target));
    let runtime_root = normalize_path(&library_root.join(RUNTIME_DIR));
    linked.starts_with(&runtime_root) && linked != runtime_root
}

fn resolve_link(path: &Path, target: &str) -> PathBuf {
    let target = PathBuf::from(target);
    if target.is_absolute() {
        target
    } else {
        path.parent().unwrap_or_else(|| Path::new(".")).join(target)
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixture() -> (TempDir, PathBuf, PathBuf) {
        let temporary = TempDir::new().expect("temp root");
        let library = temporary.path().join("library");
        let state = temporary.path().join("state");
        (temporary, library, state)
    }

    #[test]
    fn initialize_creates_pack_profile_runtime_and_local_selection() {
        let (_temporary, library, state) = fixture();
        let plan = plan_initialize(&library, &state).expect("plan");
        assert!(!plan.blocked);
        assert_eq!(plan.change_count, 8);

        initialize(&library, &state).expect("initialize");
        let snapshot = inspect(&library, &state).expect("inspect");
        assert_eq!(snapshot.state, LibraryState::Ready);
        assert_eq!(snapshot.runtime_state, RuntimeState::Current);
        assert_eq!(snapshot.active_profile_id.as_deref(), Some("default"));
        assert_eq!(snapshot.packs[0].files[0].path, "AGENTS.md");
        assert!(library.join("current/AGENTS.md").is_file());
        assert!(library.join("current").is_symlink());
    }

    #[test]
    fn legacy_source_is_migrated_and_rollback_can_restore_it() {
        let (_temporary, library, state) = fixture();
        fs::create_dir_all(&library).expect("library");
        fs::write(library.join(LEGACY_SOURCE_FILE), "# My existing rules\n").expect("legacy");

        let plan = plan_initialize(&library, &state).expect("plan");
        assert_eq!(plan.change_count, 9);
        assert!(plan.steps.iter().any(|step| step.action == "copyLegacy"));
        assert!(plan.steps.iter().any(|step| step.action == "removeLegacy"));
        initialize(&library, &state).expect("initialize");

        assert!(!library.join(LEGACY_SOURCE_FILE).exists());
        assert_eq!(
            fs::read_to_string(library.join("packs/base/AGENTS.md")).unwrap(),
            "# My existing rules\n"
        );
        let snapshot = inspect(&library, &state).expect("inspect");
        assert_eq!(snapshot.legacy_source_path, None);
        assert_eq!(
            snapshot.legacy_adapter_source_path.as_deref(),
            Some(library.join(LEGACY_SOURCE_FILE).as_path())
        );

        rollback_library_latest(&state).expect("rollback activation");
        rollback_library_latest(&state).expect("rollback migration");
        assert_eq!(
            fs::read_to_string(library.join(LEGACY_SOURCE_FILE)).unwrap(),
            "# My existing rules\n"
        );
        assert!(!library.join("packs/base/AGENTS.md").exists());
    }

    #[test]
    fn multiple_markdown_sources_render_in_manifest_order() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        let add_plan = plan_add_pack_file(&library, "base", "rules/review.md").expect("plan");
        assert_eq!(add_plan.change_count, 2);
        add_pack_file(&library, &state, "base", "rules/review.md").expect("add file");
        fs::write(library.join("packs/base/rules/review.md"), "# Review\n").expect("rule edit");

        assert_eq!(
            inspect(&library, &state).unwrap().runtime_state,
            RuntimeState::Stale
        );

        let plan = plan_activate_profile(&library, &state, "default").expect("plan");
        assert!(plan.steps.iter().any(|step| step.action == "renderRuntime"));
        activate_profile(&library, &state, "default").expect("refresh");
        let rendered = fs::read_to_string(library.join("current/AGENTS.md")).unwrap();
        assert!(rendered.contains("<!-- Source: packs/base/AGENTS.md -->"));
        assert!(rendered.contains("<!-- Source: packs/base/rules/review.md -->"));
        assert!(rendered.find("AGENTS.md").unwrap() < rendered.find("review.md").unwrap());
        assert!(!rendered.contains(&library.display().to_string()));
    }

    #[test]
    fn profile_switch_is_local_and_rollback_refuses_drift() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        create_pack(&library, &state, "work", "Work", "").expect("pack");
        create_profile(
            &library,
            &state,
            "work",
            "Work",
            "",
            &["base".into(), "work".into()],
        )
        .expect("profile");
        activate_profile(&library, &state, "work").expect("activate");
        assert_eq!(
            inspect(&library, &state)
                .unwrap()
                .active_profile_id
                .as_deref(),
            Some("work")
        );
        fs::write(state.join(MACHINE_FILE), "external edit").expect("drift");
        assert!(matches!(
            rollback_library_latest(&state),
            Err(ArmError::RollbackDrift(_))
        ));
    }

    #[test]
    fn supplemental_file_transaction_rolls_back_manifest_and_new_source_together() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        let manifest = library.join("packs/base/pack.json");
        let original = fs::read_to_string(&manifest).expect("original manifest");

        add_pack_file(&library, &state, "base", "rules/testing.md").expect("add file");
        assert!(library.join("packs/base/rules/testing.md").is_file());
        assert!(fs::read_to_string(&manifest).unwrap().contains("rules/testing.md"));

        rollback_library_latest(&state).expect("rollback");
        assert_eq!(fs::read_to_string(&manifest).unwrap(), original);
        assert!(!library.join("packs/base/rules/testing.md").exists());
    }

    #[test]
    fn synced_sources_arrive_without_a_machine_profile_selection() {
        let (temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        let synced = temporary.path().join("synced-library");
        let synced_state = temporary.path().join("synced-state");
        fs::create_dir_all(synced.join("packs/base")).expect("pack dir");
        fs::create_dir_all(synced.join("profiles")).expect("profile dir");
        for (source, target) in [
            (library.join("schema.json"), synced.join("schema.json")),
            (
                library.join("packs/base/pack.json"),
                synced.join("packs/base/pack.json"),
            ),
            (
                library.join("packs/base/AGENTS.md"),
                synced.join("packs/base/AGENTS.md"),
            ),
            (
                library.join("profiles/default.json"),
                synced.join("profiles/default.json"),
            ),
        ] {
            fs::copy(&source, &target).expect("sync source");
        }

        let snapshot = inspect(&synced, &synced_state).expect("inspect synced library");
        assert_eq!(snapshot.state, LibraryState::Ready);
        assert_eq!(snapshot.runtime_state, RuntimeState::Missing);
        assert_eq!(snapshot.active_profile_id, None);
        assert!(!synced.join("current").exists());
        assert!(!synced.join(".runtime").exists());
    }

    #[test]
    fn invalid_pack_path_cannot_escape_its_directory() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        let path = library.join("packs/base/pack.json");
        let mut pack: PackDocument = read_json(&path).expect("pack");
        pack.instructions.push("../secret.md".into());
        write_atomic(&path, &(pretty_json(&pack).unwrap() + "\n")).expect("write");
        assert!(matches!(inspect(&library, &state), Err(ArmError::InvalidLibrary(_))));
    }
}

use crate::{
    ArmError, LibraryMutationOutcome, LibraryPlan, LibraryPlanStep, LibraryState, ProfileSummary,
    RollbackOutcome, RuleFileSummary, RuntimeState,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

const LIBRARY_SCHEMA_VERSION: u32 = 2;
const LEGACY_LIBRARY_SCHEMA_VERSION: u32 = 1;
const MACHINE_SCHEMA_VERSION: u32 = 1;
const RUNTIME_SCHEMA_VERSION: u32 = 2;
const SCHEMA_FILE: &str = "schema.json";
const PACKS_DIR: &str = "packs";
const PROFILES_DIR: &str = "profiles";
const RUNTIME_DIR: &str = ".runtime";
const CURRENT_LINK: &str = "current";
const MACHINE_FILE: &str = "machine.json";
const LEGACY_SOURCE_FILE: &str = "AGENTS.md";
const PACK_MANIFEST_FILE: &str = "pack.json";
const PROFILE_MANIFEST_FILE: &str = "profile.json";
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
struct LegacyPackDocument {
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
    instructions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyProfileDocument {
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
    sources: Vec<RuntimeSource>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeProfileReference {
    profile_id: String,
}

#[derive(Debug, Clone)]
struct LoadedProfile {
    document: ProfileDocument,
    files: Vec<LoadedRuleFile>,
}

#[derive(Debug, Clone)]
struct LoadedLegacyPack {
    document: LegacyPackDocument,
    files: Vec<LoadedRuleFile>,
}

#[derive(Debug, Clone)]
struct LoadedLegacyLibrary {
    schema: SchemaDocument,
    packs: BTreeMap<String, LoadedLegacyPack>,
    profiles: BTreeMap<String, LegacyProfileDocument>,
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
    profiles: BTreeMap<String, LoadedProfile>,
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
    #[serde(default)]
    created_directories: Vec<String>,
}

#[derive(Debug, Clone)]
struct PreparedChange {
    path: PathBuf,
    original: FileState,
    desired: FileState,
}

#[derive(Debug, Clone)]
struct PreparedMigration {
    changes: Vec<PreparedChange>,
    steps: Vec<LibraryPlanStep>,
    loaded: LoadedLibrary,
}

#[derive(Debug, Clone)]
struct PreparedProfileDeletion {
    changes: Vec<PreparedChange>,
    directories: Vec<PathBuf>,
    steps: Vec<LibraryPlanStep>,
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
    let upgrade_legacy_adapter = if state == LibraryState::Upgrade {
        let schema: SchemaDocument = read_json(&library_root.join(SCHEMA_FILE))?;
        schema
            .legacy_source
            .as_ref()
            .map(|relative| library_root.join(relative))
    } else {
        None
    };
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
            legacy_adapter_source_path: legacy.or(upgrade_legacy_adapter),
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
    let active_profile_id = machine
        .as_ref()
        .map(|value| value.active_profile_id.clone());
    let mut runtime_state = RuntimeState::Missing;
    let mut active_source_path = None;
    let runtime_detail;

    if let Some(active_id) = &active_profile_id {
        let profile = loaded
            .profiles
            .get(active_id)
            .ok_or_else(|| ArmError::UnknownProfile(active_id.clone()))?;
        let rendered = render_profile(profile)?;
        active_source_path = profile.files.first().map(|file| file.path.clone());
        runtime_state = inspect_current(library_root, &rendered)?;
        runtime_detail = match runtime_state {
            RuntimeState::Current => {
                format!("Profile `{active_id}` is rendered and selected on this machine.")
            }
            RuntimeState::Missing => {
                format!("Profile `{active_id}` is selected but has not been rendered.")
            }
            RuntimeState::Stale => {
                format!("Profile `{active_id}` changed after the current runtime was rendered.")
            }
            RuntimeState::Conflict => "The `current` entry is not a managed runtime link.".into(),
        };
    } else if path_entry_exists(&library_root.join(CURRENT_LINK))? {
        runtime_state = RuntimeState::Conflict;
        runtime_detail =
            "A `current` entry exists without a machine-local profile selection.".into();
    } else {
        runtime_detail = "Choose a profile for this machine before connecting agents.".into();
    }

    let (source_exists, source_digest, source_modified_at) = inspect_rendered_source(&source)?;
    let profiles = loaded
        .profiles
        .values()
        .map(|profile| ProfileSummary {
            id: profile.document.id.clone(),
            name: profile.document.name.clone(),
            description: profile.document.description.clone(),
            files: profile
                .files
                .iter()
                .map(|file| RuleFileSummary {
                    path: file.relative_path.clone(),
                    digest: file.digest.clone(),
                    modified_at: file.modified_at.clone(),
                })
                .collect(),
            is_active: active_profile_id.as_deref() == Some(profile.document.id.as_str()),
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
        LibraryState::Upgrade => return plan_upgrade_library(library_root, state_root),
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
        library_root
            .join(PROFILES_DIR)
            .join("default")
            .join(PROFILE_MANIFEST_FILE),
        "createProfile",
        "Create the default Profile manifest and its ordered rule-file list.",
    ));
    steps.push(plan_step(
        library_root
            .join(PROFILES_DIR)
            .join("default")
            .join(ENTRYPOINT_FILE),
        if imported {
            "copyLegacy"
        } else {
            "createRules"
        },
        if imported {
            "Copy the exact legacy AGENTS.md content into the default Profile."
        } else {
            "Create the default Profile AGENTS.md."
        },
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
            "Migrate the legacy AGENTS.md into the default Profile with a rollback snapshot, remove the old root entry, then activate that Profile."
                .into()
        } else {
            "Create a Profile library and activate its default Profile.".into()
        },
        steps,
    })
}

pub(crate) fn initialize(
    library_root: &Path,
    state_root: &Path,
) -> Result<LibraryMutationOutcome, ArmError> {
    if detect_library_state(library_root)?.0 == LibraryState::Upgrade {
        return upgrade_library(library_root, state_root);
    }
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
        schema_version: LIBRARY_SCHEMA_VERSION,
        format: "agent-rules-library".into(),
        sync: SyncPolicy {
            include: vec!["schema.json".into(), "profiles/**".into()],
            exclude: vec![
                ".runtime/**".into(),
                "current".into(),
                "machine state".into(),
            ],
            machine_selection: "local".into(),
        },
        legacy_source: imported.then(|| LEGACY_SOURCE_FILE.into()),
    };
    let profile = ProfileDocument {
        schema_version: LIBRARY_SCHEMA_VERSION,
        id: "default".into(),
        name: "Default".into(),
        description: "The initial machine profile.".into(),
        instructions: vec![ENTRYPOINT_FILE.into()],
    };

    let mut changes = vec![
        missing_file_change(library_root.join(SCHEMA_FILE), pretty_json(&schema)? + "\n")?,
        missing_file_change(
            library_root
                .join(PROFILES_DIR)
                .join("default")
                .join(PROFILE_MANIFEST_FILE),
            pretty_json(&profile)? + "\n",
        )?,
        missing_file_change(
            library_root
                .join(PROFILES_DIR)
                .join("default")
                .join(ENTRYPOINT_FILE),
            rules,
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

fn migration_active_profile_id(
    library_root: &Path,
    state_root: &Path,
) -> Result<Option<String>, ArmError> {
    if let Some(machine) = read_machine(state_root)? {
        return Ok(Some(machine.active_profile_id));
    }
    let current_path = library_root.join(CURRENT_LINK);
    match read_file_state(&current_path)? {
        FileState::Missing => Ok(None),
        FileState::File { .. } => Err(ArmError::Blocked(
            "migration cannot infer the active Profile from an unmanaged `current` entry".into(),
        )),
        FileState::Symlink { target } => {
            if !managed_runtime_link(library_root, &current_path, &target) {
                return Err(ArmError::Blocked(
                    "migration cannot infer the active Profile from a foreign `current` link"
                        .into(),
                ));
            }
            let runtime_path = resolve_link(&current_path, &target);
            let reference: RuntimeProfileReference =
                read_json(&runtime_path.join("manifest.json"))?;
            validate_id(&reference.profile_id)?;
            Ok(Some(reference.profile_id))
        }
    }
}

fn plan_upgrade_library(library_root: &Path, state_root: &Path) -> Result<LibraryPlan, ArmError> {
    let migration = match prepare_v1_migration(library_root) {
        Ok(migration) => migration,
        Err(ArmError::InvalidLibrary(summary) | ArmError::Blocked(summary)) => {
            return Ok(LibraryPlan {
                operation: "upgradeLibrary".into(),
                blocked: true,
                change_count: 0,
                summary,
                steps: Vec::new(),
            })
        }
        Err(error) => return Err(error),
    };
    let active_profile_id = match migration_active_profile_id(library_root, state_root) {
        Ok(active_profile_id) => active_profile_id,
        Err(ArmError::Blocked(summary) | ArmError::InvalidLibrary(summary)) => {
            return Ok(LibraryPlan {
                operation: "upgradeLibrary".into(),
                blocked: true,
                change_count: 0,
                summary,
                steps: Vec::new(),
            })
        }
        Err(ArmError::UnknownProfile(profile_id)) => {
            return Ok(LibraryPlan {
                operation: "upgradeLibrary".into(),
                blocked: true,
                change_count: 0,
                summary: format!("migration references unknown Profile `{profile_id}`"),
                steps: Vec::new(),
            })
        }
        Err(error) => return Err(error),
    };
    let mut steps = migration.steps;
    if let Some(active_profile_id) = active_profile_id {
        let Some(profile) = migration.loaded.profiles.get(&active_profile_id) else {
            return Ok(LibraryPlan {
                operation: "upgradeLibrary".into(),
                blocked: true,
                change_count: 0,
                summary: format!("migration references unknown Profile `{active_profile_id}`"),
                steps: Vec::new(),
            });
        };
        let rendered = render_profile(profile)?;
        let runtime_path = library_root.join(RUNTIME_DIR).join(&rendered.runtime_name);
        let current_path = library_root.join(CURRENT_LINK);
        let current = read_file_state(&current_path)?;
        let current_allowed = match &current {
            FileState::Missing => true,
            FileState::Symlink { target } => {
                managed_runtime_link(library_root, &current_path, target)
            }
            FileState::File { .. } => false,
        };
        if !current_allowed {
            return Ok(LibraryPlan {
                operation: "upgradeLibrary".into(),
                blocked: true,
                change_count: 0,
                summary: "Migration is blocked by an unmanaged `current` entry.".into(),
                steps: Vec::new(),
            });
        }
        let runtime_current = runtime_matches(&runtime_path, &rendered)?;
        if path_entry_exists(&runtime_path)? && !runtime_current {
            return Ok(LibraryPlan {
                operation: "upgradeLibrary".into(),
                blocked: true,
                change_count: 0,
                summary: format!(
                    "Migration is blocked by an unexpected immutable runtime: {}",
                    runtime_path.display()
                ),
                steps: Vec::new(),
            });
        }
        if !runtime_current {
            steps.push(plan_step(
                runtime_path,
                "renderRuntime",
                "Render the active Profile from its new Profile-owned sources.",
            ));
        }
        let desired_current = FileState::Symlink {
            target: format!("{RUNTIME_DIR}/{}", rendered.runtime_name),
        };
        if current != desired_current {
            steps.push(plan_step(
                current_path,
                "switchCurrent",
                "Switch the stable current link to the migrated Profile runtime.",
            ));
        }
        let machine_path = state_root.join(MACHINE_FILE);
        let machine = read_file_state(&machine_path)?;
        let desired_machine = FileState::File {
            content: pretty_json(&MachineDocument {
                schema_version: MACHINE_SCHEMA_VERSION,
                active_profile_id,
            })? + "\n",
        };
        if machine != desired_machine {
            steps.push(plan_step(
                machine_path,
                "selectLocalProfile",
                "Preserve the inferred active Profile in machine-local state.",
            ));
        }
    }
    Ok(LibraryPlan {
        operation: "upgradeLibrary".into(),
        blocked: false,
        change_count: steps.len(),
        summary: "Migrate Rule Pack sources into Profile-owned files with a rollback snapshot."
            .into(),
        steps,
    })
}

fn upgrade_library(
    library_root: &Path,
    state_root: &Path,
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_upgrade_library(library_root, state_root)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    let active_profile_id = migration_active_profile_id(library_root, state_root)?;
    let migration = prepare_v1_migration(library_root)?;
    let migrated = apply_changes(state_root, "upgradeLibrary", migration.changes)?;
    let Some(active_profile_id) = active_profile_id else {
        return Ok(migrated);
    };
    match activate_profile(library_root, state_root, &active_profile_id) {
        Ok(mut activated) => {
            let mut changed = migrated.changed;
            changed.append(&mut activated.changed);
            Ok(LibraryMutationOutcome {
                changed,
                backup_id: activated.backup_id.or(migrated.backup_id),
            })
        }
        Err(error) => {
            let _ = rollback_library_latest(state_root);
            Err(error)
        }
    }
}

fn prepare_v1_migration(library_root: &Path) -> Result<PreparedMigration, ArmError> {
    let legacy = load_legacy_library(library_root)?;
    let referenced = legacy
        .profiles
        .values()
        .flat_map(|profile| profile.packs.iter().cloned())
        .collect::<BTreeSet<_>>();
    if let Some(unused) = legacy.packs.keys().find(|id| !referenced.contains(*id)) {
        return Err(ArmError::InvalidLibrary(format!(
            "Rule Pack `{unused}` is not used by any Profile; migration stopped to avoid orphaning its rules."
        )));
    }

    let schema_path = library_root.join(SCHEMA_FILE);
    let schema_original = read_file_state(&schema_path)?;
    let schema = SchemaDocument {
        schema_version: LIBRARY_SCHEMA_VERSION,
        format: "agent-rules-library".into(),
        sync: SyncPolicy {
            include: vec!["schema.json".into(), "profiles/**".into()],
            exclude: vec![
                ".runtime/**".into(),
                "current".into(),
                "machine state".into(),
            ],
            machine_selection: "local".into(),
        },
        legacy_source: legacy.schema.legacy_source.clone(),
    };
    let mut changes = vec![PreparedChange {
        path: schema_path.clone(),
        original: schema_original,
        desired: FileState::File {
            content: pretty_json(&schema)? + "\n",
        },
    }];
    let mut steps = vec![plan_step(
        schema_path,
        "upgradeSchema",
        "Replace the Rule Pack schema with the Profile-owned schema.",
    )];
    let mut profiles = BTreeMap::new();

    for legacy_profile in legacy.profiles.values() {
        let profile_directory = library_root.join(PROFILES_DIR).join(&legacy_profile.id);
        if path_entry_exists(&profile_directory)? {
            return Err(ArmError::Blocked(format!(
                "Profile directory already exists and will not be overwritten: {}",
                profile_directory.display()
            )));
        }
        let mut instructions = Vec::new();
        let mut files = Vec::new();
        for pack_id in &legacy_profile.packs {
            let pack = legacy.packs.get(pack_id).ok_or_else(|| {
                ArmError::InvalidLibrary(format!(
                    "legacy Profile references unknown Rule Pack `{pack_id}`"
                ))
            })?;
            for source in &pack.files {
                let relative_path = if instructions.is_empty() {
                    ENTRYPOINT_FILE.into()
                } else {
                    format!("rules/{pack_id}/{}", source.relative_path)
                };
                validate_instruction_path(&relative_path)?;
                let target = profile_directory.join(&relative_path);
                changes.push(missing_file_change(target.clone(), source.content.clone())?);
                steps.push(plan_step(
                    target.clone(),
                    "copyProfileRules",
                    "Copy an exact legacy rule source into its owning Profile.",
                ));
                instructions.push(relative_path.clone());
                files.push(LoadedRuleFile {
                    relative_path,
                    path: target,
                    content: source.content.clone(),
                    digest: source.digest.clone(),
                    modified_at: source.modified_at.clone(),
                });
            }
        }
        let document = ProfileDocument {
            schema_version: LIBRARY_SCHEMA_VERSION,
            id: legacy_profile.id.clone(),
            name: legacy_profile.name.clone(),
            description: legacy_profile.description.clone(),
            instructions,
        };
        validate_profile_document(&document, &legacy_profile.id)?;
        let manifest_path = profile_directory.join(PROFILE_MANIFEST_FILE);
        changes.push(missing_file_change(
            manifest_path.clone(),
            pretty_json(&document)? + "\n",
        )?);
        steps.push(plan_step(
            manifest_path,
            "createProfile",
            "Create the Profile-owned instruction manifest.",
        ));

        let legacy_profile_path = library_root
            .join(PROFILES_DIR)
            .join(format!("{}.json", legacy_profile.id));
        let legacy_profile_original = read_file_state(&legacy_profile_path)?;
        changes.push(PreparedChange {
            path: legacy_profile_path.clone(),
            original: legacy_profile_original,
            desired: FileState::Missing,
        });
        steps.push(plan_step(
            legacy_profile_path,
            "removeLegacyProfile",
            "Remove the old Pack-selection document after its Profile copy is prepared.",
        ));
        profiles.insert(document.id.clone(), LoadedProfile { document, files });
    }

    for pack in legacy.packs.values() {
        for source in &pack.files {
            let original = read_file_state(&source.path)?;
            changes.push(PreparedChange {
                path: source.path.clone(),
                original,
                desired: FileState::Missing,
            });
            steps.push(plan_step(
                source.path.clone(),
                "removeLegacyPackSource",
                "Remove the old Pack source after every referencing Profile has an exact copy.",
            ));
        }
        let manifest_path = library_root
            .join(PACKS_DIR)
            .join(&pack.document.id)
            .join(PACK_MANIFEST_FILE);
        let original = read_file_state(&manifest_path)?;
        changes.push(PreparedChange {
            path: manifest_path.clone(),
            original,
            desired: FileState::Missing,
        });
        steps.push(plan_step(
            manifest_path,
            "removeLegacyPack",
            "Remove the old Rule Pack manifest after all rules are copied.",
        ));
    }

    Ok(PreparedMigration {
        changes,
        steps,
        loaded: LoadedLibrary {
            profiles,
            legacy_source_hint: legacy.schema.legacy_source,
        },
    })
}

pub(crate) fn plan_create_profile(
    library_root: &Path,
    id: &str,
    name: &str,
    description: &str,
) -> Result<LibraryPlan, ArmError> {
    validate_id(id)?;
    validate_label("profile name", name, 80)?;
    validate_label("profile description", description, 240)?;
    ensure_ready(library_root)?;
    let directory = library_root.join(PROFILES_DIR).join(id);
    let blocked = path_entry_exists(&directory)?;
    let steps = if blocked {
        Vec::new()
    } else {
        vec![
            plan_step(
                directory.join(PROFILE_MANIFEST_FILE),
                "createProfile",
                "Create a versioned Profile manifest.",
            ),
            plan_step(
                directory.join(ENTRYPOINT_FILE),
                "createRules",
                "Create the required AGENTS.md entrypoint.",
            ),
        ]
    };
    Ok(LibraryPlan {
        operation: "createProfile".into(),
        blocked,
        change_count: steps.len(),
        summary: if blocked {
            format!("Profile `{id}` already exists; no file will be overwritten.")
        } else {
            format!("Create Profile `{name}` with a required AGENTS.md entrypoint.")
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
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_create_profile(library_root, id, name, description)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    let document = ProfileDocument {
        schema_version: LIBRARY_SCHEMA_VERSION,
        id: id.into(),
        name: name.trim().into(),
        description: description.trim().into(),
        instructions: vec![ENTRYPOINT_FILE.into()],
    };
    let directory = library_root.join(PROFILES_DIR).join(id);
    apply_changes(
        state_root,
        "createProfile",
        vec![
            missing_file_change(
                directory.join(PROFILE_MANIFEST_FILE),
                pretty_json(&document)? + "\n",
            )?,
            missing_file_change(directory.join(ENTRYPOINT_FILE), STARTER_RULES.into())?,
        ],
    )
}

pub(crate) fn plan_add_profile_file(
    library_root: &Path,
    profile_id: &str,
    relative_path: &str,
) -> Result<LibraryPlan, ArmError> {
    validate_id(profile_id)?;
    validate_instruction_path(relative_path)?;
    let loaded = load_library(library_root)?;
    let profile = loaded
        .profiles
        .get(profile_id)
        .ok_or_else(|| ArmError::UnknownProfile(profile_id.into()))?;
    let file_path = library_root
        .join(PROFILES_DIR)
        .join(profile_id)
        .join(relative_path);
    let already_declared = profile
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
                    .join(PROFILES_DIR)
                    .join(profile_id)
                    .join(PROFILE_MANIFEST_FILE),
                "updateProfile",
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
        operation: "addProfileFile".into(),
        blocked,
        change_count: steps.len(),
        summary: if blocked {
            format!(
                "`{relative_path}` is already declared or exists in Profile `{profile_id}`; it will not be overwritten."
            )
        } else {
            format!("Add `{relative_path}` after the existing sources in Profile `{profile_id}`.")
        },
        steps,
    })
}

pub(crate) fn add_profile_file(
    library_root: &Path,
    state_root: &Path,
    profile_id: &str,
    relative_path: &str,
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_add_profile_file(library_root, profile_id, relative_path)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    let manifest_path = library_root
        .join(PROFILES_DIR)
        .join(profile_id)
        .join(PROFILE_MANIFEST_FILE);
    let manifest_original = read_file_state(&manifest_path)?;
    let mut document: ProfileDocument = match &manifest_original {
        FileState::File { content } => serde_json::from_str(content)?,
        _ => return Err(ArmError::ApplyDrift(manifest_path.display().to_string())),
    };
    validate_profile_document(&document, profile_id)?;
    if document
        .instructions
        .iter()
        .any(|path| path == relative_path)
    {
        return Err(ArmError::ApplyDrift(manifest_path.display().to_string()));
    }
    document.instructions.push(relative_path.into());
    let file_path = library_root
        .join(PROFILES_DIR)
        .join(profile_id)
        .join(relative_path);
    let title = Path::new(relative_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Supplemental rules")
        .replace('-', " ")
        .replace('_', " ");
    apply_changes(
        state_root,
        "addProfileFile",
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

pub(crate) fn plan_remove_profile_file(
    library_root: &Path,
    profile_id: &str,
    relative_path: &str,
) -> Result<LibraryPlan, ArmError> {
    validate_id(profile_id)?;
    validate_instruction_path(relative_path)?;
    let loaded = load_library(library_root)?;
    let profile = loaded
        .profiles
        .get(profile_id)
        .ok_or_else(|| ArmError::UnknownProfile(profile_id.into()))?;
    let declared = profile
        .document
        .instructions
        .iter()
        .any(|path| path == relative_path);
    let required_entrypoint = relative_path == ENTRYPOINT_FILE;
    let blocked = required_entrypoint || !declared;
    let file_path = library_root
        .join(PROFILES_DIR)
        .join(profile_id)
        .join(relative_path);
    let steps = if blocked {
        Vec::new()
    } else {
        vec![
            plan_step(
                library_root
                    .join(PROFILES_DIR)
                    .join(profile_id)
                    .join(PROFILE_MANIFEST_FILE),
                "updateProfile",
                "Remove the Markdown path from the ordered instruction manifest.",
            ),
            plan_step(
                file_path,
                "deleteRuleFile",
                "Delete the declared Markdown source after snapshotting its exact contents.",
            ),
        ]
    };
    Ok(LibraryPlan {
        operation: "removeProfileFile".into(),
        blocked,
        change_count: steps.len(),
        summary: if required_entrypoint {
            format!(
                "`{ENTRYPOINT_FILE}` is the required first source in Profile `{profile_id}` and cannot be removed."
            )
        } else if !declared {
            format!(
                "`{relative_path}` is not declared by Profile `{profile_id}`; no file will be deleted."
            )
        } else {
            format!(
                "Remove `{relative_path}` from Profile `{profile_id}` and delete its declared Markdown source with rollback protection."
            )
        },
        steps,
    })
}

pub(crate) fn remove_profile_file(
    library_root: &Path,
    state_root: &Path,
    profile_id: &str,
    relative_path: &str,
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_remove_profile_file(library_root, profile_id, relative_path)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    let manifest_path = library_root
        .join(PROFILES_DIR)
        .join(profile_id)
        .join(PROFILE_MANIFEST_FILE);
    let manifest_original = read_file_state(&manifest_path)?;
    let mut document: ProfileDocument = match &manifest_original {
        FileState::File { content } => serde_json::from_str(content)?,
        _ => return Err(ArmError::ApplyDrift(manifest_path.display().to_string())),
    };
    validate_profile_document(&document, profile_id)?;
    let position = document
        .instructions
        .iter()
        .position(|path| path == relative_path)
        .ok_or_else(|| ArmError::ApplyDrift(manifest_path.display().to_string()))?;
    if position == 0 || relative_path == ENTRYPOINT_FILE {
        return Err(ArmError::ApplyDrift(manifest_path.display().to_string()));
    }
    document.instructions.remove(position);
    let file_path = library_root
        .join(PROFILES_DIR)
        .join(profile_id)
        .join(relative_path);
    let file_original = read_file_state(&file_path)?;
    if !matches!(file_original, FileState::File { .. }) {
        return Err(ArmError::ApplyDrift(file_path.display().to_string()));
    }
    apply_changes(
        state_root,
        "removeProfileFile",
        vec![
            PreparedChange {
                path: manifest_path,
                original: manifest_original,
                desired: FileState::File {
                    content: pretty_json(&document)? + "\n",
                },
            },
            PreparedChange {
                path: file_path,
                original: file_original,
                desired: FileState::Missing,
            },
        ],
    )
}

pub(crate) fn plan_delete_profile(
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
    let directory = library_root.join(PROFILES_DIR).join(profile_id);
    if let Some(summary) = profile_deletion_blocker(&directory, state_root, profile)? {
        return Ok(LibraryPlan {
            operation: "deleteProfile".into(),
            blocked: true,
            change_count: 0,
            summary,
            steps: Vec::new(),
        });
    }
    let prepared = prepare_profile_deletion(&directory, profile)?;
    Ok(LibraryPlan {
        operation: "deleteProfile".into(),
        blocked: false,
        change_count: prepared.steps.len(),
        summary: format!(
            "Delete inactive Profile `{profile_id}` and its {} declared Markdown source(s) after creating a rollback snapshot.",
            profile.files.len()
        ),
        steps: prepared.steps,
    })
}

pub(crate) fn delete_profile(
    library_root: &Path,
    state_root: &Path,
    profile_id: &str,
) -> Result<LibraryMutationOutcome, ArmError> {
    let plan = plan_delete_profile(library_root, state_root, profile_id)?;
    if plan.blocked {
        return Err(ArmError::Blocked(plan.summary));
    }
    let loaded = load_library(library_root)?;
    let profile = loaded
        .profiles
        .get(profile_id)
        .ok_or_else(|| ArmError::UnknownProfile(profile_id.into()))?;
    let directory = library_root.join(PROFILES_DIR).join(profile_id);
    if let Some(summary) = profile_deletion_blocker(&directory, state_root, profile)? {
        return Err(ArmError::Blocked(summary));
    }
    let prepared = prepare_profile_deletion(&directory, profile)?;
    let mut outcome = apply_changes(state_root, "deleteProfile", prepared.changes)?;
    let backup_id = outcome
        .backup_id
        .as_deref()
        .ok_or_else(|| ArmError::ApplyDrift(directory.display().to_string()))?;
    match remove_profile_directories(&prepared.directories) {
        Ok(removed) => {
            outcome.changed.extend(removed);
            Ok(outcome)
        }
        Err(error) => {
            let backup_path = library_backup_path(state_root, backup_id);
            match rollback_library_backup(state_root, &backup_path) {
                Ok(_) => Err(error),
                Err(rollback_error) => Err(rollback_error),
            }
        }
    }
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
    let rendered = render_profile(profile)?;
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
            schema_version: MACHINE_SCHEMA_VERSION,
            active_profile_id: profile_id.into(),
        })? + "\n",
    };
    let current_allowed = match &current {
        FileState::Missing => true,
        FileState::Symlink { target } => managed_runtime_link(library_root, &current_path, target),
        FileState::File { .. } => false,
    };
    let machine_allowed = matches!(machine, FileState::Missing | FileState::File { .. });
    let runtime_current = runtime_matches(&runtime_path, &rendered)?;
    let runtime_conflict = path_entry_exists(&runtime_path)? && !runtime_current;
    let blocked = !current_allowed || !machine_allowed || runtime_conflict;
    let mut steps = Vec::new();
    if blocked {
        return Ok(LibraryPlan {
            operation: "activateProfile".into(),
            blocked: true,
            change_count: 0,
            summary: if runtime_conflict {
                format!(
                    "Activation is blocked by an unexpected immutable runtime: {}",
                    runtime_path.display()
                )
            } else {
                "Activation is blocked by an unmanaged `current` link or machine state entry."
                    .into()
            },
            steps,
        });
    }
    if !runtime_current {
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
    let rendered = render_profile(profile)?;
    let runtime_path = library_root.join(RUNTIME_DIR).join(&rendered.runtime_name);
    let current_path = library_root.join(CURRENT_LINK);
    let machine_path = state_root.join(MACHINE_FILE);
    let desired_current = FileState::Symlink {
        target: format!("{RUNTIME_DIR}/{}", rendered.runtime_name),
    };
    let desired_machine = FileState::File {
        content: pretty_json(&MachineDocument {
            schema_version: MACHINE_SCHEMA_VERSION,
            active_profile_id: profile_id.into(),
        })? + "\n",
    };
    let mut changes = prepare_runtime_changes(&runtime_path, &rendered)?;
    let rendered_runtime = !changes.is_empty();
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
    if rendered_runtime {
        outcome
            .changed
            .retain(|path| !Path::new(path).starts_with(&runtime_path));
        outcome
            .changed
            .insert(0, runtime_path.display().to_string());
    }
    Ok(outcome)
}

pub(crate) fn rule_source_path(
    library_root: &Path,
    profile_id: &str,
    relative_path: &str,
) -> Result<PathBuf, ArmError> {
    let loaded = load_library(library_root)?;
    let profile = loaded
        .profiles
        .get(profile_id)
        .ok_or_else(|| ArmError::UnknownProfile(profile_id.into()))?;
    profile
        .files
        .iter()
        .find(|file| file.relative_path == relative_path)
        .map(|file| file.path.clone())
        .ok_or_else(|| {
            ArmError::InvalidLibrary(format!(
                "`{relative_path}` is not declared by Profile `{profile_id}`"
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
    rollback_library_backup(state_root, &backup_path)
}

fn rollback_library_backup(
    state_root: &Path,
    backup_path: &Path,
) -> Result<RollbackOutcome, ArmError> {
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
    cleanup_created_directories(&backup.created_directories)?;
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
            return Ok((
                LibraryState::Empty,
                "The rule library has not been created.".into(),
            ))
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
        let document: SchemaDocument = read_json(&schema)?;
        if document.format != "agent-rules-library" {
            return Ok((
                LibraryState::Conflict,
                "schema.json uses an unsupported rule-library format.".into(),
            ));
        }
        return match document.schema_version {
            LIBRARY_SCHEMA_VERSION => {
                Ok((LibraryState::Ready, "The Profile library is ready.".into()))
            }
            LEGACY_LIBRARY_SCHEMA_VERSION => Ok((
                LibraryState::Upgrade,
                "This Rule Pack library can be migrated into Profile-owned rules.".into(),
            )),
            version => Ok((
                LibraryState::Conflict,
                format!("schema.json uses unsupported version {version}."),
            )),
        };
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
        let entry =
            entry.map_err(|error| ArmError::io(library_root.display().to_string(), error))?;
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
            "A legacy root AGENTS.md can be migrated into a Profile with rollback protection."
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
    if schema.schema_version != LIBRARY_SCHEMA_VERSION || schema.format != "agent-rules-library" {
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

    let profiles_root = library_root.join(PROFILES_DIR);
    let mut profiles = BTreeMap::new();
    for path in sorted_entries(&profiles_root)? {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| ArmError::io(path.display().to_string(), error))?;
        if !metadata.is_dir() {
            return Err(ArmError::InvalidLibrary(format!(
                "Profile entry is not a directory: {}",
                path.display()
            )));
        }
        let directory_id = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| ArmError::InvalidLibrary(path.display().to_string()))?
            .to_string();
        validate_id(&directory_id)?;
        let manifest_path = path.join(PROFILE_MANIFEST_FILE);
        let manifest_metadata = fs::symlink_metadata(&manifest_path)
            .map_err(|error| ArmError::io(manifest_path.display().to_string(), error))?;
        if !manifest_metadata.is_file() {
            return Err(ArmError::InvalidLibrary(format!(
                "Profile manifest must be a regular file: {}",
                manifest_path.display()
            )));
        }
        let document: ProfileDocument = read_json(&manifest_path)?;
        validate_profile_document(&document, &directory_id)?;
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
        if profiles
            .insert(document.id.clone(), LoadedProfile { document, files })
            .is_some()
        {
            return Err(ArmError::InvalidLibrary(format!(
                "duplicate Profile id `{directory_id}`"
            )));
        }
    }

    Ok(LoadedLibrary {
        profiles,
        legacy_source_hint: schema.legacy_source,
    })
}

fn load_legacy_library(library_root: &Path) -> Result<LoadedLegacyLibrary, ArmError> {
    let schema_path = library_root.join(SCHEMA_FILE);
    let schema: SchemaDocument = read_json(&schema_path)?;
    if schema.schema_version != LEGACY_LIBRARY_SCHEMA_VERSION
        || schema.format != "agent-rules-library"
    {
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
        let document: LegacyPackDocument = read_json(&manifest_path)?;
        validate_legacy_pack_document(&document, &directory_id)?;
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
        let declared = std::iter::once(PACK_MANIFEST_FILE.to_string())
            .chain(document.instructions.iter().cloned())
            .collect::<BTreeSet<_>>();
        validate_legacy_owned_files(&path, &path, &declared)?;
        if packs
            .insert(document.id.clone(), LoadedLegacyPack { document, files })
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
        if !metadata.is_file() || path.extension().and_then(|value| value.to_str()) != Some("json")
        {
            return Err(ArmError::InvalidLibrary(format!(
                "Profile entry must be a JSON file: {}",
                path.display()
            )));
        }
        let file_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| ArmError::InvalidLibrary(path.display().to_string()))?;
        let document: LegacyProfileDocument = read_json(&path)?;
        validate_legacy_profile_document(&document, file_id, &packs)?;
        if profiles.insert(document.id.clone(), document).is_some() {
            return Err(ArmError::InvalidLibrary(format!(
                "duplicate Profile id `{file_id}`"
            )));
        }
    }
    Ok(LoadedLegacyLibrary {
        schema,
        packs,
        profiles,
    })
}

fn validate_legacy_owned_files(
    root: &Path,
    directory: &Path,
    declared: &BTreeSet<String>,
) -> Result<(), ArmError> {
    for path in sorted_entries(directory)? {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| ArmError::io(path.display().to_string(), error))?;
        if metadata.is_dir() {
            validate_legacy_owned_files(root, &path, declared)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| ArmError::InvalidLibrary(path.display().to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        if !metadata.is_file() || !declared.contains(&relative) {
            return Err(ArmError::InvalidLibrary(format!(
                "legacy Rule Pack contains unmanaged entry `{relative}`; migration will not remove it"
            )));
        }
    }
    Ok(())
}

fn validate_profile_document(
    document: &ProfileDocument,
    directory_id: &str,
) -> Result<(), ArmError> {
    if document.schema_version != LIBRARY_SCHEMA_VERSION {
        return Err(ArmError::InvalidLibrary(format!(
            "Profile `{directory_id}` uses an unsupported schema"
        )));
    }
    validate_id(&document.id)?;
    validate_label("profile name", &document.name, 80)?;
    validate_label("profile description", &document.description, 240)?;
    if document.id != directory_id {
        return Err(ArmError::InvalidLibrary(format!(
            "Profile id `{}` does not match directory `{directory_id}`",
            document.id
        )));
    }
    if document.instructions.first().map(String::as_str) != Some(ENTRYPOINT_FILE) {
        return Err(ArmError::InvalidLibrary(format!(
            "Profile `{directory_id}` must declare AGENTS.md as its first instruction file"
        )));
    }
    let mut seen = BTreeSet::new();
    for path in &document.instructions {
        validate_instruction_path(path)?;
        if !seen.insert(path) {
            return Err(ArmError::InvalidLibrary(format!(
                "Profile `{directory_id}` declares `{path}` more than once"
            )));
        }
    }
    Ok(())
}

fn profile_deletion_blocker(
    directory: &Path,
    state_root: &Path,
    profile: &LoadedProfile,
) -> Result<Option<String>, ArmError> {
    if read_machine(state_root)?
        .as_ref()
        .is_some_and(|machine| machine.active_profile_id == profile.document.id)
    {
        return Ok(Some(format!(
            "Profile `{}` is active on this machine; switch to another Profile before deleting it.",
            profile.document.id
        )));
    }
    let (declared_files, declared_directories) = profile_declared_entries(profile);
    if let Some(relative) =
        first_unmanaged_profile_entry(directory, directory, &declared_files, &declared_directories)?
    {
        return Ok(Some(format!(
            "Profile `{}` contains unmanaged entry `{relative}`; deletion will not touch it.",
            profile.document.id
        )));
    }
    Ok(None)
}

fn profile_declared_entries(profile: &LoadedProfile) -> (BTreeSet<String>, BTreeSet<String>) {
    let declared_files = std::iter::once(PROFILE_MANIFEST_FILE.to_string())
        .chain(profile.document.instructions.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut declared_directories = BTreeSet::new();
    for relative in &profile.document.instructions {
        let mut parent = Path::new(relative).parent();
        while let Some(directory) = parent {
            if directory.as_os_str().is_empty() {
                break;
            }
            declared_directories.insert(directory.to_string_lossy().replace('\\', "/"));
            parent = directory.parent();
        }
    }
    (declared_files, declared_directories)
}

fn first_unmanaged_profile_entry(
    root: &Path,
    directory: &Path,
    declared_files: &BTreeSet<String>,
    declared_directories: &BTreeSet<String>,
) -> Result<Option<String>, ArmError> {
    for path in sorted_entries(directory)? {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| ArmError::io(path.display().to_string(), error))?;
        let relative = path
            .strip_prefix(root)
            .map_err(|_| ArmError::InvalidLibrary(path.display().to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        if metadata.is_dir() {
            if !declared_directories.contains(&relative) {
                return Ok(Some(relative));
            }
            if let Some(unmanaged) =
                first_unmanaged_profile_entry(root, &path, declared_files, declared_directories)?
            {
                return Ok(Some(unmanaged));
            }
        } else if !metadata.is_file() || !declared_files.contains(&relative) {
            return Ok(Some(relative));
        }
    }
    Ok(None)
}

fn prepare_profile_deletion(
    directory: &Path,
    profile: &LoadedProfile,
) -> Result<PreparedProfileDeletion, ArmError> {
    let mut changes = Vec::new();
    let mut steps = Vec::new();
    for file in &profile.files {
        let original = read_file_state(&file.path)?;
        if !matches!(original, FileState::File { .. }) {
            return Err(ArmError::ApplyDrift(file.path.display().to_string()));
        }
        changes.push(PreparedChange {
            path: file.path.clone(),
            original,
            desired: FileState::Missing,
        });
        steps.push(plan_step(
            file.path.clone(),
            "deleteProfileSource",
            "Delete this declared Markdown source after snapshotting its exact contents.",
        ));
    }
    let manifest_path = directory.join(PROFILE_MANIFEST_FILE);
    let manifest_original = read_file_state(&manifest_path)?;
    if !matches!(manifest_original, FileState::File { .. }) {
        return Err(ArmError::ApplyDrift(manifest_path.display().to_string()));
    }
    changes.push(PreparedChange {
        path: manifest_path.clone(),
        original: manifest_original,
        desired: FileState::Missing,
    });
    steps.push(plan_step(
        manifest_path,
        "deleteProfileManifest",
        "Delete the Profile manifest after every declared source is snapshotted.",
    ));

    let (_, declared_directories) = profile_declared_entries(profile);
    let mut directories = declared_directories
        .into_iter()
        .map(|relative| directory.join(relative))
        .collect::<Vec<_>>();
    directories.push(directory.to_path_buf());
    directories.sort_by(|left, right| {
        right
            .components()
            .count()
            .cmp(&left.components().count())
            .then_with(|| right.cmp(left))
    });
    for path in &directories {
        steps.push(plan_step(
            path.clone(),
            "deleteProfileDirectory",
            if path == directory {
                "Remove the empty Profile directory."
            } else {
                "Remove this empty Profile source directory."
            },
        ));
    }
    Ok(PreparedProfileDeletion {
        changes,
        directories,
        steps,
    })
}

fn remove_profile_directories(directories: &[PathBuf]) -> Result<Vec<String>, ArmError> {
    let mut removed = Vec::new();
    for directory in directories {
        let metadata = match fs::symlink_metadata(directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(ArmError::ApplyDrift(directory.display().to_string()))
            }
            Err(error) => return Err(ArmError::io(directory.display().to_string(), error)),
        };
        if !metadata.is_dir() {
            return Err(ArmError::ApplyDrift(directory.display().to_string()));
        }
        let mut entries = fs::read_dir(directory)
            .map_err(|error| ArmError::io(directory.display().to_string(), error))?;
        if entries.next().is_some() {
            return Err(ArmError::ApplyDrift(directory.display().to_string()));
        }
        match fs::remove_dir(directory) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) =>
            {
                return Err(ArmError::ApplyDrift(directory.display().to_string()))
            }
            Err(error) => return Err(ArmError::io(directory.display().to_string(), error)),
        }
        removed.push(directory.display().to_string());
    }
    Ok(removed)
}

fn validate_legacy_pack_document(
    document: &LegacyPackDocument,
    directory_id: &str,
) -> Result<(), ArmError> {
    if document.schema_version != LEGACY_LIBRARY_SCHEMA_VERSION {
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

fn validate_legacy_profile_document(
    document: &LegacyProfileDocument,
    file_id: &str,
    packs: &BTreeMap<String, LoadedLegacyPack>,
) -> Result<(), ArmError> {
    if document.schema_version != LEGACY_LIBRARY_SCHEMA_VERSION {
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
    validate_legacy_pack_selection(packs, &document.packs)
}

fn validate_legacy_pack_selection(
    packs: &BTreeMap<String, LoadedLegacyPack>,
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
            return Err(ArmError::InvalidLibrary(format!(
                "legacy Profile references unknown Rule Pack `{id}`"
            )));
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
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
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

fn render_default_profile(library_root: &Path, rules: &str) -> Result<RenderedProfile, ArmError> {
    let profile = LoadedProfile {
        document: ProfileDocument {
            schema_version: LIBRARY_SCHEMA_VERSION,
            id: "default".into(),
            name: "Default".into(),
            description: "The initial machine profile.".into(),
            instructions: vec![ENTRYPOINT_FILE.into()],
        },
        files: vec![LoadedRuleFile {
            relative_path: ENTRYPOINT_FILE.into(),
            path: library_root
                .join(PROFILES_DIR)
                .join("default")
                .join(ENTRYPOINT_FILE),
            content: rules.into(),
            digest: short_digest(rules),
            modified_at: None,
        }],
    };
    render_profile(&profile)
}

fn render_profile(profile: &LoadedProfile) -> Result<RenderedProfile, ArmError> {
    let mut source_material = format!("profile:{}\n", profile.document.id);
    let mut sources = Vec::new();
    let mut content = format!(
        "<!-- Generated by Agent Rules Manager. Edit Profile sources, not this file. -->\n<!-- Profile: {} -->\n",
        profile.document.id
    );
    for file in &profile.files {
        let logical = format!("profiles/{}/{}", profile.document.id, file.relative_path);
        source_material.push_str(&logical);
        source_material.push('\0');
        source_material.push_str(&file.content);
        source_material.push('\0');
        content.push_str(&format!("\n<!-- Source: {logical} -->\n"));
        content.push_str(file.content.trim_end_matches('\n'));
        content.push('\n');
        sources.push(RuntimeSource {
            path: file.relative_path.clone(),
            digest: file.digest.clone(),
        });
    }
    let profile_digest = short_digest(&source_material);
    let rendered_digest = short_digest(&content);
    let runtime_name = format!("{}-{profile_digest}", profile.document.id);
    let manifest = RuntimeManifest {
        schema_version: RUNTIME_SCHEMA_VERSION,
        profile_id: profile.document.id.clone(),
        profile_digest: profile_digest.clone(),
        rendered_digest: rendered_digest.clone(),
        sources,
    };
    Ok(RenderedProfile {
        runtime_name,
        content,
        manifest,
    })
}

fn inspect_current(
    library_root: &Path,
    rendered: &RenderedProfile,
) -> Result<RuntimeState, ArmError> {
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

fn prepare_runtime_changes(
    runtime_path: &Path,
    rendered: &RenderedProfile,
) -> Result<Vec<PreparedChange>, ArmError> {
    if path_entry_exists(runtime_path)? {
        if runtime_matches(runtime_path, rendered)? {
            return Ok(Vec::new());
        }
        return Err(ArmError::Blocked(format!(
            "immutable runtime already exists with unexpected content: {}",
            runtime_path.display()
        )));
    }
    Ok(vec![
        missing_file_change(runtime_path.join(ENTRYPOINT_FILE), rendered.content.clone())?,
        missing_file_change(
            runtime_path.join("manifest.json"),
            pretty_json(&rendered.manifest)? + "\n",
        )?,
    ])
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
    let document: MachineDocument = match read_file_state(&path)? {
        FileState::Missing => return Ok(None),
        FileState::File { content } => serde_json::from_str(&content)?,
        FileState::Symlink { .. } => {
            return Err(ArmError::InvalidLibrary(format!(
                "machine state must be a regular file: {}",
                path.display()
            )))
        }
    };
    if document.schema_version != MACHINE_SCHEMA_VERSION {
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

fn collect_missing_parent_directories(changes: &[PreparedChange]) -> Result<Vec<String>, ArmError> {
    let mut directories = BTreeSet::new();
    for change in changes {
        if change.desired == FileState::Missing {
            continue;
        }
        let mut cursor = change.path.parent();
        while let Some(directory) = cursor {
            match fs::symlink_metadata(directory) {
                Ok(metadata) => {
                    if !metadata.is_dir() {
                        return Err(ArmError::UnsupportedEntry(directory.display().to_string()));
                    }
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    directories.insert(directory.to_path_buf());
                    cursor = directory.parent();
                }
                Err(error) => {
                    return Err(ArmError::io(directory.display().to_string(), error));
                }
            }
        }
    }
    let mut directories = directories.into_iter().collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        right
            .components()
            .count()
            .cmp(&left.components().count())
            .then_with(|| right.cmp(left))
    });
    Ok(directories
        .into_iter()
        .map(|path| path.display().to_string())
        .collect())
}

fn cleanup_created_directories(directories: &[String]) -> Result<(), ArmError> {
    let mut directories = directories.iter().map(PathBuf::from).collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        right
            .components()
            .count()
            .cmp(&left.components().count())
            .then_with(|| right.cmp(left))
    });
    for directory in directories {
        let metadata = match fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(ArmError::io(directory.display().to_string(), error)),
        };
        if !metadata.is_dir() {
            continue;
        }
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| ArmError::io(directory.display().to_string(), error))?;
        if entries.next().is_none() {
            match fs::remove_dir(&directory) {
                Ok(()) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                    ) => {}
                Err(error) => return Err(ArmError::io(directory.display().to_string(), error)),
            }
        }
    }
    Ok(())
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
    let created_directories = collect_missing_parent_directories(&changes)?;
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
        created_directories: created_directories.clone(),
    };
    let backup_path = write_backup(state_root, &backup)?;
    let mut completed = 0;
    for change in &changes {
        let current = match read_file_state(&change.path) {
            Ok(current) => current,
            Err(error) => {
                if restore_completed(&changes[..completed], &created_directories) {
                    let _ = archive_backup(state_root, &backup_path);
                }
                return Err(error);
            }
        };
        if current != change.original {
            if restore_completed(&changes[..completed], &created_directories) {
                let _ = archive_backup(state_root, &backup_path);
            }
            return Err(ArmError::ApplyDrift(change.path.display().to_string()));
        }
        if let Err(error) = write_file_state(&change.path, &change.desired) {
            if restore_completed(&changes[..completed], &created_directories) {
                let _ = archive_backup(state_root, &backup_path);
            }
            return Err(error);
        }
        let written = match read_file_state(&change.path) {
            Ok(written) => written,
            Err(error) => {
                if restore_completed(&changes[..=completed], &created_directories) {
                    let _ = archive_backup(state_root, &backup_path);
                }
                return Err(error);
            }
        };
        if written != change.desired {
            if restore_completed(&changes[..=completed], &created_directories) {
                let _ = archive_backup(state_root, &backup_path);
            }
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
    let path = library_backup_path(state_root, &backup.id);
    write_atomic(&path, &(pretty_json(backup)? + "\n"))?;
    Ok(path)
}

fn library_backup_path(state_root: &Path, backup_id: &str) -> PathBuf {
    state_root
        .join("library-backups")
        .join(format!("{backup_id}.json"))
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

fn restore_completed(changes: &[PreparedChange], created_directories: &[String]) -> bool {
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
    if restored && cleanup_created_directories(created_directories).is_err() {
        restored = false;
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
        let target =
            fs::read_link(path).map_err(|error| ArmError::io(path.display().to_string(), error))?;
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

    fn write_json_fixture<T: Serialize>(path: &Path, value: &T) {
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        fs::write(path, pretty_json(value).expect("fixture json") + "\n").expect("fixture write");
    }

    fn write_v1_library(library: &Path, state: &Path) {
        let schema = SchemaDocument {
            schema_version: LEGACY_LIBRARY_SCHEMA_VERSION,
            format: "agent-rules-library".into(),
            sync: SyncPolicy {
                include: vec![
                    "schema.json".into(),
                    "packs/**".into(),
                    "profiles/**".into(),
                ],
                exclude: vec![
                    ".runtime/**".into(),
                    "current".into(),
                    "machine state".into(),
                ],
                machine_selection: "local".into(),
            },
            legacy_source: None,
        };
        let base = LegacyPackDocument {
            schema_version: LEGACY_LIBRARY_SCHEMA_VERSION,
            id: "base".into(),
            name: "Base".into(),
            description: "Shared rules".into(),
            instructions: vec!["AGENTS.md".into(), "rules/review.md".into()],
        };
        let work = LegacyPackDocument {
            schema_version: LEGACY_LIBRARY_SCHEMA_VERSION,
            id: "work".into(),
            name: "Work".into(),
            description: "Work rules".into(),
            instructions: vec!["AGENTS.md".into()],
        };
        let profile = LegacyProfileDocument {
            schema_version: LEGACY_LIBRARY_SCHEMA_VERSION,
            id: "default".into(),
            name: "Default".into(),
            description: "Default route".into(),
            packs: vec!["base".into(), "work".into()],
        };
        let machine = MachineDocument {
            schema_version: MACHINE_SCHEMA_VERSION,
            active_profile_id: "default".into(),
        };

        write_json_fixture(&library.join(SCHEMA_FILE), &schema);
        write_json_fixture(&library.join("packs/base/pack.json"), &base);
        fs::write(library.join("packs/base/AGENTS.md"), "# Base\n").expect("base rules");
        fs::create_dir_all(library.join("packs/base/rules")).expect("base rule directory");
        fs::write(library.join("packs/base/rules/review.md"), "# Review\n").expect("review rules");
        write_json_fixture(&library.join("packs/work/pack.json"), &work);
        fs::write(library.join("packs/work/AGENTS.md"), "# Work\n").expect("work rules");
        write_json_fixture(&library.join("profiles/default.json"), &profile);
        write_json_fixture(&state.join(MACHINE_FILE), &machine);
    }

    #[test]
    fn initialize_creates_profile_runtime_and_local_selection() {
        let (_temporary, library, state) = fixture();
        let plan = plan_initialize(&library, &state).expect("plan");
        assert!(!plan.blocked);
        assert_eq!(plan.change_count, 7);

        initialize(&library, &state).expect("initialize");
        let snapshot = inspect(&library, &state).expect("inspect");
        assert_eq!(snapshot.state, LibraryState::Ready);
        assert_eq!(snapshot.runtime_state, RuntimeState::Current);
        assert_eq!(snapshot.active_profile_id.as_deref(), Some("default"));
        assert_eq!(snapshot.profiles[0].files[0].path, "AGENTS.md");
        assert!(library.join("current/AGENTS.md").is_file());
        assert!(library.join("current").is_symlink());
    }

    #[test]
    fn legacy_source_is_migrated_and_rollback_can_restore_it() {
        let (_temporary, library, state) = fixture();
        fs::create_dir_all(&library).expect("library");
        fs::write(library.join(LEGACY_SOURCE_FILE), "# My existing rules\n").expect("legacy");

        let plan = plan_initialize(&library, &state).expect("plan");
        assert_eq!(plan.change_count, 8);
        assert!(plan.steps.iter().any(|step| step.action == "copyLegacy"));
        assert!(plan.steps.iter().any(|step| step.action == "removeLegacy"));
        initialize(&library, &state).expect("initialize");

        assert!(!library.join(LEGACY_SOURCE_FILE).exists());
        assert_eq!(
            fs::read_to_string(library.join("profiles/default/AGENTS.md")).unwrap(),
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
        assert!(!library.join("profiles/default/AGENTS.md").exists());
        assert!(!library.join(RUNTIME_DIR).exists());
        assert_eq!(
            inspect(&library, &state)
                .expect("inspect restored legacy")
                .state,
            LibraryState::Legacy
        );
        assert!(
            !plan_initialize(&library, &state)
                .expect("replan restored legacy")
                .blocked
        );
    }

    #[test]
    fn empty_library_initialization_rolls_back_without_directory_residue() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");

        rollback_library_latest(&state).expect("rollback activation");
        rollback_library_latest(&state).expect("rollback initialization");

        assert!(!library.exists());
        assert_eq!(
            inspect(&library, &state).expect("inspect empty").state,
            LibraryState::Empty
        );
    }

    #[test]
    fn failed_library_transaction_archives_its_restored_backup() {
        let (_temporary, library, state) = fixture();
        let first = library.join("profile/first.md");
        let second = library.join("profile/second.md");
        let changes = vec![
            missing_file_change(first.clone(), "first\n".into()).expect("first change"),
            missing_file_change(second.clone(), "second\n".into()).expect("second change"),
        ];
        fs::create_dir_all(second.parent().unwrap()).expect("external parent");
        fs::write(&second, "external\n").expect("external drift");

        assert!(matches!(
            apply_changes(&state, "testFailure", changes),
            Err(ArmError::ApplyDrift(_))
        ));
        assert!(!first.exists());
        assert_eq!(fs::read_to_string(second).unwrap(), "external\n");
        assert_eq!(latest_backup_path(&state).expect("latest backup"), None);
        assert_eq!(
            sorted_entries(&state.join("library-backups/restored"))
                .expect("restored backups")
                .len(),
            1
        );
    }

    #[test]
    fn multiple_markdown_sources_render_in_manifest_order() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        let add_plan = plan_add_profile_file(&library, "default", "rules/review.md").expect("plan");
        assert_eq!(add_plan.change_count, 2);
        add_profile_file(&library, &state, "default", "rules/review.md").expect("add file");
        fs::write(
            library.join("profiles/default/rules/review.md"),
            "# Review\n",
        )
        .expect("rule edit");

        assert_eq!(
            inspect(&library, &state).unwrap().runtime_state,
            RuntimeState::Stale
        );

        let plan = plan_activate_profile(&library, &state, "default").expect("plan");
        assert!(plan.steps.iter().any(|step| step.action == "renderRuntime"));
        activate_profile(&library, &state, "default").expect("refresh");
        let rendered = fs::read_to_string(library.join("current/AGENTS.md")).unwrap();
        assert!(rendered.contains("<!-- Source: profiles/default/AGENTS.md -->"));
        assert!(rendered.contains("<!-- Source: profiles/default/rules/review.md -->"));
        assert!(rendered.find("AGENTS.md").unwrap() < rendered.find("review.md").unwrap());
        assert!(!rendered.contains(&library.display().to_string()));
    }

    #[test]
    fn profile_switch_is_local_and_rollback_refuses_drift() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        create_profile(&library, &state, "work", "Work", "").expect("profile");
        fs::write(library.join("profiles/work/AGENTS.md"), "# Work\n").expect("work rules");
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
        let manifest = library.join("profiles/default/profile.json");
        let original = fs::read_to_string(&manifest).expect("original manifest");

        add_profile_file(&library, &state, "default", "rules/testing.md").expect("add file");
        assert!(library.join("profiles/default/rules/testing.md").is_file());
        assert!(fs::read_to_string(&manifest)
            .unwrap()
            .contains("rules/testing.md"));

        rollback_library_latest(&state).expect("rollback");
        assert_eq!(fs::read_to_string(&manifest).unwrap(), original);
        assert!(!library.join("profiles/default/rules/testing.md").exists());
    }

    #[test]
    fn supplemental_file_removal_updates_manifest_and_rolls_back_exact_content() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        add_profile_file(&library, &state, "default", "rules/testing.md").expect("add file");
        let source = library.join("profiles/default/rules/testing.md");
        fs::write(&source, "# Testing\n\nKeep this exact content.\n").expect("edit source");
        activate_profile(&library, &state, "default").expect("refresh runtime");

        let plan = plan_remove_profile_file(&library, "default", "rules/testing.md").expect("plan");
        assert!(!plan.blocked);
        assert_eq!(plan.operation, "removeProfileFile");
        assert_eq!(plan.change_count, 2);
        assert_eq!(plan.steps[0].action, "updateProfile");
        assert_eq!(plan.steps[1].action, "deleteRuleFile");

        remove_profile_file(&library, &state, "default", "rules/testing.md").expect("remove file");
        assert!(!source.exists());
        assert!(
            !fs::read_to_string(library.join("profiles/default/profile.json"))
                .unwrap()
                .contains("rules/testing.md")
        );
        assert_eq!(
            inspect(&library, &state).expect("inspect").runtime_state,
            RuntimeState::Stale
        );

        rollback_library_latest(&state).expect("rollback removal");
        assert_eq!(
            fs::read_to_string(&source).unwrap(),
            "# Testing\n\nKeep this exact content.\n"
        );
        assert!(
            fs::read_to_string(library.join("profiles/default/profile.json"))
                .unwrap()
                .contains("rules/testing.md")
        );
        assert_eq!(
            inspect(&library, &state)
                .expect("inspect restored")
                .runtime_state,
            RuntimeState::Current
        );
    }

    #[test]
    fn profile_file_removal_blocks_required_and_undeclared_sources() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");

        let required =
            plan_remove_profile_file(&library, "default", ENTRYPOINT_FILE).expect("required plan");
        assert!(required.blocked);
        assert!(required.summary.contains("required first source"));
        assert!(matches!(
            remove_profile_file(&library, &state, "default", ENTRYPOINT_FILE),
            Err(ArmError::Blocked(_))
        ));

        let undeclared = plan_remove_profile_file(&library, "default", "rules/unknown.md")
            .expect("undeclared plan");
        assert!(undeclared.blocked);
        assert!(undeclared.summary.contains("is not declared"));
        assert!(matches!(
            remove_profile_file(&library, &state, "default", "rules/unknown.md"),
            Err(ArmError::Blocked(_))
        ));
        assert!(library.join("profiles/default/AGENTS.md").is_file());
    }

    #[test]
    fn profile_file_removal_rollback_refuses_recreated_source_drift() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        add_profile_file(&library, &state, "default", "rules/testing.md").expect("add file");
        remove_profile_file(&library, &state, "default", "rules/testing.md").expect("remove file");
        let source = library.join("profiles/default/rules/testing.md");
        fs::write(&source, "# Replacement\n").expect("recreate source");

        assert!(matches!(
            rollback_library_latest(&state),
            Err(ArmError::RollbackDrift(_))
        ));
        assert_eq!(fs::read_to_string(source).unwrap(), "# Replacement\n");
    }

    #[test]
    fn inactive_profile_delete_removes_owned_tree_and_rolls_back() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        create_profile(&library, &state, "work", "Work", "Work rules").expect("profile");
        add_profile_file(&library, &state, "work", "rules/testing.md").expect("add file");
        fs::write(library.join("profiles/work/AGENTS.md"), "# Work\n").expect("work rules");
        fs::write(
            library.join("profiles/work/rules/testing.md"),
            "# Testing\n",
        )
        .expect("testing rules");

        let plan = plan_delete_profile(&library, &state, "work").expect("delete plan");
        assert!(!plan.blocked);
        assert_eq!(plan.operation, "deleteProfile");
        assert_eq!(plan.change_count, 5);
        assert_eq!(
            plan.steps
                .iter()
                .map(|step| step.action.as_str())
                .collect::<Vec<_>>(),
            vec![
                "deleteProfileSource",
                "deleteProfileSource",
                "deleteProfileManifest",
                "deleteProfileDirectory",
                "deleteProfileDirectory",
            ]
        );

        let outcome = delete_profile(&library, &state, "work").expect("delete profile");
        assert_eq!(outcome.changed.len(), 5);
        assert!(!library.join("profiles/work").exists());
        assert!(inspect(&library, &state)
            .expect("inspect deleted")
            .profiles
            .iter()
            .all(|profile| profile.id != "work"));

        rollback_library_latest(&state).expect("rollback delete");
        assert_eq!(
            fs::read_to_string(library.join("profiles/work/AGENTS.md")).unwrap(),
            "# Work\n"
        );
        assert_eq!(
            fs::read_to_string(library.join("profiles/work/rules/testing.md")).unwrap(),
            "# Testing\n"
        );
        assert!(inspect(&library, &state)
            .expect("inspect restored")
            .profiles
            .iter()
            .any(|profile| profile.id == "work"));
    }

    #[test]
    fn profile_delete_blocks_active_selection_and_unmanaged_entries() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");

        let active_plan = plan_delete_profile(&library, &state, "default").expect("active plan");
        assert!(active_plan.blocked);
        assert!(active_plan.summary.contains("active on this machine"));
        assert!(matches!(
            delete_profile(&library, &state, "default"),
            Err(ArmError::Blocked(_))
        ));
        assert!(library.join("profiles/default/AGENTS.md").is_file());

        create_profile(&library, &state, "work", "Work", "").expect("profile");
        fs::write(library.join("profiles/work/notes.txt"), "keep me\n").expect("unmanaged");
        let unmanaged_plan = plan_delete_profile(&library, &state, "work").expect("unmanaged plan");
        assert!(unmanaged_plan.blocked);
        assert!(unmanaged_plan
            .summary
            .contains("unmanaged entry `notes.txt`"));
        assert!(matches!(
            delete_profile(&library, &state, "work"),
            Err(ArmError::Blocked(_))
        ));
        assert_eq!(
            fs::read_to_string(library.join("profiles/work/notes.txt")).unwrap(),
            "keep me\n"
        );
    }

    #[test]
    fn profile_delete_rollback_refuses_recreated_source_drift() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        create_profile(&library, &state, "work", "Work", "").expect("profile");
        delete_profile(&library, &state, "work").expect("delete profile");
        fs::create_dir_all(library.join("profiles/work")).expect("recreated profile directory");
        fs::write(library.join("profiles/work/AGENTS.md"), "# Replacement\n")
            .expect("recreated source");

        assert!(matches!(
            rollback_library_latest(&state),
            Err(ArmError::RollbackDrift(_))
        ));
        assert_eq!(
            fs::read_to_string(library.join("profiles/work/AGENTS.md")).unwrap(),
            "# Replacement\n"
        );
    }

    #[test]
    fn synced_sources_arrive_without_a_machine_profile_selection() {
        let (temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        let synced = temporary.path().join("synced-library");
        let synced_state = temporary.path().join("synced-state");
        fs::create_dir_all(synced.join("profiles/default")).expect("profile dir");
        for (source, target) in [
            (library.join("schema.json"), synced.join("schema.json")),
            (
                library.join("profiles/default/profile.json"),
                synced.join("profiles/default/profile.json"),
            ),
            (
                library.join("profiles/default/AGENTS.md"),
                synced.join("profiles/default/AGENTS.md"),
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
    fn v1_library_migrates_into_profile_owned_sources_and_rolls_back() {
        let (_temporary, library, state) = fixture();
        write_v1_library(&library, &state);

        let before = inspect(&library, &state).expect("inspect v1");
        assert_eq!(before.state, LibraryState::Upgrade);
        let plan = plan_initialize(&library, &state).expect("migration plan");
        assert!(!plan.blocked);
        assert_eq!(plan.operation, "upgradeLibrary");
        assert!(plan.steps.iter().any(|step| step.action == "upgradeSchema"));
        assert!(plan
            .steps
            .iter()
            .any(|step| step.action == "removeLegacyPack"));

        initialize(&library, &state).expect("migrate");
        let after = inspect(&library, &state).expect("inspect v2");
        assert_eq!(after.state, LibraryState::Ready);
        assert_eq!(after.runtime_state, RuntimeState::Current);
        assert_eq!(after.profiles[0].id, "default");
        assert_eq!(
            after.profiles[0]
                .files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "AGENTS.md",
                "rules/base/rules/review.md",
                "rules/work/AGENTS.md",
            ]
        );
        assert_eq!(
            fs::read_to_string(library.join("profiles/default/AGENTS.md")).unwrap(),
            "# Base\n"
        );
        assert_eq!(
            fs::read_to_string(library.join("profiles/default/rules/base/rules/review.md"))
                .unwrap(),
            "# Review\n"
        );
        assert_eq!(
            fs::read_to_string(library.join("profiles/default/rules/work/AGENTS.md")).unwrap(),
            "# Work\n"
        );
        assert!(!library.join("profiles/default.json").exists());
        assert!(!library.join("packs/base/pack.json").exists());

        rollback_library_latest(&state).expect("rollback activation");
        rollback_library_latest(&state).expect("rollback migration");
        let schema: SchemaDocument = read_json(&library.join(SCHEMA_FILE)).expect("v1 schema");
        assert_eq!(schema.schema_version, LEGACY_LIBRARY_SCHEMA_VERSION);
        assert_eq!(
            fs::read_to_string(library.join("packs/base/AGENTS.md")).unwrap(),
            "# Base\n"
        );
        assert!(library.join("profiles/default.json").is_file());
        assert!(!library.join("profiles/default/profile.json").exists());
        assert_eq!(
            inspect(&library, &state)
                .expect("inspect rolled-back v1")
                .state,
            LibraryState::Upgrade
        );
        assert!(
            !plan_initialize(&library, &state)
                .expect("replan rolled-back v1")
                .blocked
        );
    }

    #[test]
    fn v1_migration_blocks_unmanaged_pack_entries() {
        let (_temporary, library, state) = fixture();
        write_v1_library(&library, &state);
        fs::write(library.join("packs/base/notes.txt"), "keep me\n").expect("unmanaged file");

        let plan = plan_initialize(&library, &state).expect("migration plan");
        assert!(plan.blocked);
        assert!(plan.summary.contains("unmanaged entry"));
        assert!(matches!(
            initialize(&library, &state),
            Err(ArmError::Blocked(_))
        ));
        assert_eq!(
            fs::read_to_string(library.join("packs/base/notes.txt")).unwrap(),
            "keep me\n"
        );
    }

    #[test]
    fn v1_migration_blocks_unused_packs() {
        let (_temporary, library, state) = fixture();
        write_v1_library(&library, &state);
        let unused = LegacyPackDocument {
            schema_version: LEGACY_LIBRARY_SCHEMA_VERSION,
            id: "unused".into(),
            name: "Unused".into(),
            description: String::new(),
            instructions: vec!["AGENTS.md".into()],
        };
        write_json_fixture(&library.join("packs/unused/pack.json"), &unused);
        fs::write(library.join("packs/unused/AGENTS.md"), "# Keep\n").expect("unused rules");

        let plan = plan_initialize(&library, &state).expect("migration plan");
        assert!(plan.blocked);
        assert!(plan.summary.contains("not used by any Profile"));
        assert!(library.join("packs/unused/AGENTS.md").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn v1_migration_infers_machine_selection_from_managed_current() {
        let (_temporary, library, state) = fixture();
        write_v1_library(&library, &state);
        fs::remove_file(state.join(MACHINE_FILE)).expect("remove machine selection");
        let runtime = library.join(".runtime/default-old");
        fs::create_dir_all(&runtime).expect("legacy runtime");
        fs::write(
            runtime.join("manifest.json"),
            "{\"schemaVersion\":1,\"profileId\":\"default\"}\n",
        )
        .expect("legacy runtime manifest");
        std::os::unix::fs::symlink(".runtime/default-old", library.join(CURRENT_LINK))
            .expect("legacy current");

        let plan = plan_initialize(&library, &state).expect("migration plan");
        assert!(!plan.blocked);
        assert!(plan
            .steps
            .iter()
            .any(|step| step.action == "selectLocalProfile"));
        initialize(&library, &state).expect("migrate inferred selection");

        let snapshot = inspect(&library, &state).expect("inspect migrated library");
        assert_eq!(snapshot.active_profile_id.as_deref(), Some("default"));
        assert_eq!(snapshot.runtime_state, RuntimeState::Current);
    }

    #[test]
    fn v1_migration_without_machine_state_blocks_unmanaged_current() {
        let (_temporary, library, state) = fixture();
        write_v1_library(&library, &state);
        fs::remove_file(state.join(MACHINE_FILE)).expect("remove machine selection");
        fs::write(library.join(CURRENT_LINK), "do not replace\n").expect("foreign current");

        let plan = plan_initialize(&library, &state).expect("migration plan");
        assert!(plan.blocked);
        assert!(plan.summary.contains("unmanaged `current`"));
        assert_eq!(
            fs::read_to_string(library.join(CURRENT_LINK)).unwrap(),
            "do not replace\n"
        );
    }

    #[test]
    fn invalid_profile_path_cannot_escape_its_directory() {
        let (_temporary, library, state) = fixture();
        initialize(&library, &state).expect("initialize");
        let path = library.join("profiles/default/profile.json");
        let mut profile: ProfileDocument = read_json(&path).expect("profile");
        profile.instructions.push("../secret.md".into());
        write_atomic(&path, &(pretty_json(&profile).unwrap() + "\n")).expect("write");
        assert!(matches!(
            inspect(&library, &state),
            Err(ArmError::InvalidLibrary(_))
        ));
    }
}

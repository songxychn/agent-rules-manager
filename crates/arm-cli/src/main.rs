use arm_core::{
    default_library_root, default_state_root, ArmError, ConnectionChange, LibraryMutationOutcome,
    LibraryPlan, RulesManager,
};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "agent-rules",
    version,
    about = "Manage machine-local rule profiles and project them safely"
)]
struct Cli {
    /// Override the versioned rule library directory.
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// Override the machine-local state and backup directory.
    #[arg(long, global = true)]
    state_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Preview, initialize, or upgrade the Profile library.
    Init {
        /// Apply the previewed initialization plan.
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Inspect the library, active profile, runtime, and agent targets.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Manage syncable Profiles and the local active selection.
    Profiles {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    /// Preview exactly what agent projection would change.
    Plan {
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Preview making the selected agents independent instead of connecting them.
        #[arg(long)]
        disconnect: bool,
        #[arg(long)]
        json: bool,
    },
    /// Apply the current agent projection plan after refusing unmanaged conflicts.
    Apply {
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Materialize independent native files for the selected connected agents.
        #[arg(long)]
        disconnect: bool,
        #[arg(long)]
        json: bool,
    },
    /// Restore the latest agent projection if none of its targets drifted.
    Rollback {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    /// List every Profile in the library and the local active selection.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Preview or create a Profile with its own required AGENTS.md.
    Create {
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "")]
        description: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Preview or append a supplemental Markdown source to a Profile.
    AddFile {
        profile_id: String,
        relative_path: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Preview or activate a Profile only on this machine.
    Activate {
        id: String,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        json: bool,
    },
    /// Restore the latest library mutation or local Profile switch.
    Rollback {
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), ArmError> {
    let cli = Cli::parse();
    let root = cli.root.map(Ok).unwrap_or_else(default_library_root)?;
    let state_root = cli.state_root.map(Ok).unwrap_or_else(default_state_root)?;
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| ArmError::MissingSource("HOME is not set".into()))?;
    let manager = RulesManager::new(root, state_root, &home);

    match cli.command {
        Command::Init { apply, json } => {
            if apply {
                print_mutation(manager.initialize()?, json)?;
            } else {
                print_library_plan(manager.plan_initialize()?, json)?;
            }
        }
        Command::Status { json } => {
            let snapshot = manager.snapshot()?;
            if json {
                print_json(&snapshot)?;
            } else {
                println!(
                    "Library: {} ({:?})",
                    snapshot.library_root, snapshot.library_state
                );
                println!("Runtime: {:?}", snapshot.runtime_state);
                println!(
                    "Active profile: {}",
                    snapshot
                        .active_profile_id
                        .as_deref()
                        .unwrap_or("not selected")
                );
                println!("Entrypoint: {}", snapshot.source_path);
                println!("Profiles: {}", snapshot.profiles.len());
                for agent in snapshot.agents {
                    println!(
                        "{:<12} {:<12} {:<12} {}",
                        agent.id,
                        if agent.installed {
                            "installed"
                        } else {
                            "not-detected"
                        },
                        if agent.connected {
                            "connected"
                        } else {
                            "independent"
                        },
                        agent.target_path
                    );
                }
            }
        }
        Command::Profiles { command } => match command {
            ProfileCommand::List { json } => {
                let profiles = manager.snapshot()?.profiles;
                if json {
                    print_json(&profiles)?;
                } else {
                    for profile in profiles {
                        println!(
                            "{}{}  {}  {} file(s)",
                            if profile.is_active { "* " } else { "  " },
                            profile.id,
                            profile.name,
                            profile.files.len()
                        );
                    }
                }
            }
            ProfileCommand::Create {
                id,
                name,
                description,
                apply,
                json,
            } => {
                if apply {
                    print_mutation(manager.create_profile(&id, &name, &description)?, json)?;
                } else {
                    print_library_plan(
                        manager.plan_create_profile(&id, &name, &description)?,
                        json,
                    )?;
                }
            }
            ProfileCommand::AddFile {
                profile_id,
                relative_path,
                apply,
                json,
            } => {
                if apply {
                    print_mutation(manager.add_profile_file(&profile_id, &relative_path)?, json)?;
                } else {
                    print_library_plan(
                        manager.plan_add_profile_file(&profile_id, &relative_path)?,
                        json,
                    )?;
                }
            }
            ProfileCommand::Activate { id, apply, json } => {
                if apply {
                    print_mutation(manager.activate_profile(&id)?, json)?;
                } else {
                    print_library_plan(manager.plan_activate_profile(&id)?, json)?;
                }
            }
            ProfileCommand::Rollback { json } => {
                let outcome = manager.rollback_library_latest()?;
                if json {
                    print_json(&outcome)?;
                } else {
                    println!("Restored library snapshot {}.", outcome.backup_id);
                    for target in outcome.restored {
                        println!("  {target}");
                    }
                }
            }
        },
        Command::Plan {
            agents,
            disconnect,
            json,
        } => {
            let plan = if disconnect {
                manager.plan_connections(&connection_changes(&agents, false))?
            } else {
                manager.plan(&agents)?
            };
            if json {
                print_json(&plan)?;
            } else {
                for step in &plan.steps {
                    println!(
                        "{:<12} {:<20} {}",
                        step.agent_id, step.action, step.target_path
                    );
                }
                println!(
                    "{} change(s){}",
                    plan.change_count,
                    if plan.blocked { ", blocked" } else { "" }
                );
            }
        }
        Command::Apply {
            agents,
            disconnect,
            json,
        } => {
            let outcome = if disconnect {
                manager.apply_connections(&connection_changes(&agents, false))?
            } else {
                manager.apply(&agents)?
            };
            if json {
                print_json(&outcome)?;
            } else if outcome.changed.is_empty() {
                println!("No connection changes were needed.");
            } else {
                println!("Applied to {}.", outcome.changed.join(", "));
                if let Some(backup) = outcome.backup_id {
                    println!("Rollback snapshot: {backup}");
                }
            }
        }
        Command::Rollback { json } => {
            let outcome = manager.rollback_latest()?;
            if json {
                print_json(&outcome)?;
            } else {
                println!("Restored projection snapshot {}.", outcome.backup_id);
                for target in outcome.restored {
                    println!("  {target}");
                }
            }
        }
    }
    Ok(())
}

fn connection_changes(agents: &[String], connected: bool) -> Vec<ConnectionChange> {
    agents
        .iter()
        .map(|agent_id| ConnectionChange {
            agent_id: agent_id.clone(),
            connected,
        })
        .collect()
}

fn print_library_plan(plan: LibraryPlan, json: bool) -> Result<(), ArmError> {
    if json {
        print_json(&plan)
    } else {
        println!("{}", plan.summary);
        for step in plan.steps {
            println!("  {:<20} {}", step.action, step.path);
            println!("    {}", step.summary);
        }
        println!(
            "{} change(s){}; rerun with --apply to confirm.",
            plan.change_count,
            if plan.blocked { ", blocked" } else { "" }
        );
        Ok(())
    }
}

fn print_mutation(outcome: LibraryMutationOutcome, json: bool) -> Result<(), ArmError> {
    if json {
        print_json(&outcome)
    } else if outcome.changed.is_empty() {
        println!("Already current.");
        Ok(())
    } else {
        println!("Changed {} path(s).", outcome.changed.len());
        for path in outcome.changed {
            println!("  {path}");
        }
        if let Some(backup) = outcome.backup_id {
            println!("Library rollback snapshot: {backup}");
        }
        Ok(())
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<(), ArmError> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

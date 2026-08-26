use arm_core::{
    default_library_root, default_state_root, ArmError, ConnectionChange, RulesManager,
};
use clap::{Parser, Subcommand};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "agent-rules",
    version,
    about = "Safely project one neutral rules source into multiple coding agents"
)]
struct Cli {
    /// Override the canonical rules library directory.
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    /// Override the local state and backup directory.
    #[arg(long, global = true)]
    state_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create the canonical AGENTS.md when it is missing.
    Init,
    /// Inspect every supported target without changing files.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Preview exactly what apply would change.
    Plan {
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Preview making the selected agents independent instead of connecting them.
        #[arg(long)]
        disconnect: bool,
        #[arg(long)]
        json: bool,
    },
    /// Apply the current plan after refusing unmanaged conflicts.
    Apply {
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Materialize independent native files for the selected connected agents.
        #[arg(long)]
        disconnect: bool,
        #[arg(long)]
        json: bool,
    },
    /// Restore the latest apply if none of its targets drifted afterward.
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
        Command::Init => {
            let path = manager.initialize()?;
            println!("Initialized {}", path.display());
        }
        Command::Status { json } => {
            let snapshot = manager.snapshot()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                println!("Source: {}", snapshot.source_path);
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
                println!("{}", serde_json::to_string_pretty(&plan)?);
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
                println!("{}", serde_json::to_string_pretty(&outcome)?);
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
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                println!("Restored snapshot {}.", outcome.backup_id);
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

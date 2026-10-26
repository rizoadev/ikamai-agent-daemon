//! IKAMAI Agent Daemon — main binary.
//!
//! CLI entry point. Dispatches to daemon commands.

use anyhow::Context;
use clap::{Parser, Subcommand};
use iad_core::{
    config::{DaemonConfig, DEFAULT_CONFIG_TOML},
    pid::{acquire_pid_file, pid_alive, read_pid, remove_pid_file},
    process::{ProcessEvent, ProcessManager},
    registry::AgentRegistry,
    AgentManifest, AgentStatus,
};
use iad_storage::Storage;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc;
use tracing::{info, warn};

// ─── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "iad")]
#[command(about = "IKAMAI Agent Daemon — orchestrate your AI agent fleet")]
#[command(version)]
struct Cli {
    /// Config file path (default: ~/.iad/iad.toml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize IAD home directory
    Init,
    /// Start the daemon
    Start {
        /// Run in foreground (don't daemonize)
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status,
    /// Reload configuration
    Reload,
    /// Agent management
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
    /// Task management
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    /// Register an agent from manifest
    Add { manifest: String },
    /// Remove an agent
    Remove { name: String },
    /// List all agents
    List,
    /// Start an agent
    Start { name: String },
    /// Stop an agent
    Stop { name: String },
    /// Restart an agent
    Restart { name: String },
    /// View agent logs
    Logs { name: String },
    /// Show agent details
    Status { name: String },
}

#[derive(Subcommand)]
enum TaskCommands {
    /// Submit a task
    Submit { payload: String },
    /// List tasks
    List,
    /// Show task status
    Status { id: String },
    /// Cancel a task
    Cancel { id: String },
}

// ─── Main ────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load config.
    let config_path = cli.config.unwrap_or_else(DaemonConfig::default_path);
    let cfg = DaemonConfig::load(&config_path)
        .unwrap_or_else(|_| DaemonConfig::default());

    match cli.command {
        Commands::Init => cmd_init(&cfg).await?,
        Commands::Start { foreground } => cmd_start(&cfg, foreground).await?,
        Commands::Stop => cmd_stop(&cfg).await?,
        Commands::Status => cmd_status(&cfg).await?,
        Commands::Reload => cmd_reload(&cfg).await?,
        Commands::Agent { command } => cmd_agent(&cfg, command).await?,
        Commands::Task { command } => cmd_task(&cfg, command).await?,
    }

    Ok(())
}

// ─── Commands ────────────────────────────────────────────────────────────────

async fn cmd_init(_cfg: &DaemonConfig) -> anyhow::Result<()> {
    let home = home_dir();
    let dirs = [
        format!("{home}/.iad"),
        format!("{home}/.iad/agents"),
        format!("{home}/.iad/data"),
        format!("{home}/.iad/logs"),
        format!("{home}/.iad/run"),
    ];
    for d in &dirs {
        std::fs::create_dir_all(d)?;
        println!("   ✅ {d}");
    }
    let toml_path = format!("{home}/.iad/iad.toml");
    if !std::path::Path::new(&toml_path).exists() {
        std::fs::write(&toml_path, DEFAULT_CONFIG_TOML)?;
        println!("   ✅ {toml_path}");
    } else {
        println!("   ⚠️  {toml_path} already exists, skipping");
    }
    println!("✅ IAD initialized. Run `iad start --foreground` to begin.");
    Ok(())
}

async fn cmd_start(cfg: &DaemonConfig, foreground: bool) -> anyhow::Result<()> {
    // Expand ~ in paths.
    let pid_path = expand_tilde(&cfg.daemon.pid_file);

    // Try to acquire PID file.
    let _pid_file = acquire_pid_file(std::path::Path::new(&pid_path))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Initialize tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&cfg.daemon.log_level)),
        )
        .json()
        .init();

    info!(pid = std::process::id(), "IAD starting");

    // Open storage.
    let db_path = expand_tilde(&cfg.storage.path);
    let storage = Storage::open(std::path::Path::new(&db_path))
        .context("failed to open storage")?;

    // Build registry and load persisted agents.
    let registry = Arc::new(AgentRegistry::new());
    let persisted = storage.list_agents().context("failed to list agents")?;
    let count = persisted.len();
    registry.load_all(persisted).await;
    info!(count, "agents loaded from storage");

    // Build process manager.
    let (event_tx, mut event_rx) = mpsc::channel::<ProcessEvent>(256);
    let pm = Arc::new(ProcessManager::new(event_tx));

    // Recover previously-running agents (crash recovery, task 1.R4).
    let agents = registry.list().await;
    for agent in &agents {
        if agent.status == AgentStatus::Running || agent.status == AgentStatus::Starting {
            warn!(agent = agent.name, "recovering agent after daemon restart");
            // Mark as stopped — health monitor will restart via policy.
            let _ = registry.update(&agent.name, |a| {
                a.status = AgentStatus::Stopped;
            }).await;
        }
    }

    // Health monitor.
    let health = Arc::new(iad_core::health::HealthMonitor::new(
        registry.clone(),
        pm.clone(),
        cfg.clone(),
    ));
    tokio::spawn(health.run());

    // Signal handler.
    let pid_path_sig = pid_path.clone();
    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint  = signal(SignalKind::interrupt()).unwrap();
        tokio::select! {
            _ = sigterm.recv() => info!("SIGTERM received, shutting down"),
            _ = sigint.recv()  => info!("SIGINT received, shutting down"),
        }
        remove_pid_file(std::path::Path::new(&pid_path_sig));
        std::process::exit(0);
    });

    println!("🤖 IKAMAI Agent Daemon started (pid={})", std::process::id());
    if foreground {
        println!("   Running in foreground. Ctrl+C to stop.");
    }

    // Event loop — process agent lifecycle events.
    loop {
        while let Some(event) = event_rx.recv().await {
            match event {
                ProcessEvent::Started { agent_name, pid, generation_id } => {
                    let _ = registry.update(&agent_name, |a| {
                        a.status        = AgentStatus::Running;
                        a.pid           = Some(pid);
                        a.generation_id = generation_id;
                        a.started_at    = Some(chrono::Utc::now());
                    }).await;
                    if let Ok(agent) = registry.get(&agent_name).await {
                        let _ = storage.upsert_agent(&agent);
                    }
                    info!(agent = agent_name, pid, "agent is running");
                }
                ProcessEvent::Exited { agent_name, generation_id, exit_code, signal } => {
                    let crashed = signal.is_some() || exit_code.map(|c| c != 0).unwrap_or(true);
                    let new_status = if crashed { AgentStatus::Crashed } else { AgentStatus::Stopped };

                    let _ = registry.update(&agent_name, |a| {
                        // Only update if generation matches (avoid stale events, plan D11).
                        if a.generation_id == generation_id {
                            a.status     = new_status.clone();
                            a.pid        = None;
                            a.stopped_at = Some(chrono::Utc::now());
                            if crashed { a.failure_count += 1; }
                        }
                    }).await;

                    if let Ok(agent) = registry.get(&agent_name).await {
                        if crashed {
                            let inc = iad_core::process::make_incident(
                                &agent, exit_code, signal, "agent exited unexpectedly",
                            );
                            let _ = storage.insert_incident(&inc);
                        }
                        let _ = storage.upsert_agent(&agent);
                    }
                }
            }
        }
    }
}

async fn cmd_stop(cfg: &DaemonConfig) -> anyhow::Result<()> {
    let pid_path = expand_tilde(&cfg.daemon.pid_file);
    match read_pid(std::path::Path::new(&pid_path)) {
        Some(pid) if pid_alive(pid) => {
            println!("🛑 Stopping IAD (pid={pid})...");
            unsafe { libc::kill(pid as i32, libc::SIGTERM) };
            println!("✅ SIGTERM sent.");
        }
        _ => {
            println!("⚠️  IAD is not running.");
        }
    }
    Ok(())
}

async fn cmd_status(cfg: &DaemonConfig) -> anyhow::Result<()> {
    let pid_path = expand_tilde(&cfg.daemon.pid_file);
    match read_pid(std::path::Path::new(&pid_path)) {
        Some(pid) if pid_alive(pid) => {
            println!("📊 IKAMAI Agent Daemon");
            println!("   Status : ✅ running");
            println!("   PID    : {pid}");
            println!("   API    : http://{}", cfg.api.rest_bind);
        }
        _ => {
            println!("📊 IKAMAI Agent Daemon");
            println!("   Status : ❌ not running");
        }
    }
    Ok(())
}

async fn cmd_reload(cfg: &DaemonConfig) -> anyhow::Result<()> {
    let pid_path = expand_tilde(&cfg.daemon.pid_file);
    match read_pid(std::path::Path::new(&pid_path)) {
        Some(pid) if pid_alive(pid) => {
            unsafe { libc::kill(pid as i32, libc::SIGHUP) };
            println!("🔄 SIGHUP sent to IAD (pid={pid}).");
        }
        _ => println!("⚠️  IAD is not running."),
    }
    Ok(())
}

async fn cmd_agent(cfg: &DaemonConfig, command: AgentCommands) -> anyhow::Result<()> {
    let db_path = expand_tilde(&cfg.storage.path);
    let storage = Storage::open(std::path::Path::new(&db_path))
        .context("failed to open storage")?;

    match command {
        AgentCommands::Add { manifest } => {
            let raw = std::fs::read_to_string(&manifest)
                .with_context(|| format!("cannot read {manifest}"))?;
            let m: AgentManifest = serde_yaml::from_str(&raw)
                .context("invalid manifest YAML")?;
            // Validate required fields.
            if m.name.is_empty() {
                anyhow::bail!("manifest: `name` is required");
            }
            // Create agent record.
            let now = chrono::Utc::now();
            let agent = iad_core::Agent {
                id:            ulid::Ulid::new().to_string(),
                name:          m.name.clone(),
                version:       m.version.clone(),
                status:        AgentStatus::Registered,
                pid:           None,
                manifest:      m,
                generation_id: 0,
                failure_count: 0,
                started_at:    None,
                stopped_at:    None,
                created_at:    now,
                updated_at:    now,
            };
            storage.upsert_agent(&agent)?;
            println!("➕ Agent '{}' registered (id={})", agent.name, agent.id);
        }
        AgentCommands::Remove { name } => {
            if storage.delete_agent_by_name(&name)? {
                println!("➖ Agent '{name}' removed.");
            } else {
                println!("⚠️  Agent '{name}' not found.");
            }
        }
        AgentCommands::List => {
            let agents = storage.list_agents()?;
            if agents.is_empty() {
                println!("📋 No agents registered.");
            } else {
                println!("{:<24} {:<12} {:<10} {}", "NAME", "STATUS", "PID", "VERSION");
                println!("{}", "─".repeat(60));
                for a in agents {
                    let pid = a.pid.map(|p| p.to_string()).unwrap_or_else(|| "—".into());
                    println!("{:<24} {:<12} {:<10} {}", a.name, a.status, pid, a.version);
                }
            }
        }
        AgentCommands::Start { name } => {
            let agent = storage.get_agent_by_name(&name)?
                .ok_or_else(|| anyhow::anyhow!("agent '{name}' not found"))?;
            println!("▶️  Agent '{}' queued for start. (daemon will spawn it)", agent.name);
            // In standalone mode, update status to trigger health monitor.
            let mut a = agent;
            a.status = AgentStatus::Registered;
            storage.upsert_agent(&a)?;
        }
        AgentCommands::Stop { name } => {
            println!("⏹️  Send stop signal to agent '{name}' via daemon.");
            // TODO: IPC to running daemon (Phase 2).
        }
        AgentCommands::Restart { name } => {
            println!("🔄 Restart agent '{name}' via daemon.");
            // TODO: IPC (Phase 2).
        }
        AgentCommands::Logs { name } => {
            let incidents = storage.list_incidents_for_agent(&name)?;
            if incidents.is_empty() {
                println!("📜 No incidents for agent '{name}'.");
            } else {
                println!("📜 Incidents for agent '{name}':");
                for inc in incidents {
                    println!(
                        "  [{}] gen={} exit={:?} sig={:?} — {}",
                        inc.occurred_at.format("%Y-%m-%d %H:%M:%S"),
                        inc.generation_id,
                        inc.exit_code,
                        inc.signal,
                        inc.reason
                    );
                }
            }
        }
        AgentCommands::Status { name } => {
            match storage.get_agent_by_name(&name)? {
                Some(a) => {
                    println!("📊 Agent '{}'", a.name);
                    println!("   ID           : {}", a.id);
                    println!("   Version      : {}", a.version);
                    println!("   Status       : {}", a.status);
                    println!("   PID          : {}", a.pid.map(|p| p.to_string()).unwrap_or("—".into()));
                    println!("   Generation   : {}", a.generation_id);
                    println!("   Failures     : {}", a.failure_count);
                    println!("   Runtime      : {}", a.manifest.runtime);
                    println!("   Entrypoint   : {}", a.manifest.entrypoint);
                    println!("   Capabilities : {}", a.manifest.capabilities.join(", "));
                    println!("   Restart      : {:?}", a.manifest.restart);
                    println!("   Created      : {}", a.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                }
                None => println!("⚠️  Agent '{name}' not found."),
            }
        }
    }
    Ok(())
}

async fn cmd_task(_cfg: &DaemonConfig, command: TaskCommands) -> anyhow::Result<()> {
    // Phase 2 — basic stubs for now.
    match command {
        TaskCommands::Submit { payload } => {
            println!("📤 Task submitted: {payload}");
            println!("   (Task routing requires Phase 2 — coming soon)");
        }
        TaskCommands::List => {
            println!("📋 Tasks: (none — Phase 2 required)");
        }
        TaskCommands::Status { id } => {
            println!("📊 Task '{id}': (Phase 2 required)");
        }
        TaskCommands::Cancel { id } => {
            println!("❌ Task '{id}' cancel: (Phase 2 required)");
        }
    }
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        format!("{}/{}", home_dir(), &path[2..])
    } else {
        path.into()
    }
}

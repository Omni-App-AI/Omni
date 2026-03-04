mod commands;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "omni", version, about = "Omni AI Agent Platform")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Log level override
    #[arg(long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Omni desktop application
    Start,

    /// Run in headless CLI mode (no UI)
    Chat {
        /// Session ID to continue (or new session if omitted)
        #[arg(short, long)]
        session: Option<String>,
    },

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Manage extensions
    Ext {
        #[command(subcommand)]
        action: ExtAction,
    },

    /// Show system status
    Status,

    /// Interactive onboarding wizard
    Onboard,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Open config file in editor
    Edit,
    /// Validate config file
    Validate,
    /// Reset to defaults
    Reset,
}

#[derive(Subcommand)]
enum ExtAction {
    /// List installed extensions
    List,
    /// Install an extension
    Install { source: String },
    /// Remove an extension
    Remove { id: String },
    /// Initialize a new extension project
    Init { name: String },
    /// Build extension for distribution
    Build,
    /// Run extension in dev mode
    Dev,
    /// Publish extension to registry
    Publish,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // 1. Resolve paths
    let paths = omni_core::paths::OmniPaths::resolve()?;
    paths.ensure_dirs_exist()?;

    // 2. Determine config path
    let config_path = cli.config.unwrap_or_else(|| paths.config_file());

    // 3. Load config
    let config = omni_core::config::OmniConfig::load(&config_path)?;

    // 4. Init logging (use CLI override if provided, otherwise config value)
    let log_level = if cli.log_level != "info" {
        &cli.log_level
    } else {
        &config.general.log_level
    };
    omni_core::logging::init_logging(log_level, Some(&paths.log_file()))?;

    // 5. Dispatch command
    match cli.command {
        Commands::Config { action } => {
            commands::config_cmd::run(action, &config, &config_path)?;
        }
        Commands::Status => {
            commands::status_cmd::run(&config, &paths)?;
        }
        Commands::Start => {
            tracing::info!("Starting Omni runtime...");

            // Get or create encryption key
            let db_key = omni_core::database::get_or_create_encryption_key()?;

            // Open database
            let _db = omni_core::database::Database::open(&paths.database_file(), &db_key)?;
            tracing::info!("Database opened at {}", paths.database_file().display());

            // Create event bus
            let event_bus = omni_core::events::EventBus::new(1024);

            // Start config watcher
            let config_watcher =
                omni_core::config::ConfigWatcher::start(config_path.clone(), config.clone())?;

            // Emit ConfigChanged on watcher updates
            let bus_clone = event_bus.clone();
            let mut rx = config_watcher.receiver.clone();
            tokio::spawn(async move {
                while rx.changed().await.is_ok() {
                    tracing::info!("Configuration reloaded");
                    bus_clone.emit(omni_core::events::OmniEvent::ConfigChanged);
                }
            });

            println!("Omni runtime started. Press Ctrl+C to stop.");
            tokio::signal::ctrl_c().await?;
            tracing::info!("Shutting down");
            println!("Shutting down.");
        }
        Commands::Chat { session } => {
            // Get or create encryption key
            let db_key = omni_core::database::get_or_create_encryption_key()?;

            // Open database
            let db = omni_core::database::Database::open(&paths.database_file(), &db_key)?;
            let db = std::sync::Arc::new(std::sync::Mutex::new(db));

            // Create event bus
            let event_bus = omni_core::events::EventBus::new(1024);

            // Extensions directory
            let extensions_dir = paths.extensions_dir();

            commands::chat_cmd::run(session, &config, db, event_bus, extensions_dir).await?;
        }
        Commands::Ext { .. } => {
            println!("Extension management is not yet available.");
        }
        Commands::Onboard => {
            println!("Onboarding wizard is not yet available.");
        }
    }

    Ok(())
}

//! Turnkey CLI application.
//!
//! Command-line interface for the turnkey emulator with TUI support.

use tracing::info;
use turnkey_cli::client_tui;

mod tui;

use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use turnkey_core::{AccessDirection, DeviceId};
use turnkey_emulator::{EmulatorConfig, NetworkConfig, OperationMode, TurnstileEmulator};
use turnkey_hardware::manager::{PeripheralConfig, PeripheralManager};
use turnkey_network::{TcpClient, TcpClientConfig};
use turnkey_storage::{
    Database, DatabaseConfig, OfflineValidator, OnlineValidator, OnlineValidatorConfig, Validator,
};

use crate::tui::run_tui;

/// Turnkey CLI - Access control emulator
#[derive(Parser, Debug)]
#[command(name = "turnkey")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,

    /// Enable trace logging
    #[arg(short, long, global = true)]
    trace: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the turnstile emulator with TUI
    Run {
        /// Device ID (1-99)
        #[arg(short = 'i', long, default_value = "1")]
        device_id: u8,

        /// Operation mode
        #[arg(short, long, value_enum, default_value = "online")]
        mode: CliMode,

        /// Server IP address (for online mode)
        #[arg(short = 's', long, default_value = "127.0.0.1")]
        server_ip: String,

        /// Server port (for online mode)
        #[arg(short = 'p', long, default_value = "3000")]
        server_port: u16,

        /// Database path (for offline mode)
        #[arg(long, default_value = "turnkey.db")]
        database: PathBuf,

        /// Configuration file
        #[arg(short = 'c', long)]
        config: Option<PathBuf>,
    },

    /// Run the client-emulator (validation server) with TUI
    Server {
        /// Bind IP address
        #[arg(short, long, default_value = "0.0.0.0")]
        bind_ip: String,

        /// Bind port
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum CliMode {
    Online,
    Offline,
}

impl From<CliMode> for OperationMode {
    fn from(mode: CliMode) -> Self {
        match mode {
            CliMode::Online => OperationMode::Online,
            CliMode::Offline => OperationMode::Offline,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(cli.debug, cli.trace);

    match cli.command {
        Some(Commands::Run {
            device_id,
            mode,
            server_ip,
            server_port,
            database,
            config,
        }) => {
            run_emulator(device_id, mode, server_ip, server_port, database, config).await?;
        }
        Some(Commands::Server { bind_ip, port }) => {
            run_server(bind_ip, port).await?;
        }
        None => {
            run_emulator(
                1,
                CliMode::Online,
                "127.0.0.1".to_string(),
                3000,
                PathBuf::from("turnkey.db"),
                None,
            )
            .await?;
        }
    }

    Ok(())
}

/// Setup logging based on CLI flags.
fn setup_logging(debug: bool, trace: bool) {
    let filter = if trace {
        "trace"
    } else if debug {
        "debug"
    } else {
        "info"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Run the emulator with TUI.
async fn run_emulator(
    device_id: u8,
    mode: CliMode,
    server_ip: String,
    server_port: u16,
    database: PathBuf,
    config_path: Option<PathBuf>,
) -> Result<()> {
    let config = if let Some(path) = config_path {
        EmulatorConfig::from_file(path).context("Failed to load configuration file")?
    } else {
        let operation_mode = mode.into();
        let network = if matches!(operation_mode, OperationMode::Online) {
            let ip: IpAddr = server_ip.parse().context("Invalid server IP address")?;
            Some(NetworkConfig::new(ip, server_port))
        } else {
            None
        };

        EmulatorConfig {
            device_id,
            mode: operation_mode,
            validation_timeout_ms: 3000,
            rotation_timeout_secs: 10,
            default_display_message: "DIGITE SEU CODIGO".to_string(),
            default_direction: AccessDirection::Entry,
            idle_timeout_secs: 30,
            rotation_duration_secs: 2,
            network,
        }
    };

    let device_id = DeviceId::new(config.device_id).context("Invalid device ID")?;

    let peripheral_config = PeripheralConfig::default();
    let peripheral_manager = PeripheralManager::new(peripheral_config);
    let peripheral_handle = peripheral_manager.start();

    let validator = match config.mode {
        OperationMode::Online => {
            let network = config
                .network
                .as_ref()
                .context("Network configuration required for online mode")?;

            let server_addr_str = format!("{}:{}", network.ip_address, network.port);
            let server_addr = server_addr_str.parse().context("Invalid server address")?;

            let tcp_config = TcpClientConfig {
                server_addr,
                timeout: std::time::Duration::from_millis(config.validation_timeout_ms),
            };

            let mut tcp_client = TcpClient::new(tcp_config);

            tcp_client
                .connect()
                .await
                .context("Failed to connect to server")?;

            let online_config = OnlineValidatorConfig::default();

            let online_validator = OnlineValidator::new(tcp_client, device_id, online_config);

            Validator::Online(Box::new(online_validator))
        }
        OperationMode::Offline => {
            let db_config =
                DatabaseConfig::new(database.to_string_lossy().to_string()).auto_migrate(true);

            let db = Database::new(db_config)
                .await
                .context("Failed to open database")?;

            let offline_validator = OfflineValidator::new(db.pool().clone());

            Validator::Offline(offline_validator)
        }
    };

    let emulator = TurnstileEmulator::new(peripheral_handle, validator, config, device_id)
        .context("Failed to create emulator")?;

    let (handle, task) = emulator.spawn();

    let tui_handle = tokio::spawn(async move {
        if let Err(e) = run_tui(handle).await {
            tracing::error!(error = %e, "TUI error");
        }
    });

    tokio::select! {
        result = task => {
            result.context("Emulator task panicked")??;
        }
        result = tui_handle => {
            result.context("TUI task panicked")?;
        }
    }

    Ok(())
}

/// Run the client-emulator server with TUI.
async fn run_server(bind_ip: String, port: u16) -> Result<()> {
    let addr_str = format!("{}:{}", bind_ip, port);
    let bind_addr = addr_str.parse().context("Invalid bind address")?;

    info!("Starting client-emulator server on {}", bind_addr);

    client_tui::run_client_tui(bind_addr).await?;

    Ok(())
}

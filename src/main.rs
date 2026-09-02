//! The `draughts` binary.
//!
//! One process. The startup order in [`run`] is chosen so that failures happen
//! before the system accepts traffic (§22.3), and the shutdown order in
//! [`shutdown`] so that a `SIGTERM` loses nothing (§22.4).

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;

use draughts::api::state::FaceStatus;
use draughts::api::{self, AppState};
use draughts::config::{Config, validate};
use draughts::db;
use draughts::engine::TranspositionTable;
use draughts::face::{CircuitBreaker, DeviceRequest, SystemClock, select_device};
use draughts::telemetry;

#[derive(Parser, Debug)]
#[command(
    name = "draughts",
    version,
    about = "A draughts engine and self-play training lab"
)]
struct Cli {
    /// Path to the configuration file.
    #[arg(long, short, default_value = "draughts.toml", env = "DRAUGHTS_CONFIG")]
    config: PathBuf,

    /// Validate the configuration against this host and exit.
    ///
    /// Runs every check from §23.1 without opening the database, allocating the
    /// table, or binding a port — which makes it usable from CI and from a
    /// pre-deploy hook.
    #[arg(long)]
    check_config: bool,

    /// Emit logs as JSON.
    #[arg(long, env = "DRAUGHTS_LOG_JSON")]
    log_json: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    telemetry::init(cli.log_json);

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error = ?error, "startup failed");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    // 1 · Parse and validate configuration, including the memory budget (§23).
    let config =
        Config::load(&cli.config).with_context(|| format!("loading {}", cli.config.display()))?;

    // The device is resolved before validation because which profile must meet
    // its deadline depends on it: the active one failing refuses startup, the
    // inactive one failing warns (§23.1).
    let request = DeviceRequest::from_config(config.face.device, config.face.device_index);
    let (device, device_kind) = select_device(request);

    let mut report = validate::validate(&config, device_kind);
    validate::check_against_host(&config, &mut report);

    for warning in &report.warnings {
        tracing::warn!("{warning}");
    }

    if !report.is_ok() {
        for error in &report.errors {
            tracing::error!("configuration is invalid: {error}");
        }
        anyhow::bail!(
            "{} configuration error(s); refusing to start",
            report.errors.len()
        );
    }

    tracing::info!(
        projected_gb = report.projected_memory_bytes() as f64 / (1024.0 * 1024.0 * 1024.0),
        budget_gb = config.limits.max_total_memory_gb,
        device = %device_kind.as_health_string(),
        profile = device_kind.profile_name(),
        "configuration validated"
    );

    if cli.check_config {
        println!("configuration is valid");
        for (key, bytes) in &report.memory_breakdown {
            println!(
                "  {key}: {:.2} GB",
                *bytes as f64 / (1024.0 * 1024.0 * 1024.0)
            );
        }
        return Ok(());
    }

    let config = Arc::new(config);

    // 2 · Open the writer connection, apply pragmas, run migrations inside one
    //     transaction.
    let mut writer_conn =
        db::pool::open_writer(&config.database).context("opening the write connection")?;
    let schema_version = db::migrations::run(&mut writer_conn).context("applying migrations")?;
    tracing::info!(schema_version, "database ready");

    // 6 · Allocate the transposition table. A failure here is fatal: the
    //     allocation is large and predictable, and failing at boot beats failing
    //     at hour four.
    let tt = if config.engine.transposition.enabled {
        TranspositionTable::with_capacity(
            config.engine.transposition.capacity_entries,
            config.engine.transposition.shard_count,
        )
    } else {
        TranspositionTable::disabled()
    };

    let breaker = CircuitBreaker::new(&config.face.circuit_breaker);
    let clock = SystemClock::new();

    // 8 · Resolve the device (done above) and load the matching profile's model
    //     if `warm_on_start`; on failure — including a CUDA OOM — open the
    //     circuit permanently and continue (§17.2).
    let face_status = FaceStatus::unloaded_cpu(&config);
    let _ = device;

    let state = AppState::new(
        Arc::clone(&config),
        tt,
        breaker,
        clock,
        face_status,
        schema_version,
    );

    // 9 · Bind the HTTP listener.
    serve(config, state)
}

fn serve(config: Arc<Config>, state: AppState) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        // HTTP only. The engine does not run here (§15.4).
        .worker_threads(config.server.worker_threads)
        .enable_all()
        .build()
        .context("building the Tokio runtime")?;

    runtime.block_on(async move {
        let address = format!("{}:{}", config.server.host, config.server.port);
        let listener = tokio::net::TcpListener::bind(&address)
            .await
            .with_context(|| format!("binding {address}"))?;

        tracing::info!(%address, "serving");

        axum::serve(listener, api::router(state))
            .with_graceful_shutdown(shutdown())
            .await
            .context("serving")
    })
}

/// §22.4. A `SIGKILL` skips all of this and §11.4 defines exactly what is lost.
/// A `SIGTERM` must not.
async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("installing the Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("installing the SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("shutting down: draining in-flight requests");
}

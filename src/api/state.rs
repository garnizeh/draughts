//! Shared application state — handles, never ownership (§5.2).

use std::sync::Arc;

use crate::config::Config;
use crate::engine::TranspositionTable;
use crate::face::{CircuitBreaker, DeviceKind, DeviceRequest, MonotonicClock, SystemClock};

/// What the Face layer resolved at startup, reported verbatim on `/health`.
///
/// Kept as a snapshot rather than re-derived per request: the device is
/// resolved exactly once (§7.4.1), and a `/health` field that could disagree
/// with the running configuration would be worse than no field at all.
#[derive(Clone, Debug)]
pub struct FaceStatus {
    pub enabled: bool,
    pub provider: &'static str,
    pub device_requested: DeviceRequest,
    pub device: DeviceKind,
    pub device_name: Option<String>,
    pub model_id: String,
    pub model_loaded: bool,
    pub resident_mb: u64,
    pub vram_used_mb: u64,
    /// `None` on the CPU path (§9.2).
    pub vram_budget_mb: Option<u64>,
}

impl FaceStatus {
    /// The state of a process that has loaded nothing yet, at the given
    /// resolved device: what `/health` reports before `warm_on_start`
    /// completes, or when no model file is present, which §20.3 requires to
    /// be a fully playable configuration.
    #[must_use]
    pub fn unloaded(config: &Config, device: DeviceKind) -> Self {
        let model_id = match device {
            DeviceKind::Cuda { .. } => &config.face.cuda_profile.model_id,
            DeviceKind::Cpu => &config.face.cpu_profile.model_id,
        };
        Self {
            enabled: config.face.enabled,
            provider: "canned",
            device_requested: DeviceRequest::from_config(
                config.face.device,
                config.face.device_index,
            ),
            device,
            device_name: None,
            model_id: model_id.clone(),
            model_loaded: false,
            resident_mb: 0,
            vram_used_mb: 0,
            vram_budget_mb: device.is_cuda().then_some(config.limits.max_vram_mb),
        }
    }

    /// [`Self::unloaded`] at `DeviceKind::Cpu`, for tests and fixtures that do
    /// not resolve a real device.
    #[must_use]
    pub fn unloaded_cpu(config: &Config) -> Self {
        Self::unloaded(config, DeviceKind::Cpu)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub tt: Arc<TranspositionTable>,
    pub breaker: Arc<CircuitBreaker>,
    pub clock: Arc<dyn MonotonicClock>,
    pub face_status: Arc<FaceStatus>,
    pub schema_version: u32,
}

impl AppState {
    #[must_use]
    pub fn new(
        config: Arc<Config>,
        tt: Arc<TranspositionTable>,
        breaker: Arc<CircuitBreaker>,
        clock: Arc<dyn MonotonicClock>,
        face_status: FaceStatus,
        schema_version: u32,
    ) -> Self {
        Self {
            config,
            tt,
            breaker,
            clock,
            face_status: Arc::new(face_status),
            schema_version,
        }
    }

    /// A state with nothing loaded, for router and handler tests.
    #[must_use]
    pub fn for_tests() -> Self {
        let config = Arc::new(Config::default());
        let face_status = FaceStatus::unloaded_cpu(&config);

        Self::new(
            Arc::clone(&config),
            TranspositionTable::disabled(),
            CircuitBreaker::new(&config.face.circuit_breaker),
            SystemClock::new(),
            face_status,
            crate::db::migrations::target_version(),
        )
    }
}

//! Configuration — one file, one binary, no environment-specific code paths.
//!
//! Every field mirrors a key in `draughts.example.toml`, and the section that
//! owns each derivation is named in the doc comment. See §23.

pub mod validate;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use validate::{ValidationError, ValidationReport};

/// The whole of `draughts.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub engine: EngineConfig,
    pub lab: LabConfig,
    pub face: FaceConfig,
    pub limits: LimitsConfig,
}

impl Config {
    /// Read and parse a configuration file.
    ///
    /// Parsing is separate from validation on purpose: [`validate`] needs a
    /// fully-parsed document to compute a projected memory ceiling across
    /// sections, and an error that names a key is worth more than one that
    /// names a byte offset.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source: Box::new(source),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read configuration file {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not parse configuration file {path}")]
    Parse {
        path: PathBuf,
        #[source]
        source: Box<toml::de::Error>,
    },
}

// ---------------------------------------------------------------------------
// [server] — §15
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub static_dir: PathBuf,
    /// Tokio runtime; HTTP only. The engine does not run here (§15.4).
    pub worker_threads: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            static_dir: PathBuf::from("./static"),
            worker_threads: 4,
        }
    }
}

// ---------------------------------------------------------------------------
// [database] — §11.1
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    /// Applied at creation only; ignored for an existing file.
    pub page_size: u32,
    /// `PRAGMA cache_size = -(writer_cache_mb * 1024)` on the write connection.
    pub writer_cache_mb: u64,
    /// Per reader connection, **not** per pool. See §16.1.
    pub reader_cache_mb: u64,
    pub mmap_size_gb: u64,
    pub read_pool_size: usize,
    pub busy_timeout_ms: u64,
    pub checkpoint_every_commits: u64,
    pub journal_size_limit_mb: u64,
    pub writer: WriterConfig,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./data/draughts.db"),
            page_size: 8192,
            writer_cache_mb: 4096,
            reader_cache_mb: 256,
            mmap_size_gb: 8,
            read_pool_size: 6,
            busy_timeout_ms: 30_000,
            checkpoint_every_commits: 64,
            journal_size_limit_mb: 4096,
            writer: WriterConfig::default(),
        }
    }
}

/// `[database.writer]` — the MPSC actor that owns the only write connection (§11.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WriterConfig {
    /// Bounded, and the bound is a memory budget: ~2 GB at the default (§16.1).
    pub channel_capacity: usize,
    /// Target rows per transaction. A performance knob, never a correctness one.
    pub db_batch_rows: usize,
    /// Commit even when the batch is not full.
    pub flush_interval_ms: u64,
    pub max_retries: u32,
    pub retry_backoff_ms: Vec<u64>,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 262_144,
            db_batch_rows: 50_000,
            flush_interval_ms: 250,
            max_retries: 5,
            retry_backoff_ms: vec![10, 50, 250, 1000, 5000],
        }
    }
}

// ---------------------------------------------------------------------------
// [engine] — §16.2
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct EngineConfig {
    pub play: SearchConfig,
    pub lab: SearchConfig,
    pub transposition: TranspositionConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            play: SearchConfig::play_defaults(),
            lab: SearchConfig::lab_defaults(),
            transposition: TranspositionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SearchConfig {
    pub evaluator: String,
    /// A ceiling. On this host `time_budget_ms` binds first in Play Mode (§16.2).
    pub iterations: u32,
    /// `0` = iteration-bounded. A time budget is not reproducible, which is why
    /// lab mode does not have one.
    pub time_budget_ms: u64,
    pub exploration_constant: f32,
    pub worker_threads: usize,
    pub transposition_mode: TtMode,
}

impl SearchConfig {
    fn play_defaults() -> Self {
        Self {
            evaluator: "random_rollout".to_string(),
            iterations: 4000,
            time_budget_ms: 1500,
            exploration_constant: 1.4,
            worker_threads: 1,
            transposition_mode: TtMode::Deterministic,
        }
    }

    fn lab_defaults() -> Self {
        Self {
            evaluator: "random_rollout".to_string(),
            iterations: 800,
            time_budget_ms: 0,
            exploration_constant: 1.2,
            worker_threads: 10,
            transposition_mode: TtMode::Throughput,
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self::play_defaults()
    }
}

/// Whether the table may serve the sample mean of an impure evaluator (§6.7.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtMode {
    /// Move lists and proven scores only. Reproducible.
    Deterministic,
    /// Estimates too. Faster, and deliberately not reproducible.
    Throughput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct TranspositionConfig {
    pub enabled: bool,
    /// ~21 GB at ~83 bytes per entry against a 24 GB budget (§16.3).
    pub capacity_entries: usize,
    /// Power of two, comfortably above the worker count.
    pub shard_count: usize,
    pub reset_between_batches: bool,
    pub retire_batch_size: usize,
    pub huge_pages: HugePages,
}

impl Default for TranspositionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            capacity_entries: 256_000_000,
            shard_count: 512,
            reset_between_batches: false,
            retire_batch_size: 65_536,
            huge_pages: HugePages::Advise,
        }
    }
}

/// `MADV_HUGEPAGE` on the table's backing allocation (§16.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HugePages {
    Advise,
    Off,
}

// ---------------------------------------------------------------------------
// [lab] — §14.2
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LabConfig {
    pub sampling: SamplingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SamplingConfig {
    pub record_positions_every_n_plies: u32,
    pub record_terminal_positions: bool,
    pub max_edges_per_position: usize,
    pub store_child_stats: bool,
    pub only_store_high_visit_edges: bool,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            record_positions_every_n_plies: 2,
            record_terminal_positions: true,
            max_edges_per_position: 16,
            store_child_stats: true,
            only_store_high_visit_edges: false,
        }
    }
}

// ---------------------------------------------------------------------------
// [face] — §7
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FaceConfig {
    pub enabled: bool,
    pub provider: String,
    /// `"cuda" | "cpu" | "auto"` — resolved exactly once, at startup (§7.4.1).
    pub device: DeviceRequestConfig,
    /// Which CUDA device, when there is more than one.
    pub device_index: usize,
    /// Matters more on CUDA, not less (§16.4).
    pub warm_on_start: bool,
    pub deadline_ms: u64,
    pub max_tokens: u32,
    pub max_queue_depth: usize,
    /// CPU path only; inert on CUDA (§15.4).
    pub inference_threads: usize,
    pub min_interval_ms: u64,
    pub lab_mode_enabled: bool,
    pub fallback: String,
    pub verbosity: String,

    /// Two profiles, because the resolved device can change between boots
    /// without anyone editing this file. See §7.5.4.
    pub cuda_profile: ModelProfile,
    pub cpu_profile: ModelProfile,

    pub sampling: FaceSamplingConfig,
    pub circuit_breaker: CircuitBreakerConfig,
}

impl Default for FaceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: "candle".to_string(),
            device: DeviceRequestConfig::Auto,
            device_index: 0,
            warm_on_start: true,
            deadline_ms: 2500,
            max_tokens: 80,
            max_queue_depth: 2,
            inference_threads: 2,
            min_interval_ms: 4000,
            lab_mode_enabled: false,
            fallback: "canned".to_string(),
            verbosity: "low".to_string(),
            cuda_profile: ModelProfile::cuda_defaults(),
            cpu_profile: ModelProfile::cpu_defaults(),
            sampling: FaceSamplingConfig::default(),
            circuit_breaker: CircuitBreakerConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceRequestConfig {
    /// CUDA where available, CPU otherwise. Never an error.
    Auto,
    Cpu,
    Cuda,
}

/// One device profile. Which one is live is decided by `select_device`, not here.
///
/// Deliberately no container-level `default`: with it, a `[face.cuda_profile]`
/// table naming only `model_id` would silently fill `model_path` and
/// `tokenizer_path` from `ModelProfile::default()` — the *CPU* profile's
/// files — leaving a CUDA-labelled profile pointing at CPU weights. Without
/// it, a partial table is a parse error naming the missing key, which is what
/// [`Config::load`] promises.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelProfile {
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub model_id: String,
}

impl ModelProfile {
    fn cuda_defaults() -> Self {
        Self {
            model_path: PathBuf::from("./models/qwen2.5-1.5b-instruct-q4_k_m.gguf"),
            tokenizer_path: PathBuf::from("./models/qwen2.5-1.5b-instruct/tokenizer.json"),
            model_id: "qwen2.5-1.5b-instruct-q4_k_m".to_string(),
        }
    }

    fn cpu_defaults() -> Self {
        Self {
            model_path: PathBuf::from("./models/qwen2.5-0.5b-instruct-q4_k_m.gguf"),
            tokenizer_path: PathBuf::from("./models/qwen2.5-0.5b-instruct/tokenizer.json"),
            model_id: "qwen2.5-0.5b-instruct-q4_k_m".to_string(),
        }
    }
}

impl Default for ModelProfile {
    fn default() -> Self {
        Self::cpu_defaults()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FaceSamplingConfig {
    pub temperature: f64,
    pub top_p: f64,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
    /// `0` = per-request; non-zero = reproducible taunts.
    pub seed: u64,
}

impl Default for FaceSamplingConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            repeat_penalty: 1.1,
            repeat_last_n: 64,
            seed: 0,
        }
    }
}

/// §7.8. Three failures amputate the Face layer for five minutes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub cooldown_seconds: u64,
    pub half_open_probes: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            cooldown_seconds: 300,
            half_open_probes: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// [limits] — §16.1, §16.6
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LimitsConfig {
    /// Host RAM ceiling, validated before the listener binds (§16.1).
    pub max_total_memory_gb: u64,
    /// Device memory ceiling, validated before the model loads (§16.6).
    pub max_vram_mb: u64,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_total_memory_gb: 56,
            max_vram_mb: 4608,
        }
    }
}

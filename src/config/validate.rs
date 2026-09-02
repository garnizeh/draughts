//! Startup validation — §23.1.
//!
//! **Startup validation is not optional.** A configuration that can OOM the
//! process, or that puts a 4.3-second model against a 2.5-second deadline,
//! should fail in the first second with a message naming the offending key —
//! not at hour six of a batch, and not as a permanently open circuit that
//! reports itself healthy.
//!
//! Every constant below is derived from a figure in the architecture, and the
//! section that owns the derivation is named. Re-derive them for a different
//! host; do not tune them until they pass.

use std::path::Path;

use super::Config;
use crate::face::DeviceKind;

// --- Host memory, §16.1 / §16.3 --------------------------------------------

/// ~64 packed bytes plus ~30 % DashMap bookkeeping (§16.3).
const TT_BYTES_PER_ENTRY: u64 = 83;

/// Worst-case size of one queued write message. `channel_capacity` is a memory
/// budget expressed in slots: 262 144 × 8 KiB ≈ 2 GB (§15.2.1, §16.1).
const WRITE_MESSAGE_BYTES: u64 = 8 * 1024;

/// Arena cap per MCTS worker. A search aborts a node rather than growing past
/// it (§16.1).
const MCTS_ARENA_MB: u64 = 512;

/// Tokenizer and staging buffers, on either device. The weights are counted
/// separately, from the file on disk (§7.5.6).
const FACE_STAGING_MB: u64 = 1024;

/// Allocator overhead and fragmentation. Observed, not enforced (§16.1).
const RUNTIME_OVERHEAD_MB: u64 = 4096;

// --- Device memory, §16.6 --------------------------------------------------

/// KV cache plus CUDA context, cuBLAS workspace and allocator slack:
/// 0.3 GB + 0.5 GB. Added to the weights to give the Face layer's VRAM total.
const VRAM_FIXED_OVERHEAD_MB: u64 = 800;

// --- Deadline feasibility, §7.5.1 ------------------------------------------

/// Two Broadwell cores on two populated DDR4-2400 channels, competing with ten
/// MCTS workers hammering a 21 GB hash table. 12–18 GB/s; use 15.
const CPU_BANDWIDTH_BYTES_PER_SEC: f64 = 15.0 * 1e9;

/// 96-bit GDDR6 at 14 Gbps is ~168 GB/s nameplate. Candle's quantized CUDA
/// kernels are not llama.cpp's; the 55 % discount is deliberate.
const CUDA_BANDWIDTH_BYTES_PER_SEC: f64 = 168.0 * 1e9 * 0.55;

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;

/// A validation failure that names the key responsible for it.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error(
        "projected memory ceiling is {projected_gb:.1} GB, over the \
         limits.max_total_memory_gb budget of {budget_gb} GB. Largest consumer: \
         {largest_key} at {largest_gb:.1} GB"
    )]
    MemoryBudgetExceeded {
        projected_gb: f64,
        budget_gb: u64,
        largest_key: &'static str,
        largest_gb: f64,
    },

    #[error(
        "face.{profile}.model_path ({model_mb} MB) projects {projected_mb} MB of VRAM \
         against a limits.max_vram_mb budget of {budget_mb} MB"
    )]
    VramBudgetExceeded {
        profile: &'static str,
        model_mb: u64,
        projected_mb: u64,
        budget_mb: u64,
    },

    #[error(
        "the active profile face.{profile} ({model_mb} MB) projects {projected_ms} ms to \
         generate face.max_tokens = {max_tokens} tokens on {device}, over the \
         face.deadline_ms of {deadline_ms} ms. A Face layer that cannot meet its \
         deadline is a circuit that will be open within three moves"
    )]
    DeadlineInfeasible {
        profile: &'static str,
        device: &'static str,
        model_mb: u64,
        projected_ms: u64,
        max_tokens: u32,
        deadline_ms: u64,
    },

    #[error("engine.transposition.shard_count = {0} must be a power of two of at least 2")]
    ShardCountNotPowerOfTwo(usize),

    #[error(
        "database.writer.retry_backoff_ms has {actual} entries but \
         database.writer.max_retries is {expected}"
    )]
    RetryBackoffLengthMismatch { expected: u32, actual: usize },

    #[error("{key} must be greater than zero")]
    MustBePositive { key: &'static str },
}

/// The outcome of validation. Warnings do not prevent startup; errors do.
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    /// Line items behind the memory projection, for `--check-config` and logs.
    pub memory_breakdown: Vec<(&'static str, u64)>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn projected_memory_bytes(&self) -> u64 {
        self.memory_breakdown.iter().map(|(_, bytes)| bytes).sum()
    }
}

/// Validate a parsed configuration against the resolved inference device.
///
/// The device matters: the *active* profile failing its deadline is a refusal
/// to start, while the *inactive* one failing is a warning. A CUDA deployment
/// whose CPU fallback cannot meet the deadline is one driver update away from a
/// silent outage, which is worth saying loudly and exactly once.
pub fn validate(config: &Config, device: DeviceKind) -> ValidationReport {
    let mut report = ValidationReport::default();

    check_shapes(config, &mut report);
    check_host_memory(config, &mut report);
    check_deadlines(config, device, &mut report);
    check_vram(config, device, &mut report);

    report
}

/// Cheap structural checks that do not need arithmetic to justify.
fn check_shapes(config: &Config, report: &mut ValidationReport) {
    let tt = &config.engine.transposition;
    if !tt.shard_count.is_power_of_two()
        || tt.shard_count < crate::engine::TranspositionTable::MIN_SHARDS
    {
        report
            .errors
            .push(ValidationError::ShardCountNotPowerOfTwo(tt.shard_count));
    }

    if tt.enabled && tt.shard_count <= config.engine.lab.worker_threads {
        report.warnings.push(format!(
            "engine.transposition.shard_count = {} does not comfortably exceed \
             engine.lab.worker_threads = {}; expect shard contention (§16.3)",
            tt.shard_count, config.engine.lab.worker_threads
        ));
    }

    let writer = &config.database.writer;
    if writer.retry_backoff_ms.len() != writer.max_retries as usize {
        report
            .errors
            .push(ValidationError::RetryBackoffLengthMismatch {
                expected: writer.max_retries,
                actual: writer.retry_backoff_ms.len(),
            });
    }

    for (key, value) in [
        ("database.writer.db_batch_rows", writer.db_batch_rows),
        ("database.writer.channel_capacity", writer.channel_capacity),
        ("database.read_pool_size", config.database.read_pool_size),
        ("server.worker_threads", config.server.worker_threads),
        (
            "engine.play.worker_threads",
            config.engine.play.worker_threads,
        ),
        (
            "engine.lab.worker_threads",
            config.engine.lab.worker_threads,
        ),
    ] {
        if value == 0 {
            report.errors.push(ValidationError::MustBePositive { key });
        }
    }

    if config.engine.play.iterations == 0 && config.engine.play.time_budget_ms == 0 {
        report.errors.push(ValidationError::MustBePositive {
            key: "engine.play.iterations (or engine.play.time_budget_ms)",
        });
    }

    if config.engine.lab.time_budget_ms != 0 {
        report.warnings.push(
            "engine.lab.time_budget_ms is non-zero; a time-bounded search is a \
             function of machine load, and a batch whose strength varies with \
             what else the machine was doing produces training data with an \
             invisible confound (§16.2)"
                .to_string(),
        );
    }
}

/// The host-memory ceiling. Computed across sections, because no single section
/// can see the total it contributes to.
fn check_host_memory(config: &Config, report: &mut ValidationReport) {
    let db = &config.database;
    let tt = &config.engine.transposition;

    let tt_bytes = if tt.enabled {
        tt.capacity_entries as u64 * TT_BYTES_PER_ENTRY
    } else {
        0
    };

    report.memory_breakdown = vec![
        ("engine.transposition.capacity_entries", tt_bytes),
        (
            "database.writer.channel_capacity",
            db.writer.channel_capacity as u64 * WRITE_MESSAGE_BYTES,
        ),
        ("database.writer_cache_mb", db.writer_cache_mb * MB),
        (
            "database.reader_cache_mb × read_pool_size",
            db.reader_cache_mb * db.read_pool_size as u64 * MB,
        ),
        (
            "engine.lab.worker_threads (MCTS arenas)",
            config.engine.lab.worker_threads as u64 * MCTS_ARENA_MB * MB,
        ),
        (
            "face (staging + CPU-profile weights)",
            face_host_bytes(config),
        ),
        (
            "rust runtime and allocator overhead",
            RUNTIME_OVERHEAD_MB * MB,
        ),
    ];

    let projected = report.projected_memory_bytes();
    let budget = config.limits.max_total_memory_gb * GB;

    if projected > budget {
        let (largest_key, largest_bytes) = report
            .memory_breakdown
            .iter()
            .max_by_key(|(_, bytes)| *bytes)
            .copied()
            .unwrap_or(("unknown", 0));

        report.errors.push(ValidationError::MemoryBudgetExceeded {
            projected_gb: projected as f64 / GB as f64,
            budget_gb: config.limits.max_total_memory_gb,
            largest_key,
            largest_gb: largest_bytes as f64 / GB as f64,
        });
    }

    // `mmap_size` is a virtual mapping served by the page cache. It competes for
    // the same physical pages as the OS reservation but is not private process
    // memory, so it is deliberately excluded from the sum above (§16.1).
    if db.mmap_size_gb * GB > budget {
        report.warnings.push(format!(
            "database.mmap_size_gb = {} exceeds the whole memory budget; it is \
             not counted as process memory, but a window this large is not free \
             on a host this size (§11.1)",
            db.mmap_size_gb
        ));
    }
}

fn face_host_bytes(config: &Config) -> u64 {
    if !config.face.enabled {
        return 0;
    }
    let weights = model_file_bytes(&config.face.cpu_profile.model_path).unwrap_or(0);
    FACE_STAGING_MB * MB + weights
}

/// Deadline feasibility, checked for **both** profiles (§23.1).
///
/// The estimate is deliberately crude and deliberately conservative: generating
/// one token reads the whole quantized model once, so the floor is
/// `max_tokens × model_bytes / bandwidth`. It is not trying to predict latency.
/// It is trying to catch a configuration that is wrong by a factor of two or
/// more, which is the only kind of error this class of defect has ever taken.
fn check_deadlines(config: &Config, device: DeviceKind, report: &mut ValidationReport) {
    if !config.face.enabled {
        return;
    }

    let profiles: [(&'static str, &Path, &'static str, f64, bool); 2] = [
        (
            "cuda_profile",
            config.face.cuda_profile.model_path.as_path(),
            "cuda",
            CUDA_BANDWIDTH_BYTES_PER_SEC,
            matches!(device, DeviceKind::Cuda { .. }),
        ),
        (
            "cpu_profile",
            config.face.cpu_profile.model_path.as_path(),
            "cpu",
            CPU_BANDWIDTH_BYTES_PER_SEC,
            matches!(device, DeviceKind::Cpu),
        ),
    ];

    for (profile, path, device_name, bandwidth, is_active) in profiles {
        let Some(model_bytes) = model_file_bytes(path) else {
            // A missing model file is not a configuration error: the breaker
            // opens and the service runs on canned lines (§17.2). It does mean
            // the deadline cannot be checked, which is worth saying.
            report.warnings.push(format!(
                "face.{profile}.model_path ({}) is not readable; its deadline \
                 feasibility could not be checked (§23.1)",
                path.display()
            ));
            continue;
        };

        let projected_ms = projected_generation_ms(model_bytes, config.face.max_tokens, bandwidth);
        if projected_ms <= config.face.deadline_ms {
            continue;
        }

        let failure = ValidationError::DeadlineInfeasible {
            profile,
            device: device_name,
            model_mb: model_bytes / MB,
            projected_ms,
            max_tokens: config.face.max_tokens,
            deadline_ms: config.face.deadline_ms,
        };

        if is_active {
            report.errors.push(failure);
        } else {
            report.warnings.push(format!(
                "{failure} — this profile is not the active one, so the process \
                 will start, but a device change would turn this into a silent \
                 outage (§7.5.4)"
            ));
        }
    }
}

/// Projected wall-clock to generate `max_tokens`, in milliseconds.
pub fn projected_generation_ms(model_bytes: u64, max_tokens: u32, bandwidth: f64) -> u64 {
    let seconds = (model_bytes as f64 * f64::from(max_tokens)) / bandwidth;
    (seconds * 1000.0).round() as u64
}

/// Projected VRAM for a profile: quantized weights, plus the KV cache and the
/// fixed cost of having a CUDA context at all (§16.6).
pub fn projected_vram_mb(model_bytes: u64) -> u64 {
    model_bytes / MB + VRAM_FIXED_OVERHEAD_MB
}

/// The VRAM budget is only meaningful when the resolved device is CUDA.
///
/// Exceeding it is a refusal to load, not a CUDA OOM: the error names the
/// figure and the budget, which is information a driver error message does not
/// carry.
fn check_vram(config: &Config, device: DeviceKind, report: &mut ValidationReport) {
    if !config.face.enabled || !matches!(device, DeviceKind::Cuda { .. }) {
        return;
    }

    let Some(model_bytes) = model_file_bytes(&config.face.cuda_profile.model_path) else {
        return;
    };

    let projected_mb = projected_vram_mb(model_bytes);
    if projected_mb > config.limits.max_vram_mb {
        report.errors.push(ValidationError::VramBudgetExceeded {
            profile: "cuda_profile",
            model_mb: model_bytes / MB,
            projected_mb,
            budget_mb: config.limits.max_vram_mb,
        });
    }
}

fn model_file_bytes(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

/// Warn when the configuration promises more memory than the host has.
///
/// Separate from [`validate`] because it reads the machine rather than the
/// document, and a test fixture should not fail on the CI runner's RAM.
pub fn check_against_host(config: &Config, report: &mut ValidationReport) {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let host_bytes = system.total_memory();

    if host_bytes == 0 {
        return;
    }

    let budget = config.limits.max_total_memory_gb * GB;
    if budget > host_bytes {
        report.warnings.push(format!(
            "limits.max_total_memory_gb = {} GB exceeds the host's {:.1} GB of \
             RAM; the budget is not a budget on this machine (§16.1)",
            config.limits.max_total_memory_gb,
            host_bytes as f64 / GB as f64,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration_fits_its_own_budget() {
        let config = Config::default();
        let report = validate(&config, DeviceKind::Cpu);
        assert!(
            report.is_ok(),
            "the committed defaults must validate: {:?}",
            report.errors
        );
    }

    #[test]
    fn an_oversized_table_is_refused_and_names_its_key() {
        let mut config = Config::default();
        config.engine.transposition.capacity_entries = 2_000_000_000;

        let report = validate(&config, DeviceKind::Cpu);

        let Some(ValidationError::MemoryBudgetExceeded { largest_key, .. }) = report
            .errors
            .iter()
            .find(|e| matches!(e, ValidationError::MemoryBudgetExceeded { .. }))
        else {
            panic!("expected a memory budget error, got {:?}", report.errors);
        };
        assert_eq!(*largest_key, "engine.transposition.capacity_entries");
    }

    #[test]
    fn shard_count_must_be_a_power_of_two() {
        let mut config = Config::default();
        config.engine.transposition.shard_count = 500;

        let report = validate(&config, DeviceKind::Cpu);

        assert!(
            report
                .errors
                .iter()
                .any(|e| matches!(e, ValidationError::ShardCountNotPowerOfTwo(500)))
        );
    }

    /// `DashMap` refuses a single shard, and the refusal is a panic inside the
    /// allocation §22.3 calls fatal. Catching it here turns it into a message
    /// naming the key.
    #[test]
    fn a_single_shard_is_refused_before_the_table_is_allocated() {
        let mut config = Config::default();
        config.engine.transposition.shard_count = 1;

        let report = validate(&config, DeviceKind::Cpu);

        assert!(
            report
                .errors
                .iter()
                .any(|e| matches!(e, ValidationError::ShardCountNotPowerOfTwo(1)))
        );
    }

    /// §0.4.3: the 1.5B model on two Broadwell cores needs ~4.3 s for 64 tokens
    /// against a 2.5 s deadline. This is the defect the check exists to catch.
    #[test]
    fn the_cpu_path_cannot_run_the_cuda_profile_model() {
        let cuda_profile_bytes = 1_000 * MB;
        let ms = projected_generation_ms(cuda_profile_bytes, 64, CPU_BANDWIDTH_BYTES_PER_SEC);
        assert!(
            (4_000..=4_600).contains(&ms),
            "expected ~4.3 s, got {ms} ms — re-derive §7.5.1 before changing this"
        );
        assert!(ms > 2500, "and it must exceed the default deadline");
    }

    #[test]
    fn the_cpu_fallback_profile_does_meet_the_deadline() {
        let cpu_profile_bytes = 400 * MB;
        let ms = projected_generation_ms(cpu_profile_bytes, 64, CPU_BANDWIDTH_BYTES_PER_SEC);
        assert!(ms < 2500, "expected under the 2.5 s deadline, got {ms} ms");
    }

    /// §16.6: 1.0 GB of weights plus 0.8 GB of KV cache and context is the
    /// 1.8 GB "Face layer total" row, against a 4.5 GB cap.
    #[test]
    fn the_cuda_profile_fits_the_vram_budget() {
        let projected = projected_vram_mb(1_000 * MB);
        assert_eq!(projected, 1_800);
        assert!(projected < LimitsConfig::default().max_vram_mb);
    }

    use crate::config::LimitsConfig;
}

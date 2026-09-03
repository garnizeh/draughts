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

use super::{CircuitBreakerConfig, Config, DrawConfig};
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

pub(crate) const MB: u64 = 1024 * 1024;
pub(crate) const GB: u64 = 1024 * MB;

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

    #[error(
        "rules.draw.repetition_count = {0} would draw a game at the opening position; \
         a repetition rule needs at least two occurrences to have seen one (§5.3.1)"
    )]
    RepetitionCountTooLow(u32),
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
        self.memory_breakdown
            .iter()
            .fold(0u64, |total, (_, bytes)| total.saturating_add(*bytes))
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

    // Stat each model path once and hand the result to every check that needs
    // it — `check_host_memory`, `check_deadlines`, and `check_vram` would
    // otherwise each re-read the same two files.
    let model_sizes = ModelSizes {
        cpu: model_file_bytes(&config.face.cpu_profile.model_path),
        cuda: model_file_bytes(&config.face.cuda_profile.model_path),
    };

    check_shapes(config, &mut report);
    check_host_memory(config, &model_sizes, &mut report);
    check_deadlines(config, device, &model_sizes, &mut report);
    check_vram(config, device, &model_sizes, &mut report);

    report
}

/// The two profiles' model file sizes, resolved once per `validate()` call.
struct ModelSizes {
    cpu: Option<u64>,
    cuda: Option<u64>,
}

/// One `[face.*_profile]` section's inputs to the deadline-feasibility check.
struct DeadlineProfile<'a> {
    name: &'static str,
    path: &'a Path,
    device_name: &'static str,
    bandwidth: f64,
    is_active: bool,
    model_bytes: Option<u64>,
}

/// Cheap structural checks that do not need arithmetic to justify.
fn check_shapes(config: &Config, report: &mut ValidationReport) {
    let tt = &config.engine.transposition;
    // Only checked when the table is actually built from it: `main.rs` calls
    // `TranspositionTable::disabled()` — which hardcodes `MIN_SHARDS` and never
    // reads `shard_count` — when `tt.enabled` is false, so refusing to start
    // over a value the running process would never touch would be validation
    // stricter than the precondition it exists to protect.
    if tt.enabled
        && (!tt.shard_count.is_power_of_two()
            || tt.shard_count < crate::engine::TranspositionTable::MIN_SHARDS)
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

    // `engine.lab.time_budget_ms` is expected to stay 0 (it gets its own
    // warning below when it isn't, per §16.2), which makes `iterations` the
    // only knob that can give a lab run a search budget at all — the same
    // zero/zero degenerate case the check above catches for Play Mode.
    if config.engine.lab.iterations == 0 && config.engine.lab.time_budget_ms == 0 {
        report.errors.push(ValidationError::MustBePositive {
            key: "engine.lab.iterations (or engine.lab.time_budget_ms)",
        });
    }

    // `CircuitBreaker::new` clamps a configured `0` up to `1` (`.max(1)`,
    // `src/face/breaker.rs`) rather than rejecting it, so a `0` here would
    // otherwise resolve to a runtime value that silently disagrees with what
    // was configured.
    if config.face.circuit_breaker.failure_threshold == 0 {
        report.warnings.push(
            "face.circuit_breaker.failure_threshold = 0 is clamped to 1 at runtime; the \
             breaker will trip on the first counted failure"
                .to_string(),
        );
    }

    // These two keys are accepted and validated like any other, but nothing
    // reads them yet: `CircuitBreaker::new` hardcodes a single half-open
    // probe token (`src/face/breaker.rs`), and `Sampler` does not filter by
    // visit count (`src/lab/sampling.rs`). An operator who changes either
    // away from its default sees no behavior change and, without this
    // warning, no indication why.
    if config.face.circuit_breaker.half_open_probes
        != CircuitBreakerConfig::default().half_open_probes
    {
        report.warnings.push(
            "face.circuit_breaker.half_open_probes is not yet wired to the breaker, which \
             always admits exactly one probe; this value has no effect"
                .to_string(),
        );
    }
    if config.lab.sampling.only_store_high_visit_edges {
        report.warnings.push(
            "lab.sampling.only_store_high_visit_edges is not yet wired to the sampler; this \
             value has no effect"
                .to_string(),
        );
    }
    // The same again for the draw policy, and it goes away when the seams in
    // `rules::moves` and the game loop are implemented (§5.3.1). The permanent
    // half of this — that a departed policy is still recorded as
    // `english_draughts` — is in `check_draw_policy` and stays after that.
    if config.rules.draw != DrawConfig::default() {
        report.warnings.push(
            "rules.draw is not yet wired to the rules core or the game loop, which do not \
             adjudicate draws yet; this value has no effect"
                .to_string(),
        );
    }

    check_draw_policy(config, report);

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

/// The draw policy — checked for shape, and for variant (§23.1).
///
/// The two shape checks are refusals because they describe something that is
/// not a game: `non_progress_plies = 0` draws every game before its first move,
/// and `repetition_count < 2` draws one at the opening position, having seen no
/// repetition at all. Everything else is accepted, because being movable is the
/// entire point of these being keys (§19.4) — but not in silence, because
/// `games.rules` records `english_draughts` for every game whatever thresholds
/// it was actually played under, and a dataset whose draws came from a
/// threshold nobody remembers changing carries an invisible confound.
fn check_draw_policy(config: &Config, report: &mut ValidationReport) {
    let draw = &config.rules.draw;
    let defaults = DrawConfig::default();

    if draw.non_progress_plies == 0 {
        report.errors.push(ValidationError::MustBePositive {
            key: "rules.draw.non_progress_plies",
        });
    }

    if draw.repetition_count < 2 {
        report.errors.push(ValidationError::RepetitionCountTooLow(
            draw.repetition_count,
        ));
    }

    let moved: Vec<&'static str> = [
        (
            "rules.draw.non_progress_plies",
            draw.non_progress_plies != defaults.non_progress_plies,
        ),
        (
            "rules.draw.non_progress_reset",
            draw.non_progress_reset != defaults.non_progress_reset,
        ),
        (
            "rules.draw.repetition_count",
            draw.repetition_count != defaults.repetition_count,
        ),
        (
            "rules.draw.repetition_window",
            draw.repetition_window != defaults.repetition_window,
        ),
    ]
    .into_iter()
    .filter_map(|(key, departed)| departed.then_some(key))
    .collect();

    if !moved.is_empty() {
        report.warnings.push(format!(
            "{} departs from the english_draughts draw rules in §5.3.1; games played \
             under this policy are still recorded with rules = \"english_draughts\"",
            moved.join(", ")
        ));
    }
}

/// The host-memory ceiling. Computed across sections, because no single section
/// can see the total it contributes to.
fn check_host_memory(config: &Config, model_sizes: &ModelSizes, report: &mut ValidationReport) {
    let db = &config.database;
    let tt = &config.engine.transposition;

    // Every projection is `saturating`: TOML accepts any `u64`, and a
    // mistyped value (`capacity_entries = 1_000_000_000_000_000_000`) must
    // saturate to "obviously over budget" rather than wrap to a small number
    // that passes the check it exists to enforce.
    let tt_bytes = if tt.enabled {
        (tt.capacity_entries as u64).saturating_mul(TT_BYTES_PER_ENTRY)
    } else {
        0
    };

    report.memory_breakdown = vec![
        ("engine.transposition.capacity_entries", tt_bytes),
        (
            "database.writer.channel_capacity",
            (db.writer.channel_capacity as u64).saturating_mul(WRITE_MESSAGE_BYTES),
        ),
        (
            "database.writer_cache_mb",
            db.writer_cache_mb.saturating_mul(MB),
        ),
        (
            "database.reader_cache_mb × read_pool_size",
            db.reader_cache_mb
                .saturating_mul(db.read_pool_size as u64)
                .saturating_mul(MB),
        ),
        (
            "engine.lab.worker_threads (MCTS arenas)",
            (config.engine.lab.worker_threads as u64)
                .saturating_mul(MCTS_ARENA_MB)
                .saturating_mul(MB),
        ),
        (
            "face (staging + CPU-profile weights)",
            face_host_bytes(config, model_sizes.cpu),
        ),
        (
            "rust runtime and allocator overhead",
            RUNTIME_OVERHEAD_MB * MB,
        ),
    ];

    let projected = report.projected_memory_bytes();
    let budget = config.limits.max_total_memory_gb.saturating_mul(GB);

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
    if db.mmap_size_gb.saturating_mul(GB) > budget {
        report.warnings.push(format!(
            "database.mmap_size_gb = {} exceeds the whole memory budget; it is \
             not counted as process memory, but a window this large is not free \
             on a host this size (§11.1)",
            db.mmap_size_gb
        ));
    }
}

fn face_host_bytes(config: &Config, cpu_model_bytes: Option<u64>) -> u64 {
    if !config.face.enabled {
        return 0;
    }
    FACE_STAGING_MB * MB + cpu_model_bytes.unwrap_or(0)
}

/// Deadline feasibility, checked for **both** profiles (§23.1).
///
/// The estimate is deliberately crude and deliberately conservative: generating
/// one token reads the whole quantized model once, so the floor is
/// `max_tokens × model_bytes / bandwidth`. It is not trying to predict latency.
/// It is trying to catch a configuration that is wrong by a factor of two or
/// more, which is the only kind of error this class of defect has ever taken.
fn check_deadlines(
    config: &Config,
    device: DeviceKind,
    model_sizes: &ModelSizes,
    report: &mut ValidationReport,
) {
    if !config.face.enabled {
        return;
    }

    let profiles = [
        DeadlineProfile {
            name: "cuda_profile",
            path: config.face.cuda_profile.model_path.as_path(),
            device_name: "cuda",
            bandwidth: CUDA_BANDWIDTH_BYTES_PER_SEC,
            is_active: matches!(device, DeviceKind::Cuda { .. }),
            model_bytes: model_sizes.cuda,
        },
        DeadlineProfile {
            name: "cpu_profile",
            path: config.face.cpu_profile.model_path.as_path(),
            device_name: "cpu",
            bandwidth: CPU_BANDWIDTH_BYTES_PER_SEC,
            is_active: matches!(device, DeviceKind::Cpu),
            model_bytes: model_sizes.cpu,
        },
    ];

    for DeadlineProfile {
        name: profile,
        path,
        device_name,
        bandwidth,
        is_active,
        model_bytes,
    } in profiles
    {
        let Some(model_bytes) = model_bytes else {
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
fn check_vram(
    config: &Config,
    device: DeviceKind,
    model_sizes: &ModelSizes,
    report: &mut ValidationReport,
) {
    if !config.face.enabled || !matches!(device, DeviceKind::Cuda { .. }) {
        return;
    }

    let Some(model_bytes) = model_sizes.cuda else {
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

    let budget = config.limits.max_total_memory_gb.saturating_mul(GB);
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

    /// A mistyped, absurdly large value must saturate toward "over budget"
    /// rather than wrap past `u64::MAX` back into a value the budget check
    /// reads as fine — the exact failure mode unchecked multiplication has.
    #[test]
    fn a_wildly_oversized_value_saturates_instead_of_wrapping() {
        let mut config = Config::default();
        config.engine.transposition.capacity_entries = usize::MAX;

        let report = validate(&config, DeviceKind::Cpu);

        assert!(
            report
                .errors
                .iter()
                .any(|e| matches!(e, ValidationError::MemoryBudgetExceeded { .. })),
            "expected a memory budget error, got {:?}",
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

    /// §5.3.1: the shipped defaults are the English-draughts rules, and a
    /// configuration that has not touched them says nothing about them.
    #[test]
    fn the_default_draw_policy_is_english_draughts_and_warns_about_nothing() {
        let config = Config::default();
        assert_eq!(config.rules.draw.non_progress_plies, 80);
        assert_eq!(config.rules.draw.repetition_count, 3);

        let report = validate(&config, DeviceKind::Cpu);
        assert!(
            !report.warnings.iter().any(|w| w.contains("rules.draw")),
            "the defaults must not warn about themselves: {:?}",
            report.warnings
        );
    }

    /// A threshold of zero draws every game before its first move, which is not
    /// a game. §23.1 refuses it rather than letting a batch produce a million
    /// draws and no error.
    #[test]
    fn a_zero_non_progress_threshold_is_refused_and_names_its_key() {
        let mut config = Config::default();
        config.rules.draw.non_progress_plies = 0;

        let report = validate(&config, DeviceKind::Cpu);

        assert!(
            report.errors.iter().any(|e| matches!(
                e,
                ValidationError::MustBePositive {
                    key: "rules.draw.non_progress_plies"
                }
            )),
            "expected a refusal naming the key, got {:?}",
            report.errors
        );
    }

    /// One occurrence is not a repetition: the opening position would be drawn
    /// before anybody moved.
    #[test]
    fn a_repetition_count_below_two_is_refused() {
        for count in [0, 1] {
            let mut config = Config::default();
            config.rules.draw.repetition_count = count;

            let report = validate(&config, DeviceKind::Cpu);

            assert!(
                report
                    .errors
                    .iter()
                    .any(|e| matches!(e, ValidationError::RepetitionCountTooLow(_))),
                "repetition_count = {count} must be refused, got {:?}",
                report.errors
            );
        }
    }

    /// §23.1: moving the policy is allowed — that is what makes it a key — but
    /// it is never silent, because `games.rules` will still say
    /// `english_draughts`. The warning names the keys that actually moved and
    /// not the ones that did not.
    #[test]
    fn a_departed_draw_policy_warns_and_names_only_the_keys_that_moved() {
        let mut config = Config::default();
        config.rules.draw.non_progress_plies = 100;
        config.rules.draw.non_progress_reset = NonProgressReset::Capture;

        let report = validate(&config, DeviceKind::Cpu);

        assert!(report.is_ok(), "a departure is a warning, not a refusal");

        let warning = report
            .warnings
            .iter()
            .find(|w| w.contains("english_draughts"))
            .expect("a departed policy must warn");

        assert!(warning.contains("rules.draw.non_progress_plies"));
        assert!(warning.contains("rules.draw.non_progress_reset"));
        assert!(!warning.contains("rules.draw.repetition_count"));
        assert!(!warning.contains("rules.draw.repetition_window"));
    }

    use crate::config::NonProgressReset;
}

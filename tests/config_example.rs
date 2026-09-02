//! The committed `draughts.example.toml` must parse and must validate.
//!
//! `#[serde(deny_unknown_fields)]` makes this a two-way check: a key added to
//! the struct and not to the example is caught by the assertions below, and a
//! key left in the example after a rename fails the parse. The example is the
//! file operators copy, and §23 is the file reviewers read; this test is what
//! keeps the two of them and the code in agreement.

use draughts::config::{Config, TtMode, validate};
use draughts::face::DeviceKind;

fn example() -> Config {
    Config::load(std::path::Path::new("draughts.example.toml"))
        .expect("draughts.example.toml must parse")
}

#[test]
fn the_example_configuration_parses() {
    let config = example();

    assert_eq!(config.server.port, 8080);
    assert_eq!(config.database.writer.db_batch_rows, 50_000);
    assert_eq!(config.engine.transposition.capacity_entries, 256_000_000);
    assert_eq!(config.limits.max_total_memory_gb, 56);
    assert_eq!(config.limits.max_vram_mb, 4608);
}

/// §23.1: the committed example must be a configuration this build would agree
/// to start with. An example that fails its own validation is worse than no
/// example, because it is the file everybody copies.
#[test]
fn the_example_configuration_validates() {
    let report = validate::validate(&example(), DeviceKind::Cpu);

    assert!(
        report.is_ok(),
        "draughts.example.toml does not validate: {:?}",
        report.errors
    );
}

/// §16.1: the projected ceiling must sit under the budget with room to spare,
/// not scrape past it. A committed example at 99 % of its own limit is a
/// configuration that fails on the first machine with slightly different
/// arithmetic.
#[test]
fn the_example_leaves_headroom_against_its_own_budget() {
    let config = example();
    let report = validate::validate(&config, DeviceKind::Cpu);

    let gb = 1024.0 * 1024.0 * 1024.0;
    let projected = report.projected_memory_bytes() as f64 / gb;
    let budget = config.limits.max_total_memory_gb as f64;

    assert!(
        projected < budget * 0.9,
        "projected {projected:.1} GB against a {budget} GB budget leaves no headroom"
    );
}

/// §16.2. These two are opposite by design, and a change that makes them agree
/// has broken one of them.
#[test]
fn play_mode_is_deterministic_and_lab_mode_is_throughput() {
    let config = example();

    assert_eq!(
        config.engine.play.transposition_mode,
        TtMode::Deterministic,
        "a human's move must be reproducible"
    );
    assert_eq!(
        config.engine.lab.transposition_mode,
        TtMode::Throughput,
        "a batch is not, and §20.5 asserts the divergence"
    );
}

/// §7.5.4: two profiles, and they must not be the same file. One `model_path`
/// is the silent-outage hazard this whole arrangement exists to remove.
#[test]
fn the_two_device_profiles_are_actually_different() {
    let config = example();

    assert_ne!(
        config.face.cuda_profile.model_path,
        config.face.cpu_profile.model_path
    );
    assert_ne!(
        config.face.cuda_profile.model_id,
        config.face.cpu_profile.model_id
    );
}

/// §15.4: the lab pool must leave the host cores for Play Mode, the Face layer,
/// and the writer. Ten workers on fourteen physical cores is the derived
/// figure; a configuration that claims all of them starves the reservation that
/// keeps a human's move fast under lab load.
#[test]
fn the_lab_pool_leaves_cores_for_everything_else() {
    let config = example();

    let claimed = config.engine.lab.worker_threads
        + config.engine.play.worker_threads
        + config.face.inference_threads;

    assert!(
        claimed <= 14,
        "the lab pool and its neighbours claim {claimed} of 14 physical cores"
    );
}

//! Position sampling — §14.
//!
//! Sampling density is the knob that decides how much of a batch's search
//! becomes training data. It is also the knob that decides how much of the
//! writer's throughput a batch consumes, which is why §16.3.1 recommends tuning
//! hit rate before capacity: both spend the same scarce resource.

use crate::config::SamplingConfig;
use crate::engine::ChildStats;

pub struct Sampler {
    config: SamplingConfig,
}

impl Sampler {
    #[must_use]
    pub fn new(config: SamplingConfig) -> Self {
        Self { config }
    }

    /// Whether a position at this ply is recorded.
    #[must_use]
    pub fn should_record(&self, ply: u32, terminal: bool) -> bool {
        if terminal {
            return self.config.record_terminal_positions;
        }
        if self.config.record_positions_every_n_plies == 0 {
            return false;
        }
        ply.is_multiple_of(self.config.record_positions_every_n_plies)
    }

    /// Trim root statistics to the configured edge budget, highest-visit first.
    ///
    /// Truncation is by visits rather than by move order so that a trimmed row
    /// still carries the distribution's mass — a policy target built from the
    /// tail would be worse than no target at all.
    #[must_use]
    pub fn select_edges(&self, mut stats: Vec<ChildStats>) -> Vec<ChildStats> {
        if !self.config.store_child_stats {
            return Vec::new();
        }

        stats.sort_unstable_by_key(|stat| std::cmp::Reverse(stat.visits));
        stats.truncate(self.config.max_edges_per_position);
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Move, MoveFlags};

    fn stats(visits: u32) -> ChildStats {
        ChildStats {
            mv: Move {
                from: 0,
                to: 1,
                flags: MoveFlags::NONE,
            },
            visits,
            wins: 0,
            draws: 0,
            losses: 0,
            q_value: 0.0,
            prior: 0.0,
        }
    }

    #[test]
    fn the_default_density_records_every_second_ply() {
        let sampler = Sampler::new(SamplingConfig::default());

        assert!(sampler.should_record(0, false));
        assert!(!sampler.should_record(1, false));
        assert!(sampler.should_record(2, false));
    }

    #[test]
    fn a_terminal_position_is_recorded_regardless_of_density() {
        let sampler = Sampler::new(SamplingConfig::default());
        assert!(sampler.should_record(7, true), "ply 7 is not on the stride");
    }

    #[test]
    fn edges_are_trimmed_by_visits_not_by_order() {
        let sampler = Sampler::new(SamplingConfig {
            max_edges_per_position: 2,
            ..SamplingConfig::default()
        });

        let selected = sampler.select_edges(vec![stats(1), stats(500), stats(50)]);

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].visits, 500);
        assert_eq!(selected[1].visits, 50);
    }

    #[test]
    fn disabling_child_stats_stores_no_edges() {
        let sampler = Sampler::new(SamplingConfig {
            store_child_stats: false,
            ..SamplingConfig::default()
        });

        assert!(sampler.select_edges(vec![stats(500)]).is_empty());
    }
}

//! The circuit breaker — §7.8.
//!
//! Commentary is optional; gameplay is not. Without this, a model that has
//! begun timing out imposes its full deadline on every subsequent request —
//! 2.5 s per move, forever, plus the CPU burned producing output that is then
//! thrown away. The breaker converts a persistent fault into a single cheap
//! decision: one atomic load per request while it is open.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use serde::Serialize;

use super::FaceError;
use crate::config::CircuitBreakerConfig;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitState {
    /// The `face_events.circuit_state` encoding (§12).
    #[must_use]
    pub fn as_i64(self) -> i64 {
        match self {
            Self::Closed => 0,
            Self::Open => 1,
            Self::HalfOpen => 2,
        }
    }

    fn from_u8(raw: u8) -> Self {
        match raw {
            1 => Self::Open,
            2 => Self::HalfOpen,
            _ => Self::Closed,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Admission {
    /// Go to the primary adapter.
    Allow,
    /// Serve canned. No inference is attempted.
    ShortCircuit,
}

/// Monotonic milliseconds since process start.
///
/// A trait, and injected, so that §20.7 can advance past a 300-second cooldown
/// without any test sleeping for five minutes.
pub trait MonotonicClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            origin: Instant::now(),
        })
    }
}

impl MonotonicClock for SystemClock {
    fn now_ms(&self) -> u64 {
        self.origin.elapsed().as_millis() as u64
    }
}

pub struct CircuitBreaker {
    state: AtomicU8,
    consecutive_failures: AtomicU32,
    opened_at_ms: AtomicU64,
    /// Single-probe admission: at most one request enters a half-open circuit,
    /// however many arrive at once.
    half_open_token: AtomicBool,
    /// Set when the model could not be loaded at all. Suppresses the cooldown
    /// entirely, so `/health` keeps reporting `open` rather than drifting to
    /// `half_open` on a probe that can never be admitted.
    permanently_open: AtomicBool,
    failure_threshold: u32,
    cooldown_ms: u64,
    trips: AtomicU64,
    short_circuited: AtomicU64,
}

impl CircuitBreaker {
    #[must_use]
    pub fn new(config: &CircuitBreakerConfig) -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(CircuitState::Closed as u8),
            consecutive_failures: AtomicU32::new(0),
            opened_at_ms: AtomicU64::new(0),
            half_open_token: AtomicBool::new(true),
            permanently_open: AtomicBool::new(false),
            failure_threshold: config.failure_threshold.max(1),
            cooldown_ms: config.cooldown_seconds.saturating_mul(1000),
            trips: AtomicU64::new(0),
            short_circuited: AtomicU64::new(0),
        })
    }

    #[must_use]
    pub fn state(&self) -> CircuitState {
        CircuitState::from_u8(self.state.load(Ordering::Acquire))
    }

    #[must_use]
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn trips_total(&self) -> u64 {
        self.trips.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn short_circuited_total(&self) -> u64 {
        self.short_circuited.load(Ordering::Relaxed)
    }

    /// Seconds until the circuit becomes half-open. `0` when it is not open.
    #[must_use]
    pub fn cooldown_remaining_seconds(&self, now_ms: u64) -> u64 {
        if self.state() != CircuitState::Open {
            return 0;
        }
        let elapsed = now_ms.saturating_sub(self.opened_at_ms.load(Ordering::Acquire));
        self.cooldown_ms.saturating_sub(elapsed) / 1000
    }

    /// Cheap enough to call on every commentary request.
    pub fn admit(&self, now_ms: u64) -> Admission {
        if self.permanently_open.load(Ordering::Acquire) {
            self.short_circuited.fetch_add(1, Ordering::Relaxed);
            return Admission::ShortCircuit;
        }

        match self.state() {
            CircuitState::Closed => Admission::Allow,
            CircuitState::Open => {
                let opened_at = self.opened_at_ms.load(Ordering::Acquire);
                if now_ms.saturating_sub(opened_at) >= self.cooldown_ms {
                    self.state
                        .store(CircuitState::HalfOpen as u8, Ordering::Release);
                    self.try_take_probe_token()
                } else {
                    self.short_circuited.fetch_add(1, Ordering::Relaxed);
                    Admission::ShortCircuit
                }
            }
            CircuitState::HalfOpen => self.try_take_probe_token(),
        }
    }

    pub fn on_success(&self) {
        // A request admitted while `Closed` can still be in flight when three
        // other requests trip the circuit to `Open`; its eventual success is
        // stale and must not force the circuit shut ahead of its cooldown —
        // only a request that actually observed `HalfOpen` (the admitted
        // probe) may close it. `Closed` handles the ordinary case; `Open`
        // silently drops the stale result.
        if self.state() == CircuitState::Open {
            return;
        }
        self.consecutive_failures.store(0, Ordering::Release);
        self.state
            .store(CircuitState::Closed as u8, Ordering::Release);
        self.half_open_token.store(true, Ordering::Release);
    }

    pub fn on_failure(&self, now_ms: u64, error: &FaceError) {
        // §7.8.3. Saturation and a disabled layer are not faults, and counting
        // them would trip the breaker under exactly the load it exists for.
        if !error.counts_toward_trip() {
            return;
        }

        let failures = self.consecutive_failures.fetch_add(1, Ordering::AcqRel) + 1;
        let probe_failed = self.state() == CircuitState::HalfOpen;

        if failures >= self.failure_threshold || probe_failed {
            self.opened_at_ms.store(now_ms, Ordering::Release);
            self.state
                .store(CircuitState::Open as u8, Ordering::Release);
            // A reopened circuit must be probeable again after its next
            // cooldown. Only `open_permanently` withholds the token — leaving
            // it spent here would strand the circuit in `HalfOpen` forever
            // after the first failed probe, since only `on_success` restores
            // it and no further probe would ever be admitted to succeed.
            if !self.permanently_open.load(Ordering::Acquire) {
                self.half_open_token.store(true, Ordering::Release);
            }
            self.trips.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Open the circuit and leave it open.
    ///
    /// Used when the model could not be loaded at startup — including a CUDA
    /// OOM. The service runs on canned lines and says so on `/health`, rather
    /// than retrying a load that will not start succeeding (§17.2).
    pub fn open_permanently(&self, now_ms: u64) {
        self.opened_at_ms.store(now_ms, Ordering::Release);
        self.state
            .store(CircuitState::Open as u8, Ordering::Release);
        self.half_open_token.store(false, Ordering::Release);
        self.permanently_open.store(true, Ordering::Release);
        self.trips.fetch_add(1, Ordering::Relaxed);
    }

    fn try_take_probe_token(&self) -> Admission {
        if self
            .half_open_token
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            Admission::Allow
        } else {
            self.short_circuited.fetch_add(1, Ordering::Relaxed);
            Admission::ShortCircuit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaker() -> Arc<CircuitBreaker> {
        CircuitBreaker::new(&CircuitBreakerConfig::default())
    }

    /// §20.7: two consecutive failures leave the circuit closed; the third
    /// opens it.
    #[test]
    fn three_consecutive_failures_open_the_circuit() {
        let breaker = breaker();

        breaker.on_failure(0, &FaceError::Timeout);
        assert_eq!(breaker.state(), CircuitState::Closed);
        breaker.on_failure(1, &FaceError::Timeout);
        assert_eq!(breaker.state(), CircuitState::Closed);
        breaker.on_failure(2, &FaceError::Timeout);
        assert_eq!(breaker.state(), CircuitState::Open);
        assert_eq!(breaker.trips_total(), 1);
    }

    #[test]
    fn a_success_resets_the_failure_count_to_zero() {
        let breaker = breaker();

        breaker.on_failure(0, &FaceError::Timeout);
        breaker.on_failure(1, &FaceError::Timeout);
        breaker.on_success();
        breaker.on_failure(2, &FaceError::Timeout);
        breaker.on_failure(3, &FaceError::Timeout);

        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    /// §7.8.3: saturation is expected backpressure under lab load, and a
    /// hundred of them must not trip anything.
    #[test]
    fn saturation_never_trips_the_breaker() {
        let breaker = breaker();
        for tick in 0..100 {
            breaker.on_failure(tick, &FaceError::Saturated);
            breaker.on_failure(tick, &FaceError::Disabled);
        }
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.trips_total(), 0);
    }

    /// §20.7, with an injected clock. No test sleeps for five minutes.
    #[test]
    fn the_cooldown_admits_exactly_one_probe() {
        let breaker = breaker();
        for tick in 0..3 {
            breaker.on_failure(tick, &FaceError::Timeout);
        }

        assert_eq!(breaker.admit(1_000), Admission::ShortCircuit);
        assert_eq!(breaker.cooldown_remaining_seconds(1_000), 299);

        // A hundred simultaneous requests past the cooldown produce one probe.
        let after_cooldown = 300_100;
        let admitted = (0..100)
            .filter(|_| breaker.admit(after_cooldown) == Admission::Allow)
            .count();
        assert_eq!(admitted, 1);
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn a_successful_probe_closes_the_circuit() {
        let breaker = breaker();
        for tick in 0..3 {
            breaker.on_failure(tick, &FaceError::Timeout);
        }

        assert_eq!(breaker.admit(300_100), Admission::Allow);
        breaker.on_success();

        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.admit(300_101), Admission::Allow);
    }

    #[test]
    fn a_failed_probe_reopens_with_a_fresh_cooldown() {
        let breaker = breaker();
        for tick in 0..3 {
            breaker.on_failure(tick, &FaceError::Timeout);
        }

        assert_eq!(breaker.admit(300_100), Admission::Allow);
        breaker.on_failure(300_101, &FaceError::Timeout);

        assert_eq!(breaker.state(), CircuitState::Open);
        assert_eq!(breaker.trips_total(), 2);
        assert_eq!(breaker.admit(300_102), Admission::ShortCircuit);
        assert_eq!(breaker.cooldown_remaining_seconds(300_102), 299);
    }

    /// §7.8: a success from a request admitted before the circuit tripped
    /// must not be able to force it closed once it has reopened — only the
    /// admitted probe's own outcome may do that.
    #[test]
    fn a_stale_success_does_not_close_an_open_circuit() {
        let breaker = breaker();
        for tick in 0..3 {
            breaker.on_failure(tick, &FaceError::Timeout);
        }
        assert_eq!(breaker.state(), CircuitState::Open);

        // A success arrives after the trip, from a request admitted earlier.
        breaker.on_success();

        assert_eq!(breaker.state(), CircuitState::Open);
        assert_eq!(
            breaker.admit(1),
            Admission::ShortCircuit,
            "cooldown must still apply"
        );
    }

    /// A reopened (but not permanently open) circuit must recover: the second
    /// cooldown must produce a second probe, not permanent silence.
    #[test]
    fn a_reopened_circuit_admits_a_probe_after_the_next_cooldown() {
        let breaker = breaker();
        for tick in 0..3 {
            breaker.on_failure(tick, &FaceError::Timeout);
        }

        assert_eq!(breaker.admit(300_100), Admission::Allow);
        breaker.on_failure(300_101, &FaceError::Timeout);

        assert_eq!(breaker.admit(300_102), Admission::ShortCircuit);
        assert_eq!(breaker.admit(600_200), Admission::Allow);
        breaker.on_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    /// §17.2: a model that could not load at all does not get probed back to
    /// life on a timer.
    #[test]
    fn a_permanently_open_circuit_never_admits_a_probe() {
        let breaker = breaker();
        breaker.open_permanently(0);

        assert_eq!(breaker.state(), CircuitState::Open);
        assert_eq!(breaker.admit(0), Admission::ShortCircuit);
        assert_eq!(breaker.admit(600_000), Admission::ShortCircuit);
        assert_eq!(breaker.admit(86_400_000), Admission::ShortCircuit);
        assert_eq!(
            breaker.state(),
            CircuitState::Open,
            "/health must keep saying open, not drift to half_open"
        );
    }
}

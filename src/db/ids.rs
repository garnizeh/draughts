//! Monotonic id allocation — §15.2.
//!
//! Lab workers pre-assign `games.id` and `positions.id` from a lease, which is
//! what makes it possible to build `position_edges` rows before their parent
//! rows have been committed. Without this the edges would have to wait for a
//! round trip per position, and the batching actor would have nothing to batch.

use std::sync::atomic::{AtomicI64, Ordering};

/// A block of ids owned exclusively by one worker.
///
/// Deliberately not `Copy`: a lease that could be implicitly duplicated could
/// have `take()` called on both copies, issuing the same id twice and
/// producing a primary-key collision at insert time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdLease {
    next: i64,
    end: i64,
}

impl IdLease {
    #[must_use]
    pub fn take(&mut self) -> Option<i64> {
        if self.next >= self.end {
            return None;
        }
        let id = self.next;
        self.next += 1;
        Some(id)
    }

    #[must_use]
    pub fn remaining(&self) -> i64 {
        self.end - self.next
    }
}

/// Hands out disjoint leases. Seeded from `MAX(id)` at startup (§22.3 step 3),
/// so that a restart never reissues a used id.
pub struct IdAllocator {
    next: AtomicI64,
}

impl IdAllocator {
    /// `resume_from` is `MAX(id) + 1` for the table this allocator serves.
    #[must_use]
    pub fn resume_from(next: i64) -> Self {
        Self {
            next: AtomicI64::new(next),
        }
    }

    pub fn lease(&self, count: i64) -> IdLease {
        assert!(count > 0, "a lease of zero ids is a bug at the call site");
        let start = self.next.fetch_add(count, Ordering::Relaxed);
        IdLease {
            next: start,
            end: start + count,
        }
    }

    #[must_use]
    pub fn peek(&self) -> i64 {
        self.next.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lease_hands_out_exactly_its_block() {
        let allocator = IdAllocator::resume_from(100);
        let mut lease = allocator.lease(3);

        assert_eq!(lease.take(), Some(100));
        assert_eq!(lease.take(), Some(101));
        assert_eq!(lease.take(), Some(102));
        assert_eq!(lease.take(), None, "a lease does not overrun");
    }

    /// §20.6: concurrent leases produce disjoint ranges.
    #[test]
    fn concurrent_leases_are_disjoint() {
        use std::sync::Arc;

        let allocator = Arc::new(IdAllocator::resume_from(1));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let allocator = Arc::clone(&allocator);
                std::thread::spawn(move || {
                    let mut lease = allocator.lease(1_000);
                    std::iter::from_fn(move || lease.take()).collect::<Vec<_>>()
                })
            })
            .collect();

        let mut seen = std::collections::HashSet::new();
        for thread in threads {
            for id in thread.join().expect("worker finished") {
                assert!(seen.insert(id), "id {id} was issued twice");
            }
        }
        assert_eq!(seen.len(), 8_000);
    }

    /// §20.6: `resume_from` after a crash never reissues a used id.
    #[test]
    fn resuming_never_reissues() {
        let first = IdAllocator::resume_from(1);
        let mut lease = first.lease(500);
        while lease.take().is_some() {}
        let high_water = first.peek();

        let after_restart = IdAllocator::resume_from(high_water);
        let mut lease = after_restart.lease(1);

        assert_eq!(lease.take(), Some(501));
    }
}

//! Hybrid Logical Clock (HLC) for causal ordering.
//!
//! The HLC combines a physical wall-clock component (milliseconds since
//! epoch) with a logical counter so that:
//!
//! 1. Timestamps are **monotonically increasing** even when the wall clock
//!    is stale or drifts backward.
//! 2. Causally-related events are always ordered correctly.
//! 3. No floating-point arithmetic is involved.

use crate::{
    error::{ExoError, Result},
    types::Timestamp,
};

/// Maximum tolerable forward drift in milliseconds.
/// If a remote timestamp is more than this far ahead of our wall clock
/// we reject it as drift.
const MAX_DRIFT_MS: u64 = 60_000; // 60 seconds

/// A Hybrid Logical Clock instance.
///
/// Each node in the EXOCHAIN network maintains its own `HybridClock`.
/// The clock is driven by a wall-clock source (injectable for testing)
/// and a logical counter.
pub struct HybridClock {
    /// Last-known physical time in milliseconds since epoch.
    physical: u64,
    /// Logical counter within the same physical millisecond.
    logical: u32,
    /// Wall-clock source — returns current millis since epoch.
    wall_clock: Box<dyn Fn() -> Result<u64> + Send>,
}

impl HybridClock {
    /// Create a new clock driven by the system wall clock.
    #[must_use]
    pub fn new() -> Self {
        Self {
            physical: 0,
            logical: 0,
            wall_clock: Box::new(system_time_millis),
        }
    }

    /// Create a clock with a custom wall-clock source (for testing).
    #[must_use]
    pub fn with_wall_clock(wall_clock: impl Fn() -> u64 + Send + 'static) -> Self {
        Self {
            physical: 0,
            logical: 0,
            wall_clock: Box::new(move || Ok(wall_clock())),
        }
    }

    /// Create a clock with a fallible wall-clock source.
    #[must_use]
    pub fn with_fallible_wall_clock(wall_clock: impl Fn() -> Result<u64> + Send + 'static) -> Self {
        Self {
            physical: 0,
            logical: 0,
            wall_clock: Box::new(wall_clock),
        }
    }

    /// Generate the next timestamp.
    ///
    /// Guarantees: the returned timestamp is strictly greater than any
    /// previously returned by this clock.
    pub fn now(&mut self) -> Result<Timestamp> {
        let wall = (self.wall_clock)()?;
        if wall > self.physical {
            self.physical = wall;
            self.logical = 0;
        } else {
            advance_logical_or_carry_physical(&mut self.physical, &mut self.logical)?;
        }
        Ok(Timestamp::new(self.physical, self.logical))
    }

    /// Merge a remote timestamp and advance the local clock.
    ///
    /// The returned timestamp is guaranteed to be greater than both the
    /// local state and the remote timestamp.
    ///
    /// # Errors
    ///
    /// Returns `ExoError::ClockDrift` if the remote timestamp is
    /// unreasonably far ahead of the local wall clock.
    pub fn update(&mut self, remote: &Timestamp) -> Result<Timestamp> {
        let wall = (self.wall_clock)()?;

        // Drift guard
        if remote.physical_ms > wall.saturating_add(MAX_DRIFT_MS) {
            return Err(ExoError::ClockDrift {
                physical_ms: remote.physical_ms,
                tolerance_ms: MAX_DRIFT_MS,
            });
        }

        if wall > self.physical && wall > remote.physical_ms {
            // Wall clock is ahead of both — reset logical
            self.physical = wall;
            self.logical = 0;
        } else if self.physical == remote.physical_ms {
            // Same physical — advance logical past both
            self.logical = self.logical.max(remote.logical);
            advance_logical_or_carry_physical(&mut self.physical, &mut self.logical)?;
        } else if remote.physical_ms > self.physical {
            // Remote is ahead — adopt remote physical, advance logical
            self.physical = remote.physical_ms;
            self.logical = remote.logical;
            advance_logical_or_carry_physical(&mut self.physical, &mut self.logical)?;
        } else {
            // Local is ahead — advance own logical
            advance_logical_or_carry_physical(&mut self.physical, &mut self.logical)?;
        }

        Ok(Timestamp::new(self.physical, self.logical))
    }

    /// Causal ordering check: returns `true` if `a` happened-before `b`.
    #[must_use]
    pub fn is_before(a: &Timestamp, b: &Timestamp) -> bool {
        a < b
    }

    /// Return the current state as a `Timestamp` without advancing.
    #[must_use]
    pub fn current(&self) -> Timestamp {
        Timestamp::new(self.physical, self.logical)
    }
}

fn advance_logical_or_carry_physical(physical: &mut u64, logical: &mut u32) -> Result<()> {
    if *logical == u32::MAX {
        if let Some(next_physical) = physical.checked_add(1) {
            *physical = next_physical;
            *logical = 0;
            Ok(())
        } else {
            Err(ExoError::ClockOverflow {
                physical_ms: *physical,
                logical: *logical,
            })
        }
    } else {
        *logical += 1;
        Ok(())
    }
}

impl Default for HybridClock {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for HybridClock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HybridClock")
            .field("physical", &self.physical)
            .field("logical", &self.logical)
            .finish()
    }
}

/// Default wall-clock implementation.
#[cfg(not(target_arch = "wasm32"))]
fn system_time_millis() -> Result<u64> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration =
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ExoError::ClockUnavailable {
                reason: "system time is before the Unix epoch".into(),
            })?;

    u64::try_from(duration.as_millis()).map_err(|_| ExoError::ClockUnavailable {
        reason: "system time milliseconds exceed u64".into(),
    })
}

/// WASM wall-clock: route through js_sys::Date::now().
#[cfg(target_arch = "wasm32")]
fn system_time_millis() -> Result<u64> {
    let millis = js_sys::Date::now();
    if !millis.is_finite() || millis.is_sign_negative() {
        return Err(ExoError::ClockUnavailable {
            reason: "Date.now returned a non-finite or negative value".into(),
        });
    }
    millis
        .to_string()
        .parse::<u64>()
        .map_err(|_| ExoError::ClockUnavailable {
            reason: "Date.now milliseconds cannot be represented as u64".into(),
        })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    use super::*;

    /// Helper: create a clock with a controllable wall time.
    fn test_clock(initial: u64) -> (HybridClock, Arc<AtomicU64>) {
        let time = Arc::new(AtomicU64::new(initial));
        let t = Arc::clone(&time);
        let clock = HybridClock::with_wall_clock(move || t.load(Ordering::Relaxed));
        (clock, time)
    }

    #[test]
    fn now_monotonic_same_wall_time() {
        let (mut clock, _wall) = test_clock(1000);
        let t1 = clock.now().expect("HLC timestamp");
        let t2 = clock.now().expect("HLC timestamp");
        let t3 = clock.now().expect("HLC timestamp");
        assert!(t1 < t2);
        assert!(t2 < t3);
        // All share the same physical
        assert_eq!(t1.physical_ms, 1000);
        assert_eq!(t2.physical_ms, 1000);
        assert_eq!(t3.physical_ms, 1000);
        // Logical increments
        assert_eq!(t1.logical, 0);
        assert_eq!(t2.logical, 1);
        assert_eq!(t3.logical, 2);
    }

    #[test]
    fn now_advances_with_wall_clock() {
        let (mut clock, wall) = test_clock(1000);
        let t1 = clock.now().expect("HLC timestamp");
        wall.store(2000, Ordering::Relaxed);
        let t2 = clock.now().expect("HLC timestamp");
        assert!(t1 < t2);
        assert_eq!(t2.physical_ms, 2000);
        assert_eq!(t2.logical, 0);
    }

    #[test]
    fn now_handles_backward_wall_clock() {
        let (mut clock, wall) = test_clock(2000);
        let t1 = clock.now().expect("HLC timestamp");
        wall.store(1000, Ordering::Relaxed); // wall goes backward
        let t2 = clock.now().expect("HLC timestamp");
        assert!(t1 < t2);
        // Physical stays at 2000, logical increments
        assert_eq!(t2.physical_ms, 2000);
        assert_eq!(t2.logical, 1);
    }

    #[test]
    fn update_wall_ahead_of_both() {
        let (mut clock, wall) = test_clock(1000);
        let _ = clock.now().expect("HLC timestamp");
        wall.store(5000, Ordering::Relaxed);
        let remote = Timestamp::new(3000, 5);
        let result = clock.update(&remote).expect("ok");
        assert_eq!(result.physical_ms, 5000);
        assert_eq!(result.logical, 0);
    }

    #[test]
    fn update_remote_ahead() {
        let (mut clock, _wall) = test_clock(1000);
        let _ = clock.now().expect("HLC timestamp");
        let remote = Timestamp::new(2000, 10);
        let result = clock.update(&remote).expect("ok");
        assert_eq!(result.physical_ms, 2000);
        assert_eq!(result.logical, 11);
    }

    #[test]
    fn update_same_physical() {
        let (mut clock, _wall) = test_clock(1000);
        let _ = clock.now().expect("HLC timestamp"); // physical=1000, logical=0
        let remote = Timestamp::new(1000, 5);
        let result = clock.update(&remote).expect("ok");
        assert_eq!(result.physical_ms, 1000);
        // max(local_logical=0, remote_logical=5) + 1 = 6
        assert_eq!(result.logical, 6);
    }

    #[test]
    fn update_local_ahead() {
        let (mut clock, wall) = test_clock(3000);
        let _ = clock.now().expect("HLC timestamp"); // physical=3000, logical=0
        wall.store(1000, Ordering::Relaxed); // wall backward
        let remote = Timestamp::new(2000, 0);
        let result = clock.update(&remote).expect("ok");
        // Local physical (3000) > remote (2000), local advances logical
        assert_eq!(result.physical_ms, 3000);
        assert_eq!(result.logical, 1);
    }

    #[test]
    fn update_rejects_excessive_drift() {
        let (mut clock, _wall) = test_clock(1000);
        let remote = Timestamp::new(1000 + MAX_DRIFT_MS + 1, 0);
        let err = clock.update(&remote).unwrap_err();
        assert!(matches!(err, ExoError::ClockDrift { .. }));
    }

    #[test]
    fn update_accepts_at_drift_boundary() {
        let (mut clock, _wall) = test_clock(1000);
        let remote = Timestamp::new(1000 + MAX_DRIFT_MS, 0);
        let result = clock.update(&remote);
        assert!(result.is_ok());
    }

    #[test]
    fn is_before_ordering() {
        let a = Timestamp::new(1, 0);
        let b = Timestamp::new(1, 1);
        let c = Timestamp::new(2, 0);
        assert!(HybridClock::is_before(&a, &b));
        assert!(HybridClock::is_before(&b, &c));
        assert!(HybridClock::is_before(&a, &c));
        assert!(!HybridClock::is_before(&b, &a));
        assert!(!HybridClock::is_before(&a, &a));
    }

    #[test]
    fn current_does_not_advance() {
        let (mut clock, _wall) = test_clock(1000);
        let _ = clock.now().expect("HLC timestamp");
        let c1 = clock.current();
        let c2 = clock.current();
        assert_eq!(c1, c2);
    }

    #[test]
    fn debug_format() {
        let (clock, _wall) = test_clock(42);
        let dbg = format!("{clock:?}");
        assert!(dbg.contains("HybridClock"));
    }

    #[test]
    fn default_clock() {
        let mut clock = HybridClock::default();
        let t = clock.now().expect("HLC timestamp");
        // Should have a reasonable physical time (non-zero)
        assert!(t.physical_ms > 0);
    }

    #[test]
    fn concurrent_updates_maintain_monotonicity() {
        let (mut clock, wall) = test_clock(100);
        let _ = clock.now().expect("HLC timestamp");

        // Simulate multiple rapid remote updates
        let remotes = [
            Timestamp::new(100, 3),
            Timestamp::new(100, 1),
            Timestamp::new(100, 7),
            Timestamp::new(100, 2),
        ];

        let mut last = clock.current();
        for r in &remotes {
            let ts = clock.update(r).expect("ok");
            assert!(ts > last, "monotonicity violated: {ts:?} <= {last:?}");
            last = ts;
        }

        // Then advance wall clock
        wall.store(200, Ordering::Relaxed);
        let ts = clock.now().expect("HLC timestamp");
        assert!(ts > last);
        assert_eq!(ts.physical_ms, 200);
        assert_eq!(ts.logical, 0);
    }

    #[test]
    fn now_remains_monotonic_when_logical_counter_is_exhausted() {
        let (mut clock, _wall) = test_clock(1000);
        clock.physical = 1000;
        clock.logical = u32::MAX;

        let ts = clock.now().expect("HLC timestamp");

        assert!(ts > Timestamp::new(1000, u32::MAX));
        assert_eq!(ts.physical_ms, 1001);
        assert_eq!(ts.logical, 0);
    }

    #[test]
    fn update_remains_monotonic_when_logical_counter_is_exhausted() {
        let (mut clock, _wall) = test_clock(1000);
        clock.physical = 1000;
        clock.logical = u32::MAX;

        let ts = clock.update(&Timestamp::new(1000, u32::MAX)).expect("ok");

        assert!(ts > Timestamp::new(1000, u32::MAX));
        assert_eq!(ts.physical_ms, 1001);
        assert_eq!(ts.logical, 0);
    }

    #[test]
    fn now_rejects_terminal_clock_exhaustion_without_reusing_timestamp() {
        let (mut clock, _wall) = test_clock(u64::MAX);
        clock.physical = u64::MAX;
        clock.logical = u32::MAX;

        let err = clock
            .now()
            .expect_err("terminal HLC state must fail closed");

        assert!(matches!(
            err,
            ExoError::ClockOverflow {
                physical_ms: u64::MAX,
                logical: u32::MAX
            }
        ));
        assert_eq!(clock.current(), Timestamp::new(u64::MAX, u32::MAX));
    }

    #[test]
    fn update_rejects_terminal_clock_exhaustion_without_reusing_timestamp() {
        let (mut clock, _wall) = test_clock(u64::MAX);
        clock.physical = u64::MAX;
        clock.logical = u32::MAX;

        let err = clock
            .update(&Timestamp::new(u64::MAX, u32::MAX))
            .expect_err("terminal HLC update must fail closed");

        assert!(matches!(
            err,
            ExoError::ClockOverflow {
                physical_ms: u64::MAX,
                logical: u32::MAX
            }
        ));
        assert_eq!(clock.current(), Timestamp::new(u64::MAX, u32::MAX));
    }

    #[test]
    fn now_propagates_wall_clock_error_without_mutating_state() {
        let mut clock = HybridClock::with_fallible_wall_clock(|| {
            Err(ExoError::ClockUnavailable {
                reason: "injected wall-clock failure".into(),
            })
        });

        let err = clock
            .now()
            .expect_err("wall-clock failures must fail closed");

        assert!(matches!(err, ExoError::ClockUnavailable { .. }));
        assert_eq!(clock.current(), Timestamp::new(0, 0));
    }

    #[test]
    fn update_propagates_wall_clock_error_without_mutating_state() {
        let calls = Arc::new(AtomicU64::new(0));
        let calls_for_clock = Arc::clone(&calls);
        let mut clock = HybridClock::with_fallible_wall_clock(move || {
            if calls_for_clock.fetch_add(1, Ordering::Relaxed) == 0 {
                Ok(1000)
            } else {
                Err(ExoError::ClockUnavailable {
                    reason: "injected wall-clock failure".into(),
                })
            }
        });
        let first = clock.now().expect("first timestamp");

        let err = clock
            .update(&Timestamp::new(1000, 0))
            .expect_err("wall-clock failures must fail closed");

        assert!(matches!(err, ExoError::ClockUnavailable { .. }));
        assert_eq!(clock.current(), first);
    }

    #[test]
    fn system_time_millis_returns_nonzero() {
        let ms = system_time_millis().expect("system clock should be available");
        assert!(ms > 0);
    }

    #[test]
    fn default_wall_clock_source_has_no_epoch_zero_fallback() {
        let production = include_str!("hlc.rs")
            .split("// ===========================================================================")
            .next()
            .expect("production section");

        assert!(
            !production.contains(".unwrap_or(0)"),
            "HLC wall-clock failures must propagate instead of silently using epoch zero"
        );
    }

    #[test]
    fn wasm_clock_source_uses_checked_date_now_conversion() {
        let source = include_str!("hlc.rs");
        let wasm_clock_source = source
            .split("/// WASM wall-clock: route through js_sys::Date::now().")
            .nth(1)
            .expect("WASM clock source exists")
            .split("// ===========================================================================")
            .next()
            .expect("WASM clock source ends before tests");

        assert!(
            !wasm_clock_source.contains("clippy::as_conversions"),
            "WASM HLC clock conversion must not suppress checked conversion lints"
        );
        assert!(
            !wasm_clock_source.contains("Date::now() as u64"),
            "WASM HLC clock conversion must not use lossy float-to-integer casts"
        );
    }
}

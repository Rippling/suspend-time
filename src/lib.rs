//! A library to solve platform inconsistencies in the standard time library,
//! specifically surrounding system suspension.  See [`SuspendUnawareInstant`]
//! for more details
use std::{
    ops::{Add, Sub},
    time::Duration,
};

mod platform;

const NANOS_PER_SECOND: u32 = 1_000_000_000;

/// Similar to the standard library's implementation of
/// [`Instant`](https://doc.rust-lang.org/1.78.0/std/time/struct.Instant.html),
/// except it is consistently unaware of system suspends across all platforms
/// supported by this library.
///
/// Historically, this has been inconsistent in the standard library, with
/// windows allowing time to pass when the system is suspended/hibernating,
/// however unix systems do not "pass time" during system suspension. In this
/// library, time **never passes** when the system is suspended on **any
/// platform**.
///
/// This instant implementation is:
/// - Opaque (you cannot manually create an Instant. You must call ::now())
/// - Cross platform (windows, macOS)
/// - Monotonic (time never goes backwards)
/// - Suspend-unaware (when you put your computer to sleep, "time" does not pass.)
///
/// # Undefined behavior / Invariants
/// 1. When polling the system clock, nanoseconds should never exceed 10^9 (the number of nanoseconds in 1 second).
///    If this happens, we simply return zero. The standard library has a similar invariant (0 <= nanos <= 10^9), but handles it differently.
/// 2. If an instant in the future is subtracted from an instant in the past, we return a Duration of 0.
/// 3. If a duration is subtracted that would cause an instant to be negative, we return an instant set at 0.
/// 4. If a duration is added to an instant that would cause the instant to exceed 2^64 seconds, we return an instant set to 0.
///
/// # Underlying System calls
///
/// The following system calls are currently being used by `now()` to find out
/// the current time:
///
/// |  Platform |               System call                               |
/// |-----------|---------------------------------------------------------|
/// | UNIX      | [clock_gettime] (CLOCK_UPTIME_RAW)                      |
/// | Darwin    | [clock_gettime] (CLOCK_UPTIME_RAW)                      |
/// | VXWorks   | [clock_gettime] (CLOCK_UPTIME_RAW)                      |
/// | Windows   | [QueryUnbiasedInterruptTimePrecise]                     |
///
/// [clock_gettime]: https://www.manpagez.com/man/3/clock_gettime/
/// [QueryUnbiasedInterruptTimePrecise]:
/// https://learn.microsoft.com/en-us/windows/win32/api/realtimeapiset/nf-realtimeapiset-queryunbiasedinterrupttimeprecise
///
/// Certain overflows are dependent on how the standard library implements
/// Duration.  For example, right now it is implemented as a u64 counting
/// seconds. As such, to prevent overflow we must check if the number of seconds
/// in two Durations exceeds the bounds of a u64.  To avoid being dependent on
/// the standard library for cases like this, we choose our own representation
/// of time which matches the "apple" libc platform implementation.
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct SuspendUnawareInstant {
    secs: u64,
    nanos: u32, // invariant: 0 <= self.nanos <= NANOS_PER_SECOND
}

impl SuspendUnawareInstant {
    /// Returns an instant corresponding to "now".
    ///
    /// # Examples
    ///
    /// ```
    /// use suspend_time::SuspendUnawareInstant;
    ///
    /// let now = SuspendUnawareInstant::now();
    /// ```
    pub fn now() -> SuspendUnawareInstant {
        platform::now()
    }

    /// Returns the amount of system unsuspended time elapsed since this suspend
    /// unaware instant was created, or zero duration if that this instant is in
    /// the future.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::{thread, time};
    /// use suspend_time::{SuspendUnawareInstant};
    ///
    /// let instant = SuspendUnawareInstant::now();
    /// let three_secs = time::Duration::from_secs(3);
    /// thread::sleep(three_secs);
    /// assert!(instant.elapsed() >= three_secs);
    /// ```
    pub fn elapsed(&self) -> Duration {
        Self::now() - *self
    }
}

impl Sub<SuspendUnawareInstant> for SuspendUnawareInstant {
    type Output = Duration;

    fn sub(self, rhs: SuspendUnawareInstant) -> Duration {
        if rhs > self {
            Duration::new(0, 0)
        } else {
            // The following operations are guaranteed to be valid, since we confirmed self >= rhs
            let diff_secs = self.secs - rhs.secs;
            if rhs.nanos > self.nanos {
                Duration::new(diff_secs - 1, NANOS_PER_SECOND + self.nanos - rhs.nanos)
            } else {
                Duration::new(diff_secs, self.nanos - rhs.nanos)
            }
        }
    }
}

// When adding/subtracting a `Duration` to/from a SuspendUnawareInstant, we want
// the result to be a new instant (point in time)

impl Sub<Duration> for SuspendUnawareInstant {
    type Output = SuspendUnawareInstant;

    fn sub(self, rhs: Duration) -> SuspendUnawareInstant {
        let rhs_secs = rhs.as_secs();
        let rhs_nanos = rhs.subsec_nanos();

        if self.secs.checked_sub(rhs_secs).is_none() {
            SuspendUnawareInstant { secs: 0, nanos: 0 }
        } else if rhs_nanos > self.nanos {
            // Since (self.secs - rhs_secs) passed, we know that self.secs >= rhs_secs.
            // The only case in which rhs_nanos > self.nanos is a problem is
            // when self.secs == rhs_secs, since this will cause the instant
            // to be "negative".
            if self.secs == rhs_secs {
                SuspendUnawareInstant { secs: 0, nanos: 0 }
            } else {
                SuspendUnawareInstant {
                    secs: self.secs - rhs_secs - 1,
                    nanos: (NANOS_PER_SECOND + self.nanos) - rhs_nanos,
                }
            }
        } else {
            SuspendUnawareInstant {
                secs: self.secs - rhs_secs,
                nanos: self.nanos - rhs_nanos,
            }
        }
    }
}

impl Add<Duration> for SuspendUnawareInstant {
    type Output = SuspendUnawareInstant;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Duration) -> SuspendUnawareInstant {
        let rhs_secs = rhs.as_secs();
        let rhs_nanos = rhs.subsec_nanos();

        if self.secs.checked_add(rhs_secs).is_none() {
            // undefined behavior, return 0
            SuspendUnawareInstant { secs: 0, nanos: 0 }
        } else {
            let nanos_carry = (self.nanos + rhs_nanos) / NANOS_PER_SECOND;
            // very pedantic edge case where the nanos pushed us over the
            // overflow limit. Nevertheless, we handle it.
            if (self.secs + rhs_secs)
                .checked_add(nanos_carry as u64)
                .is_none()
            {
                SuspendUnawareInstant { secs: 0, nanos: 0 }
            } else {
                SuspendUnawareInstant {
                    secs: self.secs + rhs_secs + (nanos_carry as u64),
                    nanos: (self.nanos + rhs_nanos) % NANOS_PER_SECOND,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{SuspendUnawareInstant, NANOS_PER_SECOND};
    use std::{cmp::Ordering, time::Duration};

    // Locally, this should pass with a 10ms tolerance. However, in circleci
    // this is flaky even at 100ms.
    const TOLERANCE_MS: u64 = 1000;
    const TOLERANCE_MS_U128: u128 = TOLERANCE_MS as u128;

    fn create_instant(secs: u64, nanos: u32) -> SuspendUnawareInstant {
        SuspendUnawareInstant { secs, nanos }
    }

    /// Testing that SuspendUnawareInstant is within a fixed tolerance of std's
    /// Instant. (see the tolerance variables) This is difficult since we cannot
    /// take both instants at the exact same time.
    #[test]
    fn accuracy() {
        let std_instant = std::time::Instant::now();
        let suspend_unaware_instant = SuspendUnawareInstant::now();
        let std_elapsed = std_instant.elapsed();
        let suspend_unaware_elapsed = suspend_unaware_instant.elapsed();
        assert!(
            std_elapsed
                .as_millis()
                .abs_diff(suspend_unaware_elapsed.as_millis())
                < TOLERANCE_MS_U128
        )
    }

    // This test is in fact different than the elapsed test, since we are
    // testing the Sub trait on SuspendUnawareInstant I actually ran into a case
    // where elapsed() was fine, but Sub was broken. Thus this test is necessary
    #[test]
    fn subtraction() {
        let a = SuspendUnawareInstant::now();
        std::thread::sleep(Duration::from_secs(1));
        let b = SuspendUnawareInstant::now();
        let dur = b - a;

        assert!(dur.as_millis().abs_diff(1000) < TOLERANCE_MS_U128);
    }

    // We should clamp to 0 if we subtract an instant from the future from an instant in the past
    #[test]
    fn incorrect_subtraction() {
        let start = SuspendUnawareInstant::now();
        std::thread::sleep(Duration::from_millis(10));
        let stop = SuspendUnawareInstant::now();

        let res = start - stop;
        assert_eq!(res, Duration::from_secs(0));
    }

    #[test]
    fn ordering_tests() {
        // (lhs, rhs, expected_result)
        #[rustfmt::skip]
        let cases = vec![
            (create_instant(0, 0), create_instant(0, 1), Ordering::Less),
            (create_instant(0, 1), create_instant(0, 2), Ordering::Less),
            (create_instant(0, 100), create_instant(1, 0), Ordering::Less),
            (create_instant(123, 456), create_instant(123, 456), Ordering::Equal),
            (create_instant(0, 1), create_instant(0, 0), Ordering::Greater),
            (create_instant(0, 2), create_instant(0, 1), Ordering::Greater),
            (create_instant(1, 0), create_instant(0, 100), Ordering::Greater),
        ];

        for (lhs, rhs, expected_result) in cases.iter() {
            assert_eq!(lhs.cmp(rhs), *expected_result);
        }
    }

    #[test]
    fn addition_tests() {
        // (lhs, rhs, expected_result)
        #[rustfmt::skip]
        let cases = vec![
            (create_instant(0, 0), Duration::new(0, 1), create_instant(0, 1)),
            (create_instant(0, 0), Duration::new(1, 0), create_instant(1, 0)),
            (create_instant(0, 0), Duration::new(u64::MAX, NANOS_PER_SECOND - 1), create_instant(u64::MAX, NANOS_PER_SECOND - 1)),
            (create_instant(1, 0), Duration::new(u64::MAX, 0), create_instant(0, 0)), // floor to 0 when out of bounds/overflow
            (create_instant(u64::MAX, 0), Duration::new(1, 0), create_instant(0, 0)), // floor to 0 when out of bounds/overflow
            (create_instant(u64::MAX, 1), Duration::new(0, NANOS_PER_SECOND - 1), create_instant(0, 0)), // literal edge case, where the nanoseconds push us over the boundary
            (create_instant(u64::MAX, 0), Duration::new(0, NANOS_PER_SECOND - 1), create_instant(u64::MAX, NANOS_PER_SECOND - 1)), // case where this is still valid, since we are just 1 nanosecond shy of going over
            (create_instant(0, NANOS_PER_SECOND - 1), Duration::from_secs(0) + Duration::from_nanos(10), create_instant(1, 9)), // testing nanosecond --> second carry.
        ];

        for (lhs, rhs, expected_result) in cases.iter() {
            assert_eq!((*lhs) + (*rhs), *expected_result);
        }
    }

    #[test]
    fn subtraction_duration_tests() {
        #[rustfmt::skip]
        let cases = vec![
            (create_instant(2, 0), Duration::new(1, 0), create_instant(1, 0)),
            (create_instant(2, 0), Duration::new(0, 1), create_instant(1, NANOS_PER_SECOND - 1)), // testing carry seconds --> nanos
            (create_instant(3, 10), Duration::new(1, 2), create_instant(2, 8)), // both fields lhs > rhs
            (create_instant(1, 0), Duration::new(2, 0), create_instant(0, 0)), // testing set to 0 on negative result
            (create_instant(1, 1), Duration::new(1, 2), create_instant(0, 0)), // testing set to 0 on negative result, this time with nanos causing the fault
        ];

        for (lhs, rhs, expected_result) in cases.iter() {
            assert_eq!((*lhs) - (*rhs), *expected_result);
        }
    }

    #[test]
    fn subtraction_instant_tests() {
        #[rustfmt::skip]
        let cases = [
            (create_instant(10, 5), create_instant(1, 2), Duration::new(9, 3)),
            (create_instant(1, 0), create_instant(2, 0), Duration::new(0, 0)), // seconds cause negative
            (create_instant(1, 0), create_instant(1, 1), Duration::new(0, 0)), // nanos cause negative
            (create_instant(2, 0), create_instant(0, 1), Duration::new(1, NANOS_PER_SECOND - 1)), // nano carry
        ];

        for (lhs, rhs, expected_result) in cases.iter() {
            assert_eq!((*lhs) - (*rhs), *expected_result);
        }
    }
}

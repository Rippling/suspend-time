//! Suspend-time is a cross-platform monotonic clock that is suspend-unaware, written in Rust!
//! It allows system suspension (e.g. when a user closes their laptop on windows) to not affect
//! `Instant` durations and timeouts!
//!
//! Example of using [`SuspendUnawareInstant`]:
//! ```
//! use std::{thread, time};
//! use suspend_time::{SuspendUnawareInstant};
//!
//! fn main() {
//!     // If you used std::time::Instant here and you suspend the system on windows,
//!     // it will print more than 3 seconds (circa July 2024).
//!     // With SuspendUnawareInstant this has no effect.
//!     let instant = SuspendUnawareInstant::now();
//!     let three_secs = time::Duration::from_secs(3);
//!     thread::sleep(three_secs);
//!     println!("{:#?}", instant.elapsed());
//! }
//! ```
//!
//! Example of using `suspend_time::`[`timeout`]
//!
//! ```
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     // If you suspend the system during main's execution, Tokio will time
//!     // out even though it only slept for 1 second. suspend_time::timeout does not.
//!     let _ = suspend_time::timeout(
//!         Duration::from_secs(2),
//!         suspend_time::sleep(Duration::from_secs(1)),
//!     ).await;
//! }
//! ```
//!
use std::{
    error::Error,
    fmt,
    future::Future,
    ops::{Add, Sub},
    time::Duration,
};

mod platform;
#[cfg(test)]
mod tests;

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
    /// let one_sec = time::Duration::from_secs(1);
    /// thread::sleep(one_sec);
    /// assert!(instant.elapsed() >= one_sec);
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

/// Suspend-time's equivalent of tokio's `tokio::time::error::Elapsed`.
/// Constructing the `Elapsed` struct is impossible due to its private construct
/// and private members. As such, we must create our own struct
#[derive(Clone, Debug, PartialEq)]
pub struct TimedOutError;

impl fmt::Display for TimedOutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Timed out")
    }
}

impl Error for TimedOutError {}

/// The same API as tokio::time::timeout, except it is uses on SuspendUnawareInstant for measuring time.
pub async fn timeout<'a, F>(duration: Duration, future: F) -> Result<F::Output, TimedOutError>
where
    F: Future + 'a,
{
    tokio::select! {
        _ = sleep(duration) => {
            Err(TimedOutError)
        }
        output = future => {
            Ok(output)
        }
    }
}

/// The same API as tokio::time::sleep, except it is uses on SuspendUnawareInstant for measuring time.
pub async fn sleep(duration: Duration) {
    let deadline = SuspendUnawareInstant::now() + duration;
    let mut now = SuspendUnawareInstant::now();
    while now < deadline {
        tokio::time::sleep(deadline - now).await;

        now = SuspendUnawareInstant::now();
    }
}

//! A library to solve platform inconsistencies in the standard time library, specifically surrounding system suspension.
//! See [`SuspendUnawareInstant`] for more details
use std::{
    ops::{Add, Sub},
    time::Duration,
};

mod platform;

/// Similar to the standard library's implementation of [`Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html), except it is consistently unaware of system suspends across all platforms supported by this library.
///
/// Historically, this has been inconsistent in the standard library, with windows allowing time to pass when the system is suspended/hibernating, however unix systems
/// do not "pass time" during system suspension. In this library, time **never passes** when the system is suspended on **any platform**.
///
/// This instant implementation is:
///  - Cross platform (windows, unix)
///  - Monotonic (time never goes backwards)
///  - Suspend-unaware (when you put your computer to sleep, "time" does not pass.)
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
/// [QueryUnbiasedInterruptTimePrecise]: https://learn.microsoft.com/en-us/windows/win32/api/realtimeapiset/nf-realtimeapiset-queryunbiasedinterrupttimeprecise
///
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct SuspendUnawareInstant {
    t: Duration,
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
        SuspendUnawareInstant { t: platform::now() }
    }

    /// Returns the amount of system unsuspended time elapsed since this suspend unaware instant was created,
    /// or zero duration if that this instant is in the future.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::{thread, time};
    /// use suspend_time::{SuspendUnawareInstant};
    ///
    /// fn main() {
    ///     let instant = SuspendUnawareInstant::now();
    ///     let three_secs = time::Duration::from_secs(3);
    ///     thread::sleep(three_secs);
    ///     assert!(instant.elapsed() >= three_secs);
    /// }
    /// ```
    pub fn elapsed(&self) -> Duration {
        Self::now().t - self.t
    }
}

// When taking the _difference_ between two `SuspendUnawareInstant`s we want the result to be a Duration,
// since it represents the duration between the two points in time

impl Sub<SuspendUnawareInstant> for SuspendUnawareInstant {
    type Output = Duration;

    fn sub(self, rhs: SuspendUnawareInstant) -> Duration {
        self.t - rhs.t
    }
}

// When adding/subtracting a `Duration` to/from a SuspendUnawareInstant, we want the result to be a new instant (point in time)

impl Sub<Duration> for SuspendUnawareInstant {
    type Output = SuspendUnawareInstant;

    fn sub(self, rhs: Duration) -> SuspendUnawareInstant {
        SuspendUnawareInstant { t: self.t - rhs }
    }
}

impl Add<Duration> for SuspendUnawareInstant {
    type Output = SuspendUnawareInstant;

    fn add(self, rhs: Duration) -> SuspendUnawareInstant {
        SuspendUnawareInstant { t: self.t + rhs }
    }
}

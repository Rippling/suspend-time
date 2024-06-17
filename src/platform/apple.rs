use crate::SuspendUnawareInstant;
use libc::timespec;
use std::cmp;

const NANOS_PER_SECOND: u32 = 1_000_000_000;

pub fn now() -> SuspendUnawareInstant {
    // This excerpt of code is taken from the standard library's implementation
    // of Instant:
    // https://github.com/rust-lang/rust/blob/master/library/std/src/sys/pal/unix/time.rs#L260
    // https://www.manpagez.com/man/3/clock_gettime/
    //
    // CLOCK_UPTIME_RAW   clock that increments monotonically, in the same man-
    // ner as CLOCK_MONOTONIC_RAW, but that does not incre- ment while the
    // system is asleep.  The returned value is identical to the result of
    // mach_absolute_time() after the appropriate mach_timebase conversion is
    // applied.
    let mut t: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        libc::clock_gettime(libc::CLOCK_UPTIME_RAW, &mut t);
    }

    // NOTE: It possible for tv_sec/tv_nsec be negative in weird edge cases
    // mentioned in the standard library.  It should NOT be possible for us,
    // since we are polling a performance counter, but out of an ABUNDANCE
    // of caution, we handle this case and floor to 0.  Also, nanos should
    // be capped in size to 10^9. This is done in the standard library as
    // well:
    // https://github.com/rust-lang/rust/blob/9b00956e56009bab2aa15d7bff10916599e3d6d6/library/core/src/time.rs#L96
    // NOTE: ^ that is taken from the release branch for Rust 1.78.0

    t.tv_sec = cmp::max(t.tv_sec, 0);
    t.tv_nsec = cmp::max(t.tv_nsec, 0);
    if t.tv_nsec >= NANOS_PER_SECOND as i64 {
        t.tv_nsec = 0;
    }
    SuspendUnawareInstant {
        secs: t.tv_sec as u64,
        nanos: t.tv_nsec as u32, // (i64 --> u32) we know this type conversion will work since we just clamped it
    }
}

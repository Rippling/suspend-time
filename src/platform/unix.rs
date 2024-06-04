use std::time::Duration;

use libc::timespec;

pub fn now() -> Duration {
    // This excerpt of code is taken from the standard library's implementation of Instant:
    // https://github.com/rust-lang/rust/blob/master/library/std/src/sys/pal/unix/time.rs#L260
    // https://www.manpagez.com/man/3/clock_gettime/
    //
    // CLOCK_UPTIME_RAW   clock that increments monotonically, in the same man-
    //                    ner as CLOCK_MONOTONIC_RAW, but that does not incre-
    //                    ment while the system is asleep.  The returned value
    //                    is identical to the result of mach_absolute_time()
    //                    after the appropriate mach_timebase conversion is
    //                    applied.
    let mut t: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        libc::clock_gettime(libc::CLOCK_UPTIME_RAW, &mut t);
    }

    // TODO: Is it possible for tv_sec/tv_nsec to be negative? Should we have code/panics/errors to handle this case?
    Duration::from_secs(t.tv_sec as u64) + Duration::from_nanos(t.tv_nsec as u64)
}

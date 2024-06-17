use crate::{SuspendUnawareInstant, NANOS_PER_SECOND};
use windows_sys::Win32;

/// As per the windows documentation, the perf count for the counter we are
/// polling stores its value
/// as a count of 100 nanosecond intervals from system
/// boot, not including time spent while the system is suspended/hibernating
const WINDOWS_PERF_INTERVAL_SIZE_NS: u64 = 100;

fn query_unbiased_interrupt_time_precise() -> u64 {
    let mut res: u64 = 0;
    unsafe {
        Win32::System::WindowsProgramming::QueryUnbiasedInterruptTimePrecise(&mut res);
    }
    res
}

/// Calls the windows realtime api function to return the count of 100ns
/// intervals since the system was booted, ignoring periods when the system was
/// suspended/hibernating.    
///
/// Source:
/// https://learn.microsoft.com/en-us/windows/win32/api/realtimeapiset/nf-realtimeapiset-queryunbiasedinterrupttimeprecise
///
/// Note that this is slower and more precise than the similar
/// QueryUnbiasedInterruptTime call. Quoting the documentation, > To provide a
/// system time value that is more precise than that of
/// QueryUnbiasedInterruptTime, QueryUnbiasedInterruptTimePrecise reads the
/// timer hardware directly, therefore a QueryUnbiasedInterruptTimePrecise call
/// can be slower than a QueryUnbiasedInterruptTime call.
pub fn now() -> SuspendUnawareInstant {
    let nanos_per_second_u64 = NANOS_PER_SECOND as u64;
    let nano_intervals = query_unbiased_interrupt_time_precise();
    let secs = nano_intervals / ((nanos_per_second_u64) / WINDOWS_PERF_INTERVAL_SIZE_NS);
    let nanos = ((nano_intervals % nanos_per_second_u64) * 100) % nanos_per_second_u64;

    SuspendUnawareInstant {
        secs,
        nanos: nanos as u32,
    }
}

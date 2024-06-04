use std::time::Duration;

use windows_sys::Win32;

/// Calls the windows realtime api function to return the count of 100ns intervals since the system was booted, ignoring periods when
/// the system was suspended/hibernating.    
/// 
/// Source: https://learn.microsoft.com/en-us/windows/win32/api/realtimeapiset/nf-realtimeapiset-queryunbiasedinterrupttimeprecise    
/// 
/// Note that this is slower and more precise than the similar QueryUnbiasedInterruptTime call. Quoting the documentation,
/// > To provide a system time value that is more precise than that of QueryUnbiasedInterruptTime, QueryUnbiasedInterruptTimePrecise reads the timer hardware directly, therefore a QueryUnbiasedInterruptTimePrecise call can be slower than a QueryUnbiasedInterruptTime call.
pub fn query_unbiased_interrupt_time_precise() -> u64 {
	let mut res: u64 = 0;
	unsafe {
		Win32::System::WindowsProgramming::QueryUnbiasedInterruptTimePrecise(&mut res);
	}
	return res;
}

pub fn now() -> Duration {
	return Duration::from_nanos(query_unbiased_interrupt_time_precise() * 100);
}
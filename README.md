# suspend-time
A cross-platform monotonic clock that is suspend-unaware, written in Rust!

**Documentation**: [to be hosted]

## Example

```rust
use std::{thread, time};
use suspend_time::{SuspendUnawareInstant};

fn main() {
    let instant = SuspendUnawareInstant::now();
    let three_secs = time::Duration::from_secs(3);
    thread::sleep(three_secs);
    assert!(instant.elapsed() >= three_secs);
}
```

## `Instant` vs `SuspendUnawareInstant`

This library is a drop-in replacement for [`std::time::Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html), so you don't need to worry about updating your code.


Similar to the standard library's implementation of [`Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html), except it is consistently unaware of system suspends across all platforms supported by this library.

Historically, this has been inconsistent in the standard library, with windows allowing time to pass when the system is suspended/hibernating, however macOS systems
do not "pass time" during system suspension. In this library, time **never passes** when the system is suspended on **any platform**.

This instant implementation is:
 - Cross platform (windows, macOS)
 - Monotonic (time never goes backwards)
 - Suspend-unaware (when you put your computer to sleep, "time" does not pass.)

# suspend-time
Suspend-time is a cross-platform monotonic clock that is suspend-unaware, written in Rust!    
It allows system suspension (e.g. when a user closes their laptop on windows) to not affect `Instant` durations and timeouts!

**[API Documentation](https://docs.rs/suspend-time/latest/suspend_time/)**

## Example

Example of using `SuspendUnawareInstant`:

```rust
use std::{thread, time};
use suspend_time::{SuspendUnawareInstant};

fn main() {
    // If you used std::time::Instant here and you suspend the system on windows,
    // it will print that more than 3 seconds (circa July 2024).
    // With SuspendUnawareInstant this has no effect.
    let instant = SuspendUnawareInstant::now();
    let three_secs = time::Duration::from_secs(3);
    thread::sleep(three_secs);
    println!("{:#?}", instant.elapsed());
}
```

Example of using `suspend_time::timeout`:

```rust
use std::time::Duration;

#[tokio::main]
async fn main() {
    // If you suspend the system during main's execution, Tokio will time
    // out even though it only slept for 1 second. suspend_time::timeout does not.
    let _ = suspend_time::timeout(
        Duration::from_secs(2),
        suspend_time::sleep(Duration::from_secs(1)),
    ).await;
}
```


## `Instant` vs `SuspendUnawareInstant`

This library is a drop-in replacement for [`std::time::Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html), so you don't need to worry about updating your code.


Similar to the standard library's implementation of [`Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html), except it is consistently unaware of system suspends across all platforms supported by this library.

Historically, this has been inconsistent in the standard library, with windows allowing time to pass when the system is suspended/hibernating, however unix systems
do not "pass time" during system suspension. In this library, time **never passes** when the system is suspended on **any platform**.

This instant implementation is:
 - Cross platform (windows, unix)
 - Monotonic (time never goes backwards)
 - Suspend-unaware (when you put your computer to sleep, "time" does not pass.)
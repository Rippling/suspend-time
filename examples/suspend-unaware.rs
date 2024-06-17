use std::{
    thread::sleep,
    time::{Duration, Instant as StdInstant},
};
use suspend_time::SuspendUnawareInstant;

fn main() {
    let std_start = StdInstant::now();
    let suspend_unaware_start = SuspendUnawareInstant::now();
    loop {
        println!(
            "std time passed: {:#?} suspend unaware time passed: {:#?}",
            std_start.elapsed(),
            suspend_unaware_start.elapsed()
        );

        sleep(Duration::from_secs(1));
    }
}

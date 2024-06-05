use std::time::Instant;

use suspend_time::SuspendUnawareInstant;

fn main() {
    let t1 = Instant::now();

    let z = SuspendUnawareInstant::now();

    let duration = t1.elapsed();

    // dummy print to get the optimizer to keep z around
    println!("dummy print: {:#?}", z.elapsed());

    println!("Took: {:#?}", duration);
}

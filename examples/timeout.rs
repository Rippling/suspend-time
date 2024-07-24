use std::time::Duration;

#[tokio::main]
async fn main() {
    // If you suspend the system during main's execution, Tokio will time
    // out even though it only slept for 1 second. `suspend_time::timeout` does not.
    let _ = suspend_time::timeout(
        Duration::from_secs(2),
        suspend_time::sleep(Duration::from_secs(1)),
    )
    .await;
}

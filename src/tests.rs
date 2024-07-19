use crate::{SuspendUnawareInstant, TimedOutError, NANOS_PER_SECOND};
use futures::task::Context;
use std::{cmp::Ordering, future::Future, task::Poll, time::Duration};

// Locally, this should pass with a 10ms tolerance. However, in circleci
// this is flaky even at 100ms.
const TOLERANCE_MS: u64 = 1000;
const TOLERANCE_MS_U128: u128 = TOLERANCE_MS as u128;

fn create_instant(secs: u64, nanos: u32) -> SuspendUnawareInstant {
    SuspendUnawareInstant { secs, nanos }
}

/// Testing that SuspendUnawareInstant is within a fixed tolerance of std's
/// Instant. (see the tolerance variables) This is difficult since we cannot
/// take both instants at the exact same time.
#[test]
fn accuracy() {
    let std_instant = std::time::Instant::now();
    let suspend_unaware_instant = SuspendUnawareInstant::now();
    let std_elapsed = std_instant.elapsed();
    let suspend_unaware_elapsed = suspend_unaware_instant.elapsed();
    assert!(
        std_elapsed
            .as_millis()
            .abs_diff(suspend_unaware_elapsed.as_millis())
            < TOLERANCE_MS_U128
    )
}

// This test is in fact different than the elapsed test, since we are
// testing the Sub trait on SuspendUnawareInstant I actually ran into a case
// where elapsed() was fine, but Sub was broken. Thus this test is necessary
#[test]
fn subtraction() {
    let a = SuspendUnawareInstant::now();
    std::thread::sleep(Duration::from_secs(1));
    let b = SuspendUnawareInstant::now();
    let dur = b - a;

    assert!(dur.as_millis().abs_diff(1000) < TOLERANCE_MS_U128);
}

// We should clamp to 0 if we subtract an instant from the future from an instant in the past
#[test]
fn incorrect_subtraction() {
    let start = SuspendUnawareInstant::now();
    std::thread::sleep(Duration::from_millis(10));
    let stop = SuspendUnawareInstant::now();

    let res = start - stop;
    assert_eq!(res, Duration::from_secs(0));
}

#[test]
fn ordering_tests() {
    // (lhs, rhs, expected_result)
    #[rustfmt::skip]
        let cases = vec![
            (create_instant(0, 0), create_instant(0, 1), Ordering::Less),
            (create_instant(0, 1), create_instant(0, 2), Ordering::Less),
            (create_instant(0, 100), create_instant(1, 0), Ordering::Less),
            (create_instant(123, 456), create_instant(123, 456), Ordering::Equal),
            (create_instant(0, 1), create_instant(0, 0), Ordering::Greater),
            (create_instant(0, 2), create_instant(0, 1), Ordering::Greater),
            (create_instant(1, 0), create_instant(0, 100), Ordering::Greater),
        ];

    for (lhs, rhs, expected_result) in cases {
        assert_eq!(lhs.cmp(&rhs), expected_result);
    }
}

#[test]
fn addition_tests() {
    // (lhs, rhs, expected_result)
    #[rustfmt::skip]
        let cases = vec![
            (create_instant(0, 0), Duration::new(0, 1), create_instant(0, 1)),
            (create_instant(0, 0), Duration::new(1, 0), create_instant(1, 0)),
            (create_instant(0, 0), Duration::new(u64::MAX, NANOS_PER_SECOND - 1), create_instant(u64::MAX, NANOS_PER_SECOND - 1)),
            (create_instant(1, 0), Duration::new(u64::MAX, 0), create_instant(0, 0)), // floor to 0 when out of bounds/overflow
            (create_instant(u64::MAX, 0), Duration::new(1, 0), create_instant(0, 0)), // floor to 0 when out of bounds/overflow
            (create_instant(u64::MAX, 1), Duration::new(0, NANOS_PER_SECOND - 1), create_instant(0, 0)), // literal edge case, where the nanoseconds push us over the boundary
            (create_instant(u64::MAX, 0), Duration::new(0, NANOS_PER_SECOND - 1), create_instant(u64::MAX, NANOS_PER_SECOND - 1)), // case where this is still valid, since we are just 1 nanosecond shy of going over
            (create_instant(0, NANOS_PER_SECOND - 1), Duration::from_secs(0) + Duration::from_nanos(10), create_instant(1, 9)), // testing nanosecond --> second carry.
        ];

    for (lhs, rhs, expected_result) in cases {
        assert_eq!((lhs) + (rhs), expected_result);
    }
}

#[test]
fn subtraction_duration_tests() {
    #[rustfmt::skip]
        let cases = vec![
            (create_instant(2, 0), Duration::new(1, 0), create_instant(1, 0)),
            (create_instant(2, 0), Duration::new(0, 1), create_instant(1, NANOS_PER_SECOND - 1)), // testing carry seconds --> nanos
            (create_instant(3, 10), Duration::new(1, 2), create_instant(2, 8)), // both fields lhs > rhs
            (create_instant(1, 0), Duration::new(2, 0), create_instant(0, 0)), // testing set to 0 on negative result
            (create_instant(1, 1), Duration::new(1, 2), create_instant(0, 0)), // testing set to 0 on negative result, this time with nanos causing the fault
        ];

    for (lhs, rhs, expected_result) in cases {
        assert_eq!((lhs) - (rhs), expected_result);
    }
}

#[test]
fn subtraction_instant_tests() {
    #[rustfmt::skip]
        let cases = [
            (create_instant(10, 5), create_instant(1, 2), Duration::new(9, 3)),
            (create_instant(1, 0), create_instant(2, 0), Duration::new(0, 0)), // seconds cause negative
            (create_instant(1, 0), create_instant(1, 1), Duration::new(0, 0)), // nanos cause negative
            (create_instant(2, 0), create_instant(0, 1), Duration::new(1, NANOS_PER_SECOND - 1)), // nano carry
        ];

    for (lhs, rhs, expected_result) in cases {
        assert_eq!((lhs) - (rhs), expected_result);
    }
}

// Tests the behaviour of the sleep future as a task, testing against a tokio timeout (with tolerance).
// (If the waking logic in crate::sleep is wrong, this test will fail)
#[tokio::test]
async fn sleep_task_test() {
    let sleep_duration = Duration::from_secs(1);
    let completion_deadline_duration = sleep_duration + Duration::from_millis(TOLERANCE_MS);
    let task = tokio::task::spawn(crate::sleep(sleep_duration));

    let res = crate::timeout(completion_deadline_duration, task).await;

    assert!(res.is_ok());
}

// test that the suspend unaware timeout truly times out before a task is completed.
#[tokio::test]
async fn timeout_tokio_test() {
    let task_duration = Duration::from_secs(999); // sleep for a long time
    let suspend_unaware_deadline = Duration::from_secs(1);
    let tokio_deadline = suspend_unaware_deadline + Duration::from_secs(1);
    // we are using a TOKIO timeout to wrap our SUSPEND UNAWARE TIMEOUT which
    // itself is wrapping a task that sleeps forever.  if the SUSPEND UNAWARE
    // TIMEOUT times out, the tokio timeout will not. If it runs forever, then
    // the TOKIO timeout will time out and return an error.
    let res = crate::timeout(
        tokio_deadline,
        crate::timeout(
            suspend_unaware_deadline,
            tokio::task::spawn(async move { tokio::time::sleep(task_duration).await }),
        ),
    )
    .await;

    assert!(res.is_ok());
}

#[tokio::test]
async fn timeout_table_test() {
    // (timeout duration, task duration, expected result)
    let cases = vec![
        (
            Duration::from_secs(1),
            Duration::from_secs(2),
            Err(TimedOutError),
        ),
        (Duration::from_secs(2), Duration::from_secs(1), Ok(())),
    ];
    for (timeout_duration, task_duration, expected_result) in cases {
        let res = crate::timeout(
            timeout_duration,
            tokio::task::spawn(async move { tokio::time::sleep(task_duration).await }),
        )
        .await;

        match expected_result {
            Err(expected_error) => {
                assert_eq!(res.err().unwrap(), expected_error);
            }
            Ok(_) => {
                assert!(res.unwrap().is_ok());
            }
        }
    }
}

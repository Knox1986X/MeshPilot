use super::*;
use crate::interval::GAP;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FIRST: &str = "cpu  100 50 30 400 10 3 4 2 10 4\n";
// user +8, nice +2, system +4, idle +15, iowait +5, irq +2, softirq +1, steal +3.
const SECOND: &str = "cpu  108 52 34 415 15 5 5 5 16 6\ncpu0 999\n";

fn counts(line: &str) -> Option<Counters> {
    Counters::parse(line.as_bytes())
}

fn with_samples(
    first: &str,
    second: &str,
    duration: Duration,
    wall: SystemTime,
) -> Option<CpuObservation> {
    let t = Instant::now();
    counts(first)?.report(counts(second)?, Window::new(t, t + duration, wall)?)
}

#[test]
fn reports_the_delta_not_lifetime_totals_and_uses_the_actual_interval() {
    let r = with_samples(
        FIRST,
        SECOND,
        Duration::from_millis(637),
        UNIX_EPOCH + Duration::from_millis(1234),
    )
    .unwrap();
    assert_eq!(
        (r.busy_ticks, r.total_ticks, r.usage_percent),
        (20, 40, 50.0)
    );
    assert_eq!(r.interval_ms, 637); // Не запрошенные 500 и не разность UTC-часов.
    assert_eq!(r.source, "/proc/stat");
    assert_eq!(r.observed_at_utc, "1970-01-01T00:00:01.234Z");
}

#[test]
fn true_idle_and_fully_busy_are_both_measurements() {
    for (second, pct) in [
        ("cpu 1 0 0 3 1 0 0 0\n", 0.0),
        ("cpu 2 0 0 1 0 0 0 1\n", 100.0),
    ] {
        let r = with_samples("cpu 1 0 0 1 0 0 0 0\n", second, GAP, UNIX_EPOCH).unwrap();
        assert_eq!(r.usage_percent, pct);
    }
}

#[test]
fn requires_complete_bounded_first_aggregate_only() {
    for line in [
        "",
        "cpu0 1 2 3 4 5 6 7 8\n",
        "intr 1\ncpu 1 2 3 4 5 6 7 8\n",
        "cpu 1 2 3 4\n",
        "cpu 1 2 3 4 5 6 7 8",
        "cpu 1 2 3 4 5 6 7 8 9 10 11\n",
    ] {
        assert!(counts(line).is_none(), "{line}");
    }
    for bad in ["+1", "-2", "3.5", "18446744073709551616", "x"] {
        assert!(counts(&format!("cpu 1 2 3 4 5 6 7 8 {bad}\n")).is_none());
    }
    assert!(counts(&format!("cpu{}1 2 3 4 5 6 7 8\n", " ".repeat(MAX_LINE))).is_none());
    assert!(counts(&format!("{FIRST}{}", "x".repeat(65537))).is_some());
    assert!(Counters::parse(&b"cpu 1 2 3 4 5 6 7 \xff\n"[..]).is_none());
}

#[test]
fn regression_changed_layout_no_ticks_and_overflow_are_not_idle() {
    for second in [
        FIRST,
        "cpu 300 50 30 400 9 3 4 2 10 4\n",
        "cpu 200 50 30 400 10 3 4 2\n",
        "cpu 200 50 30 400 10 3 4 2 9 4\n",
        "cpu 18446744073709551615 999 30 400 10 3 4 2 10 4\n",
        "cpu 9007199254741092 50 30 400 10 3 4 2 10 4\n",
    ] {
        assert!(
            with_samples(FIRST, second, GAP, UNIX_EPOCH).is_none(),
            "{second}"
        );
    }
    // Вычитаем счётчики до сложения, даже когда суммы накопленных значений не помещаются в u64.
    let a = "cpu 18446744073709551615 1 0 10 0 0 0 0\n";
    let b = "cpu 18446744073709551615 1 0 11 0 0 0 0\n";
    assert_eq!(counts(a).unwrap().deltas(&counts(b).unwrap()), Some((0, 1)));
}

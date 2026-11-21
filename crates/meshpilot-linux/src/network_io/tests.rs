use super::*;
use std::time::{Duration, Instant, UNIX_EPOCH};

const HEAD: &str = "Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n";
fn row(name: &str, rx: u64, tx: u64) -> String {
    format!(" {name}: {rx} 2 3 4 5 6 7 8 {tx} 9 10 11 12 13 14 15\n")
}
fn counts(body: &str) -> Option<Counters> {
    Counters::parse(format!("{HEAD}{body}").as_bytes())
}
fn report(a: &str, b: &str) -> NetworkIoObservation {
    let t = Instant::now();
    counts(a)
        .unwrap()
        .report(
            counts(b).unwrap(),
            Window::new(t, t + Duration::from_millis(625), UNIX_EPOCH).unwrap(),
        )
        .unwrap()
}

#[test]
fn matches_names_not_row_positions_and_uses_the_measured_interval() {
    let r = report(
        &(row("lo", 10, 20) + &row("eth0.4", 800, 1300)),
        &(row("eth0.4", 1800, 3300) + &row("lo", 10, 20)),
    );
    assert_eq!(r.interval_ms, 625);
    assert_eq!(r.source, "/proc/net/dev");
    assert_eq!(r.observed_at_utc, "1970-01-01T00:00:00.000Z");
    let [eth, lo] = r.interfaces.as_slice() else {
        panic!()
    };
    assert_eq!(eth.name, "eth0.4");
    let v = eth.rates.as_ref().unwrap();
    assert_eq!(
        (
            v.rx_delta_bytes,
            v.tx_delta_bytes,
            v.rx_bytes_per_sec,
            v.tx_bytes_per_sec
        ),
        (1000, 2000, 1600.0, 3200.0)
    );
    assert_eq!(lo.name, "lo");
    let v = lo.rates.as_ref().unwrap();
    assert_eq!((v.rx_bytes_per_sec, v.tx_bytes_per_sec), (0.0, 0.0));
}

#[test]
fn empty_table_new_missing_reset_and_idle_are_distinct() {
    assert!(report(&row("old", 1, 1), "").interfaces.is_empty());
    let r = report(
        &(row("old", 1, 1) + &row("eth0", 1, 1) + &row("lo", 1, 1)),
        &(row("eth0", 2, 0) + &row("new", 1, 1) + &row("lo", 2, 1)),
    );
    assert_eq!(
        r.interfaces
            .iter()
            .map(|i| (&*i.name, i.rates.is_some()))
            .collect::<Vec<_>>(),
        [("eth0", false), ("lo", true), ("new", false)]
    );
}

#[test]
fn lifetime_counters_stay_u64_only_deltas_cross_ipc() {
    assert!(rates((u64::MAX - 1, u64::MAX), (u64::MAX, u64::MAX), 2.0).is_some());
    assert!(rates((10, 0), (9, 20), 2.0).is_none());
    assert!(rates((0, 0), (MAX_SAFE_INTEGER + 1, 0), 2.0).is_none());
    assert!(rates((0, 0), (0, MAX_SAFE_INTEGER + 1), 2.0).is_none());
}

#[test]
fn rejects_ambiguous_truncated_or_malformed_tables() {
    let ok = row("eth0", 1, 2);
    for bad in [
        "lo: 1 2\n".into(),
        ok.trim_end().into(),
        ok.clone() + &ok,
        ok.replace(" 2 3 ", " +2 3 "),
        ok.replace(" 2 3 ", " -2 3 "),
        ok.replace(" 9 10 ", " 18446744073709551616 10 "),
        ok.replace("15\n", "15 16\n"),
        row("a/b", 1, 2),
        row("a\x1bb", 1, 2),
        row("a b", 1, 2),
        row("..", 1, 2),
        row("abcdefghijklmnop", 1, 2),
    ] {
        assert!(counts(&bad).is_none(), "{bad:?}");
    }
    for bad in [
        HEAD.replace("Receive", "Other"),
        HEAD.replace("colls carrier", "carrier colls"),
        "".into(),
    ] {
        assert!(Counters::parse(bad.as_bytes()).is_none());
    }
    assert!(Counters::parse(&b"\xff"[..]).is_none());
}

#[test]
fn budgets_bound_reads_and_serialized_interfaces() {
    let mut table = (0..MAX_INTERFACES)
        .map(|i| row(&format!("veth{i}"), 0, 0))
        .collect::<String>();
    assert!(counts(&table).is_some());
    table.push_str(&row("extra", 0, 0));
    assert!(counts(&table).is_none());
    assert!(counts(&format!("{}{}", " ".repeat(65536), row("eth0", 0, 0))).is_none());
}

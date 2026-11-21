//! Трафик по именам интерфейсов в сетевом пространстве процесса. Не проба связи.
use crate::{
    MAX_SAFE_INTEGER, bounded_text,
    interval::{self, Probe, Window},
};
use meshpilot_core::{InterfaceIo, NetworkIoObservation, TransferRates};
use std::{collections::BTreeMap, fs::File, io::Read};

const SOURCE: &str = "/proc/net/dev";
const MAX_INTERFACES: usize = 256;
const RX: [&str; 8] = [
    "bytes",
    "packets",
    "errs",
    "drop",
    "fifo",
    "frame",
    "compressed",
    "multicast",
];
const TX: [&str; 8] = [
    "bytes",
    "packets",
    "errs",
    "drop",
    "fifo",
    "colls",
    "carrier",
    "compressed",
];

pub(super) struct Counters(BTreeMap<String, (u64, u64)>);

impl Counters {
    fn parse(reader: impl Read) -> Option<Self> {
        let text = bounded_text(reader)?; // ≤ 64 КиБ; не принять усечённую последнюю строку.
        if !text.ends_with('\n') {
            return None;
        }
        let mut rows = text.lines();
        if !rows
            .next()?
            .split('|')
            .map(str::trim)
            .eq(["Inter-", "Receive", "Transmit"])
        {
            return None;
        }
        let mut columns = rows.next()?.split('|');
        if columns.next()?.trim() != "face"
            || !columns.next()?.split_ascii_whitespace().eq(RX)
            || !columns.next()?.split_ascii_whitespace().eq(TX)
            || columns.next().is_some()
        {
            return None;
        }
        let mut devices = BTreeMap::new();
        for row in rows {
            let (name, fields) = row.split_once(':')?;
            let name = name.trim_start_matches(' ');
            if devices.len() >= MAX_INTERFACES
                || !(1..=15).contains(&name.len())
                || matches!(name, "." | "..")
                || !name.bytes().all(|b| b.is_ascii_graphic() && b != b'/')
            {
                return None;
            }
            let (mut rx, mut tx) = (0, 0);
            let mut fields = fields.split_ascii_whitespace();
            for i in 0..16 {
                let field = fields.next()?;
                if !field.as_bytes().first()?.is_ascii_digit() {
                    return None;
                }
                let n = field.parse::<u64>().ok()?;
                match i {
                    0 => rx = n,
                    8 => tx = n,
                    _ => {}
                }
            }
            if fields.next().is_some() || devices.insert(name.into(), (rx, tx)).is_some() {
                return None;
            }
        }
        Some(Self(devices))
    }
}

fn rates(before: (u64, u64), after: (u64, u64), per_sec: f64) -> Option<TransferRates> {
    let rx = after.0.checked_sub(before.0)?;
    let tx = after.1.checked_sub(before.1)?;
    (rx <= MAX_SAFE_INTEGER && tx <= MAX_SAFE_INTEGER).then_some(TransferRates {
        rx_delta_bytes: rx,
        tx_delta_bytes: tx,
        rx_bytes_per_sec: rx as f64 * per_sec,
        tx_bytes_per_sec: tx as f64 * per_sec,
    })
}

impl Probe for Counters {
    type Report = NetworkIoObservation;
    fn read() -> Option<Self> {
        Self::parse(File::open(SOURCE).ok()?)
    }
    fn report(self, next: Self, window: Window) -> Option<Self::Report> {
        let per_sec = 1000.0 / window.ms as f64;
        let interfaces = next
            .0
            .into_iter()
            .map(|(name, after)| {
                let rates = self
                    .0
                    .get(&name)
                    .and_then(|&before| rates(before, after, per_sec));
                InterfaceIo { name, rates }
            })
            .collect();
        Some(NetworkIoObservation {
            interfaces,
            interval_ms: window.ms,
            source: SOURCE,
            observed_at_utc: window.utc,
        })
    }
}

/// Отдельное измерение для кнопки окна; CLI разделяет паузу с CPU.
pub fn network_io_snapshot() -> Option<NetworkIoObservation> {
    interval::single::<Counters>()
}

#[cfg(test)]
mod tests;

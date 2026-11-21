//! Средняя доля не простаивавшего CPU по агрегату ядра, не load average или нагрузка процесса.
use crate::utc_stamp;
use meshpilot_core::CpuObservation;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    thread,
    time::{Duration, Instant, SystemTime},
};

const SOURCE: &str = "/proc/stat";
const SAMPLE_GAP: Duration = Duration::from_millis(500);
const MAX_INTERVAL: Duration = Duration::from_secs(5);
const MAX_LINE: usize = 512;

struct Counters(Vec<u64>);

impl Counters {
    fn read(reader: impl Read) -> Option<Self> {
        // В procfs агрегат всегда первый. Не читать длинные списки CPU и IRQ.
        let mut line = String::new();
        BufReader::new(reader.take(MAX_LINE as u64 + 1))
            .read_line(&mut line)
            .ok()?;
        if line.len() > MAX_LINE || !line.ends_with('\n') {
            return None;
        }
        let mut fields = line.split_ascii_whitespace();
        if fields.next()? != "cpu" {
            return None;
        }
        let ticks: Vec<_> = fields
            .map(|field| {
                if !field.bytes().all(|c| c.is_ascii_digit()) {
                    return None;
                }
                field.parse::<u64>().ok()
            })
            .collect::<Option<_>>()?;
        // От user до steal обязательны. За ними возможны только guest и guest_nice.
        (8..=10).contains(&ticks.len()).then_some(Self(ticks))
    }

    fn deltas(&self, next: &Self) -> Option<(u64, u64)> {
        if self.0.len() != next.0.len() {
            return None;
        }
        let (mut busy, mut total) = (0u64, 0u64);
        for (i, (before, after)) in self.0.iter().zip(&next.0).enumerate() {
            let delta = after.checked_sub(*before)?; // В том числе возможная регрессия iowait.
            if i >= 8 {
                continue;
            } // guest и guest_nice уже входят в user и nice.
            total = total.checked_add(delta)?;
            if i != 3 && i != 4 {
                busy = busy.checked_add(delta)?;
            } // idle и iowait — простой.
        }
        // Не передавать lossy integers через JSON в WebView.
        (1..=(1 << 53) - 1)
            .contains(&total)
            .then_some((busy, total))
    }
}

fn observe(
    mut sample: impl FnMut() -> Option<(Counters, Instant)>,
    wait: impl FnOnce(Duration),
    now: impl FnOnce() -> SystemTime,
) -> Option<CpuObservation> {
    let (first, start) = sample()?;
    wait(SAMPLE_GAP);
    let (second, end) = sample()?;
    let observed_at_utc = utc_stamp(now())?;
    let interval = end.checked_duration_since(start)?;
    if !(SAMPLE_GAP..=MAX_INTERVAL).contains(&interval) {
        return None;
    }
    let (busy_ticks, total_ticks) = first.deltas(&second)?;
    Some(CpuObservation {
        usage_percent: 100.0 * (busy_ticks as f64 / total_ticks as f64),
        busy_ticks,
        total_ticks,
        interval_ms: interval.as_millis().try_into().ok()?,
        source: SOURCE,
        observed_at_utc,
    })
}

/// Два ограниченных чтения и одна короткая пауза. Не вызывать на потоке окна.
pub fn cpu_snapshot() -> Option<CpuObservation> {
    observe(
        || Some((Counters::read(File::open(SOURCE).ok()?)?, Instant::now())),
        thread::sleep,
        SystemTime::now,
    )
}

#[cfg(test)]
mod tests;

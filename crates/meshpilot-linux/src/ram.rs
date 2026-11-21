//! Один снимок памяти, видимой ядру через procfs (не лимит cgroup и не RSS приложения).
use crate::{bounded_text, utc_stamp};
use meshpilot_core::RamObservation;
use std::{fs::File, io, time::SystemTime};

const SOURCE: &str = "/proc/meminfo";

/// Без путей и параметров из IPC. Время ставит читатель непосредственно после чтения.
pub fn ram_snapshot() -> Option<RamObservation> {
    observe(File::open(SOURCE), SystemTime::now)
}

fn observe(
    file: io::Result<impl io::Read>,
    now: impl FnOnce() -> SystemTime,
) -> Option<RamObservation> {
    let text = bounded_text(file.ok()?)?;
    let observed_at_utc = utc_stamp(now())?;
    let (mut total, mut available) = (None, None);
    for line in text.lines() {
        let Some((key, raw)) = line.split_once(':') else {
            continue;
        };
        let slot = match key {
            "MemTotal" => &mut total,
            "MemAvailable" => &mut available,
            _ => continue,
        };
        let mut fields = raw.split_ascii_whitespace();
        let digits = fields.next()?;
        if !digits.bytes().all(|b| b.is_ascii_digit())
            || fields.next() != Some("kB")
            || fields.next().is_some()
        {
            return None;
        }
        // kB ядра Linux — 1024 байта. Не округлять переполнение или дубликат до «рабочего» числа.
        let bytes = digits.parse::<u64>().ok()?.checked_mul(1024)?;
        if slot.replace(bytes).is_some() {
            return None;
        }
    }
    let total_bytes = total?;
    // Целые числа JSON должны быть точными и в WebView.
    if total_bytes == 0 || total_bytes > crate::MAX_SAFE_INTEGER {
        return None;
    }
    Some(RamObservation {
        total_bytes,
        used_bytes: total_bytes.checked_sub(available?)?,
        source: SOURCE,
        observed_at_utc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn sample(text: &str) -> Option<RamObservation> {
        observe(Ok(text.as_bytes()), || {
            UNIX_EPOCH + Duration::from_millis(1234)
        })
    }

    #[test]
    fn measured_bytes_use_available_not_free_and_carry_the_read_time() {
        let r = sample("MemFree: 3 kB\nMemAvailable:\t12 kB\nSwapTotal: 0 kB\nMemTotal: 32 kB\n")
            .unwrap();
        assert_eq!((r.used_bytes, r.total_bytes), (20 * 1024, 32 * 1024));
        assert_eq!(r.source, "/proc/meminfo");
        assert_eq!(r.observed_at_utc, "1970-01-01T00:00:01.234Z");
    }

    #[test]
    fn both_zero_used_and_zero_available_can_be_real() {
        for (available, used) in [(16, 0), (0, 16 * 1024)] {
            assert_eq!(
                sample(&format!("MemTotal: 16 kB\nMemAvailable: {available} kB"))
                    .unwrap()
                    .used_bytes,
                used
            );
        }
    }

    #[test]
    fn missing_ambiguous_and_impossible_measurements_are_unavailable() {
        for text in [
            "",
            "MemAvailable: 0 kB",
            "MemTotal: 4 kB\nMemFree: 1 kB",
            "MemTotal: 1 kB\nMemAvailable: 2 kB",
            "MemTotal: 0 kB\nMemAvailable: 0 kB",
            "MemTotal: 1 kB\nMemAvailable: 0 kB\nMemAvailable: 0 kB",
            "MemTotal: 1 kB\nMemAvailable: 0 kB\nMemTotal: 2 kB",
        ] {
            assert!(sample(text).is_none(), "{text}");
        }
        for value in [
            "",
            "-1 kB",
            "+1 kB",
            "1.2 kB",
            "1",
            "1 KB",
            "1 B",
            "1 kB extra",
            "18446744073709551615 kB",
            "18446744073709551616 kB",
            "8796093022208 kB",
        ] {
            assert!(
                sample(&format!("MemTotal: {value}\nMemAvailable: 0 kB")).is_none(),
                "{value}"
            );
        }
    }

    #[test]
    fn missing_timestamp_does_not_become_a_successful_sample() {
        let text = &b"MemTotal: 4 kB\nMemAvailable: 1 kB"[..];
        assert!(observe(Ok(text), || UNIX_EPOCH - Duration::from_secs(1)).is_none());
    }

    #[test]
    fn unreadable_or_truncated_input_never_yields_a_partial_sample() {
        assert!(
            observe(
                Err::<&[u8], _>(io::ErrorKind::PermissionDenied.into()),
                SystemTime::now
            )
            .is_none()
        );
        assert!(
            observe(
                Ok(&b"MemTotal: 2 kB\nMemAvailable: 1 kB\n\xff"[..]),
                SystemTime::now
            )
            .is_none()
        );
        assert!(
            sample(&format!(
                "MemTotal: 2 kB\nMemAvailable: 1 kB\n{}",
                "x".repeat(65536)
            ))
            .is_none()
        );
    }
}

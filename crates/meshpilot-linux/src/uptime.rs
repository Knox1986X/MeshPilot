//! Одно чтение секунд с момента загрузки ядра. Не тикер, не опрос, не второй замер.
use crate::{bounded_text, utc_stamp};
use meshpilot_core::UptimeObservation;
use std::{fs::File, io, time::SystemTime};

const SOURCE: &str = "/proc/uptime";

/// Файл вида `11931.90 22378.30\n`: аптайм и простоя ядра, один пробел,
/// завершающий перевод строки. Берём целую часть первого поля; второе поле
/// не используем, но требуем полную и валидную таблицу.
pub fn uptime_snapshot() -> Option<UptimeObservation> {
    observe(File::open(SOURCE), SystemTime::now)
}

fn observe(
    file: io::Result<impl io::Read>,
    now: impl FnOnce() -> SystemTime,
) -> Option<UptimeObservation> {
    let text = bounded_text(file.ok()?)?;
    let observed_at_utc = utc_stamp(now())?;
    let body = text.strip_suffix('\n').unwrap_or(&text);
    let (first, rest) = body.split_once(' ')?;
    (rest.split_once(' ').is_none()).then_some(())?;
    let seconds = parse_seconds(first)?;
    parse_seconds(rest)?;
    Some(UptimeObservation {
        uptime_seconds: seconds,
        source: SOURCE,
        observed_at_utc,
    })
}

/// Целые секунды из поля `digits` или `digits.digits`. Десятичная дробь —
/// тот же счётчик с точностью до сотых, поэтому результат — целая часть.
fn parse_seconds(raw: &str) -> Option<u64> {
    let (integer, frac) = match raw.split_once('.') {
        None => (raw, ""),
        Some((a, b)) if !b.is_empty() => (a, b),
        _ => return None,
    };
    if integer.is_empty()
        || !integer.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    integer.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn sample(text: &str) -> Option<UptimeObservation> {
        observe(Ok(text.as_bytes()), || {
            UNIX_EPOCH + Duration::from_millis(1234)
        })
    }

    #[test]
    fn measured_seconds_are_the_integer_part_with_source_and_time() {
        let s = sample("11931.90 22378.30").unwrap();
        assert_eq!(s.uptime_seconds, 11931);
        assert_eq!(s.source, "/proc/uptime");
        assert_eq!(s.observed_at_utc, "1970-01-01T00:00:01.234Z");
        assert_eq!(sample("11931.90 22378.30\n").unwrap().uptime_seconds, 11931);
    }

    #[test]
    fn freshly_booted_system_can_be_zero_seconds() {
        assert_eq!(sample("0.50 0.10").unwrap().uptime_seconds, 0);
        assert_eq!(sample("7 9").unwrap().uptime_seconds, 7);
    }

    #[test]
    fn incomplete_or_invalid_tables_are_unavailable() {
        for text in [
            "",
            "12.5",
            "12.5 8.0 extra",
            " 12 13",
            "12 13 ",
            ".5 1.0",
            "5. 1.0",
            "1.2.3 4.0",
            "1e2 3.0",
            "-1 2.0",
            "+1 2.0",
            "99999999999999999999999 1",
        ] {
            assert!(sample(text).is_none(), "{text}");
        }
    }

    #[test]
    fn missing_clock_or_unreadable_source_is_not_a_sample() {
        let text = &b"12.5 13.0"[..];
        assert!(observe(Ok(text), || UNIX_EPOCH - Duration::from_secs(1)).is_none());
        assert!(
            observe(
                Err::<&[u8], _>(io::ErrorKind::PermissionDenied.into()),
                SystemTime::now
            )
            .is_none()
        );
        let big = format!("12.5 {}\n", "x".repeat(65536));
        assert!(observe(Ok(big.as_bytes()), SystemTime::now).is_none());
    }
}

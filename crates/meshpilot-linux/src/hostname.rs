//! Одно чтение имени узла из `/proc/sys/kernel/hostname`.
//! Файл ядра, внешние программы не запускаем.
use crate::{bounded_text, utc_stamp};
use meshpilot_core::HostnameObservation;
use std::{fs::File, io, time::SystemTime};

const SOURCE: &str = "/proc/sys/kernel/hostname";

/// Имя узла на момент чтения; 1–64 печатных ASCII без краевых пробелов.
/// Имя можно сменить в работе (`hostnamectl`), поэтому приёмка в VM —
/// по границам независимых наблюдений, а не по одному значению.
pub fn hostname_snapshot() -> Option<HostnameObservation> {
    observe(File::open(SOURCE), SystemTime::now)
}

fn observe(
    file: io::Result<impl io::Read>,
    now: impl FnOnce() -> SystemTime,
) -> Option<HostnameObservation> {
    let text = bounded_text(file.ok()?)?;
    let observed_at_utc = utc_stamp(now())?;
    // Файл заканчивается переводом строки; вторая строка или управляющие
    // символы — не формат, остаются недопустимыми.
    let name = text.strip_suffix('\n').unwrap_or(text.as_str());
    // HNAME_MAX на Linux — 64 байта; не-ASCII для имени узла не допустим.
    if name.is_empty()
        || name.len() > 64
        || name.bytes().any(|b| !(0x20..=0x7E).contains(&b))
        || name.starts_with(' ')
        || name.ends_with(' ')
    {
        return None;
    }
    Some(HostnameObservation {
        name: name.to_owned(),
        source: SOURCE,
        observed_at_utc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn sample(text: &str) -> Option<HostnameObservation> {
        observe(Ok(text.as_bytes()), || {
            UNIX_EPOCH + Duration::from_millis(1234)
        })
    }

    #[test]
    fn name_kept_verbatim_with_source_and_time() {
        let h = sample("cachyos\n").unwrap();
        assert_eq!(h.name, "cachyos");
        assert_eq!(h.source, "/proc/sys/kernel/hostname");
        assert_eq!(h.observed_at_utc, "1970-01-01T00:00:01.234Z");
        // Перевод строки в конце файла допустим, его отсутствие тоже.
        assert_eq!(sample("e2b.local").unwrap().name, "e2b.local");
    }

    #[test]
    fn malformed_names_are_unavailable_not_empty() {
        for text in [
            "",
            "\n",
            " x\n",
            "x \n",
            &format!("{}\n", "x".repeat(65)),
            "a\x01b\n",
            "тест\n",
            "a\nb\n",
        ] {
            assert!(sample(text).is_none(), "{text:?}");
        }
        // Пробелы внутри имени допустимы на Linux.
        assert_eq!(sample("a b c d\n").unwrap().name, "a b c d");
    }

    #[test]
    fn missing_clock_or_unreadable_source_is_not_a_sample() {
        assert!(
            observe(Ok(b"cachyos\n" as &[u8]), || UNIX_EPOCH
                - Duration::from_secs(1))
            .is_none()
        );
        assert!(
            observe(
                Err::<&[u8], _>(io::ErrorKind::PermissionDenied.into()),
                SystemTime::now
            )
            .is_none()
        );
        let big = format!("{}\n", "x".repeat(65536));
        assert!(observe(Ok(big.as_bytes()), SystemTime::now).is_none());
    }
}

//! Одно чтение версии ядра. Внешние программы (`uname`) не запускаются.
use crate::{bounded_text, utc_stamp};
use meshpilot_core::KernelObservation;
use std::{fs::File, io, time::SystemTime};

const SOURCE: &str = "/proc/version";

/// Строка ядра: `Linux version <версия> (кто собирал) (компилятор) #N ...`.
/// Версия — второе слово; во время загрузки она не меняется, поэтому
/// сверка в VM — по точному равенству, не по дельтам.
pub fn kernel_snapshot() -> Option<KernelObservation> {
    observe(File::open(SOURCE), SystemTime::now)
}

fn observe(
    file: io::Result<impl io::Read>,
    now: impl FnOnce() -> SystemTime,
) -> Option<KernelObservation> {
    let text = bounded_text(file.ok()?)?;
    let observed_at_utc = utc_stamp(now())?;
    let (line, rest) = text.split_once('\n').unwrap_or((text.as_str(), ""));
    (rest.is_empty()).then_some(line)?;
    let mut fields = line.split_whitespace();
    if fields.next() != Some("Linux") || fields.next() != Some("version") {
        return None;
    }
    let version = fields.next()?;
    // Печатные ASCII без пробелов; длина ограничена, как и у имён интерфейсов.
    if version.is_empty() || version.len() > 64 || !version.bytes().all(|b| b.is_ascii_graphic()) {
        return None;
    }
    Some(KernelObservation {
        version: version.to_owned(),
        source: SOURCE,
        observed_at_utc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn sample(text: &str) -> Option<KernelObservation> {
        observe(Ok(text.as_bytes()), || {
            UNIX_EPOCH + Duration::from_millis(1234)
        })
    }

    const LINE: &str = "Linux version 6.12.9-cachyos1-1 (build@cachyos) (gcc 15.1.0) #1 SMP PREEMPT_DYNAMIC Mon Nov 17 10:00:00 UTC 2026";

    #[test]
    fn second_word_is_the_kernel_version_with_source_and_time() {
        let k = sample(LINE).unwrap();
        assert_eq!(k.version, "6.12.9-cachyos1-1");
        assert_eq!(k.source, "/proc/version");
        assert_eq!(k.observed_at_utc, "1970-01-01T00:00:01.234Z");
        // /proc/version завершается переводом строки — он допустим.
        assert_eq!(
            sample(&format!("{LINE}\n")).unwrap().version,
            "6.12.9-cachyos1-1"
        );
    }

    #[test]
    fn plus_and_dots_in_version_are_kept_verbatim() {
        assert_eq!(
            sample("Linux version 6.1.158+ (root@runner) (gcc) #1 SMP")
                .unwrap()
                .version,
            "6.1.158+"
        );
        assert_eq!(
            sample("Linux version 1.0.0 (a) (b) #1").unwrap().version,
            "1.0.0"
        );
    }

    #[test]
    fn malformed_lines_are_unavailable_not_empty() {
        for text in [
            "",
            "\n",
            "Linux 6.12 (a) (b) #1",
            "Linux version",
            "Linux version 6.12\nsecond line",
            "linux version 6.12 (a) (b) #1",
            "Linux version 6.1\x012 (a) (b) #1",
            &format!("Linux version {} (a) (b) #1", "1".repeat(65)),
        ] {
            assert!(sample(text).is_none(), "{text:?}");
        }
    }

    #[test]
    fn missing_clock_or_unreadable_source_is_not_a_sample() {
        let text = &b"Linux version 6.12 (a) (b) #1"[..];
        assert!(observe(Ok(text), || UNIX_EPOCH - Duration::from_secs(1)).is_none());
        assert!(
            observe(
                Err::<&[u8], _>(io::ErrorKind::PermissionDenied.into()),
                SystemTime::now
            )
            .is_none()
        );
        let big = format!("Linux version 6.12 {}\n", "x".repeat(65536));
        assert!(observe(Ok(big.as_bytes()), SystemTime::now).is_none());
    }
}

//! Одно чтение версии системы: поле VERSION из `/etc/os-release`.
//! Внешние программы не запускаются.
use crate::{bounded_text, utc_stamp};
use meshpilot_core::OsVersionObservation;
use std::{fs::File, io, time::SystemTime};

const SOURCE: &str = "/etc/os-release";

/// Значение единственного поля `VERSION=` — версия системы дословно
/// (допустимы пробелы и скобки). За загрузку оно не меняется, поэтому
/// сверка в VM — по точному равенству, не по дельтам.
pub fn os_version_snapshot() -> Option<OsVersionObservation> {
    observe(File::open(SOURCE), SystemTime::now)
}

fn observe(
    file: io::Result<impl io::Read>,
    now: impl FnOnce() -> SystemTime,
) -> Option<OsVersionObservation> {
    let text = bounded_text(file.ok()?)?;
    let observed_at_utc = utc_stamp(now())?;
    let mut values = text.lines().filter_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key == "VERSION").then_some(value)
    });
    let raw = values.next()?;
    // Два поля VERSION — некорректная ос-release-таблица.
    if values.next().is_some() {
        return None;
    }
    let version = unquote(raw)?;
    // Печатные ASCII, без кавычек и краевых пробелов; длина ограничена.
    if version.is_empty()
        || version.len() > 64
        || version.bytes().any(|b| !(0x20..=0x7E).contains(&b))
        || version.bytes().any(|b| b == b'"' || b == b'\'')
        || version.starts_with(' ')
        || version.ends_with(' ')
    {
        return None;
    }
    Some(OsVersionObservation {
        version: version.to_owned(),
        source: SOURCE,
        observed_at_utc,
    })
}

// Значение может не иметь кавычек или быть заключено в одну пару;
// «"a" "b"» и незакрытая кавычка — не формат.
fn unquote(raw: &str) -> Option<&str> {
    match raw.as_bytes().first()? {
        b'"' => raw.strip_prefix('"').and_then(|v| v.strip_suffix('"')),
        b'\'' => raw.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')),
        _ => Some(raw),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn sample(text: &str) -> Option<OsVersionObservation> {
        observe(Ok(text.as_bytes()), || {
            UNIX_EPOCH + Duration::from_millis(1234)
        })
    }

    const FILE: &str =
        "NAME=\"Debian GNU/Linux\"\nVERSION_ID=\"12\"\nVERSION=\"12 (bookworm)\"\nID=debian\n";

    #[test]
    fn quoted_version_is_kept_verbatim_with_source_and_time() {
        let v = sample(FILE).unwrap();
        assert_eq!(v.version, "12 (bookworm)");
        assert_eq!(v.source, "/etc/os-release");
        assert_eq!(v.observed_at_utc, "1970-01-01T00:00:01.234Z");
    }

    #[test]
    fn unquoted_and_single_quoted_values_are_accepted() {
        assert_eq!(
            sample("ID=arch\nVERSION=26.11.20261108\n").unwrap().version,
            "26.11.20261108"
        );
        assert_eq!(
            sample("VERSION='CachyOS (26.11.20261108)'\n")
                .unwrap()
                .version,
            "CachyOS (26.11.20261108)"
        );
    }

    #[test]
    fn malformed_fields_are_unavailable_not_empty() {
        for text in [
            "",
            "NAME=x\nVERSION_ID=12\n",
            "VERSION=\n",
            "VERSION=\"\"\n",
            "VERSION=\"\n",
            "VERSION=a\"\n",
            "VERSION=\"a\" \"b\"\n",
            "VERSION= \"12 (bookworm)\"\n",
            "VERSION=12 (bookworm) \n",
            "VERSION=12 (bookworm)\nVERSION=13\n",
            "VERSION=\"12\t(bookworm)\"\n",
            "VERSION=\"12\x01(bookworm)\"\n",
            "VERSION=\u{4242}\u{435}\n",
            &format!("VERSION=\"{} (bookworm)\"\n", "1".repeat(65)),
        ] {
            assert!(sample(text).is_none(), "{text:?}");
        }
    }

    #[test]
    fn missing_clock_or_unreadable_source_is_not_a_sample() {
        let text = &b"VERSION=12 (bookworm)\n"[..];
        assert!(observe(Ok(text), || UNIX_EPOCH - Duration::from_secs(1)).is_none());
        assert!(
            observe(
                Err::<&[u8], _>(io::ErrorKind::PermissionDenied.into()),
                SystemTime::now
            )
            .is_none()
        );
        let big = format!("VERSION=12\n{}\n", "NAME=x".repeat(11_000));
        assert!(observe(Ok(big.as_bytes()), SystemTime::now).is_none());
    }
}

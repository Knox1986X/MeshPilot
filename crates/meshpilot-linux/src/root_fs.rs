//! Один statvfs для `/` в пространстве монтирования процесса. Без обхода путей и запуска df.
use meshpilot_core::RootFsObservation;

#[cfg(any(target_os = "linux", test))]
use std::time::SystemTime;

#[cfg(any(target_os = "linux", test))]
struct Blocks {
    fragment_size: u64,
    total: u64,
    free: u64,
    available: u64,
}

/// Путь не является аргументом API или IPC.
pub fn root_fs_snapshot() -> Option<RootFsObservation> {
    #[cfg(target_os = "linux")]
    {
        let stats = rustix::fs::statvfs("/").ok().map(|s| Blocks {
            fragment_size: s.f_frsize,
            total: s.f_blocks,
            free: s.f_bfree,
            available: s.f_bavail,
        });
        observe(stats, SystemTime::now())
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(any(target_os = "linux", test))]
fn observe(stats: Option<Blocks>, at: SystemTime) -> Option<RootFsObservation> {
    let b = stats?;
    let total_bytes = b.total.checked_mul(b.fragment_size)?;
    if !(1..=crate::MAX_SAFE_INTEGER).contains(&total_bytes) || b.available > b.free {
        return None;
    }
    // Свободные, но недоступные обычному пользователю блоки не заняты и не доступны ему.
    Some(RootFsObservation {
        used_bytes: b.total.checked_sub(b.free)?.checked_mul(b.fragment_size)?,
        available_bytes: b.available.checked_mul(b.fragment_size)?,
        total_bytes,
        source: "statvfs(/)",
        observed_at_utc: crate::utc_stamp(at)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn sample(size: u64, total: u64, free: u64, available: u64) -> Option<RootFsObservation> {
        observe(
            Some(Blocks {
                fragment_size: size,
                total,
                free,
                available,
            }),
            UNIX_EPOCH + Duration::from_millis(1234),
        )
    }

    #[test]
    fn free_is_not_available_and_reserved_space_is_not_used() {
        let s = sample(1024, 40, 17, 12).unwrap();
        assert_eq!(
            (s.used_bytes, s.total_bytes, s.available_bytes),
            (23 * 1024, 40 * 1024, 12 * 1024)
        );
        assert_eq!(s.source, "statvfs(/)");
        assert_eq!(s.observed_at_utc, "1970-01-01T00:00:01.234Z");
    }

    #[test]
    fn zero_used_and_zero_available_are_valid_independently() {
        let empty = sample(4096, 3, 3, 2).unwrap();
        assert_eq!((empty.used_bytes, empty.available_bytes), (0, 8192));
        let reserved_only = sample(4096, 3, 1, 0).unwrap();
        assert_eq!(
            (reserved_only.used_bytes, reserved_only.available_bytes),
            (8192, 0)
        );
        let full = sample(1, crate::MAX_SAFE_INTEGER, 0, 0).unwrap();
        assert_eq!(full.used_bytes, full.total_bytes);
    }

    #[test]
    fn impossible_counts_and_lossy_or_overflowing_sizes_are_not_reported() {
        for (size, total, free, available) in [
            (0, 1, 1, 0),
            (4096, 0, 0, 0),
            (1, 3, 4, 2),
            (1, 5, 3, 4),
            (1, 1, 1, u64::MAX),
            (4096, u64::MAX, 0, 0),
            (4096, 1 << 41, 0, 0),
        ] {
            assert!(sample(size, total, free, available).is_none());
        }
    }

    #[test]
    fn failed_stat_or_clock_is_unavailable_not_zero() {
        assert!(observe(None, UNIX_EPOCH).is_none());
        assert!(
            observe(
                Some(Blocks {
                    fragment_size: 1,
                    total: 1,
                    free: 0,
                    available: 0
                }),
                UNIX_EPOCH - Duration::from_secs(1)
            )
            .is_none()
        );
    }
}

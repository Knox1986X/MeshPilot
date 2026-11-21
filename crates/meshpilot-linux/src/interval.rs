//! Один интервал для независимых счётчиков: CLI не ждёт последовательно по 500 мс на источник.
use crate::utc_stamp;
use std::{
    thread,
    time::{Duration, Instant, SystemTime},
};

pub(super) const GAP: Duration = Duration::from_millis(500);
const MAX: Duration = Duration::from_secs(5);

pub(super) struct Window {
    pub ms: u64,
    pub utc: String,
}

impl Window {
    pub fn new(start: Instant, end: Instant, at: SystemTime) -> Option<Self> {
        let elapsed = end.checked_duration_since(start)?;
        if !(GAP..=MAX).contains(&elapsed) {
            return None;
        }
        Some(Self {
            ms: elapsed.as_millis().try_into().ok()?,
            utc: utc_stamp(at)?,
        })
    }
}

pub(super) trait Probe: Sized {
    type Report;
    fn read() -> Option<Self>;
    fn report(self, next: Self, window: Window) -> Option<Self::Report>;
}

struct First<P>(P, Instant);
impl<P: Probe> First<P> {
    fn read() -> Option<Self> {
        Some(Self(P::read()?, Instant::now()))
    }
    fn finish(self) -> Option<P::Report> {
        let next = P::read()?;
        self.0.report(
            next,
            Window::new(self.1, Instant::now(), SystemTime::now())?,
        )
    }
}

pub(super) fn single<P: Probe>() -> Option<P::Report> {
    let first = First::<P>::read()?;
    thread::sleep(GAP);
    first.finish()
}

pub(super) fn pair<A: Probe, B: Probe>(
    wait: impl FnOnce(Duration),
) -> (Option<A::Report>, Option<B::Report>) {
    let a = First::<A>::read();
    let b = First::<B>::read();
    if a.is_some() || b.is_some() {
        wait(GAP);
    }
    (a.and_then(First::finish), b.and_then(First::finish))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, time::UNIX_EPOCH};

    #[test]
    fn accepts_only_bounded_monotonic_windows_with_a_utc_stamp() {
        let t = Instant::now();
        for dt in [
            Duration::ZERO,
            GAP - Duration::from_nanos(1),
            MAX + Duration::from_nanos(1),
        ] {
            assert!(Window::new(t, t + dt, UNIX_EPOCH).is_none());
        }
        assert!(Window::new(t, t + GAP, UNIX_EPOCH - Duration::from_secs(1)).is_none());
        let w = Window::new(t, t + MAX, UNIX_EPOCH + Duration::from_millis(1234)).unwrap();
        assert_eq!(w.ms, 5000);
        assert_eq!(w.utc, "1970-01-01T00:00:01.234Z");
    }

    #[test]
    fn failed_second_read_never_produces_a_report() {
        struct Gone;
        impl Probe for Gone {
            type Report = ();
            fn read() -> Option<Self> {
                None
            }
            fn report(self, _: Self, _: Window) -> Option<()> {
                panic!("нет второго чтения")
            }
        }
        assert!(First(Gone, Instant::now() - GAP).finish().is_none());
    }

    #[test]
    fn two_probes_share_one_wait_and_neither_retries_failed_reads() {
        thread_local! { static READS: Cell<usize> = const { Cell::new(0) }; }
        struct TestProbe<const PRESENT: bool>;
        impl<const PRESENT: bool> Probe for TestProbe<PRESENT> {
            type Report = ();
            fn read() -> Option<Self> {
                READS.set(READS.get() + 1);
                PRESENT.then_some(Self)
            }
            fn report(self, _: Self, _: Window) -> Option<()> {
                Some(())
            }
        }
        let mut waits = 0;
        let mut wait = |d| {
            assert_eq!(d, GAP);
            waits += 1;
        };
        pair::<TestProbe<true>, TestProbe<true>>(&mut wait);
        assert_eq!(READS.replace(0), 4);
        pair::<TestProbe<false>, TestProbe<true>>(&mut wait);
        assert_eq!(READS.replace(0), 3);
        pair::<TestProbe<false>, TestProbe<false>>(|_| panic!("нет показаний — нет паузы"));
        assert_eq!(READS.get(), 2);
        assert_eq!(waits, 2);
    }
}

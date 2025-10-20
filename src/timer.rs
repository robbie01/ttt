use std::{pin::Pin, rc::{Rc, Weak}, sync::atomic::{AtomicBool, Ordering}, task::{Context, Poll}, time::{Duration, Instant}};

use atomic_waker::AtomicWaker;

#[derive(Default)]
struct TimerInner {
    waker: AtomicWaker,
    flag: AtomicBool
}

pub struct Timer {
    inner: Option<Rc<TimerInner>>
}

impl Timer {
    pub fn at(instant: Instant) -> (PendingTimer, Timer) {
        let inner = Rc::<TimerInner>::default();
        (
            PendingTimer {
                at: instant,
                inner: Rc::downgrade(&inner)
            },
            Timer { inner: Some(inner) }
        )
    }

    pub fn after(duration: Duration) -> (PendingTimer, Timer) {
        let instant = Instant::now() + duration;
        Self::at(instant)
    }

    #[expect(dead_code)]
    pub fn never() -> Timer {
        Timer { inner: None }
    }
}

impl Future for Timer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Some(ref inner) = self.inner else { return Poll::Pending };

        if inner.flag.load(Ordering::Relaxed) {
            return Poll::Ready(())
        }

        inner.waker.register(cx.waker());

        if inner.flag.load(Ordering::Relaxed) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub struct PendingTimer {
    pub at: Instant,
    inner: Weak<TimerInner>
}

impl PendingTimer {
    pub fn set(self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.flag.store(true, Ordering::Relaxed);
            inner.waker.wake();
        }
    }
}

impl PartialEq for PendingTimer {
    fn eq(&self, other: &Self) -> bool {
        self.at.eq(&other.at)
    }
}

impl Eq for PendingTimer {}

impl PartialOrd for PendingTimer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PendingTimer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Order is correct. This is meant to be inserted into a max-heap (BinaryHeap)
        other.at.cmp(&self.at)
    }
}
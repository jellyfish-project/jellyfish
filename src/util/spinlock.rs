use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering, spin_loop_hint};

// SpinLock implementation.
//
// FIXME: use parking_lot_core::SpintWait?
struct SpinLock {
    busy: AtomicBool,
}

impl SpinLock {
    pub fn new() -> Self {
        SpinLock { busy: ATOMIC_BOOL_INIT }
    }

    pub fn lock(&mut self) {
        while ! self.busy.compare_and_swap(false, true, Ordering::Acquire) {
            spin_loop_hint();
        }
    }

    pub fn unlock(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}

impl Drop for SpinLock {
    fn drop(&mut self) {
        assert!(!self.busy.load(Ordering::Relaxed));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        let mut lock = SpinLock::new();
        lock.lock();
        lock.unlock();
    }
}

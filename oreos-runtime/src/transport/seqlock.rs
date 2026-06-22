use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering, compiler_fence};
use core::cell::UnsafeCell;

pub struct SeqLock<T: Copy> {
    seq: AtomicU32,
    slots: [UnsafeCell<T>; 2],
    index: AtomicUsize,
}

impl<T: Copy> SeqLock<T> {
    pub const fn new(initial: T) -> Self {
        Self {
            seq: AtomicU32::new(0),
            slots: [UnsafeCell::new(initial), UnsafeCell::new(initial)],
            index: AtomicUsize::new(0),
        }
    }
}

unsafe impl<T: Copy + Send> Sync for SeqLock<T> {}

impl<T: Copy> SeqLock<T> {
    pub fn write(&self, val: T) {
        let active = self.index.load(Ordering::Relaxed);
        let next = 1 - active;

        let seq = self.seq.load(Ordering::Relaxed);
        self.seq.store(seq + 1, Ordering::Relaxed);

        compiler_fence(Ordering::Release);
        unsafe {
            *self.slots[next].get() = val;
        }
        compiler_fence(Ordering::Release);
        self.index.store(next, Ordering::Release);

        self.seq.store(seq + 2, Ordering::Relaxed);
    }

    pub fn read(&self) -> T {
        loop {
            let seq_before = self.seq.load(Ordering::Relaxed);
            if seq_before & 1 != 0 {
                core::hint::spin_loop();
                continue;
            }

            compiler_fence(Ordering::Acquire);
            let index: usize = self.index.load(Ordering::Relaxed);
            let val = unsafe { *self.slots[index].get() };
            compiler_fence(Ordering::Acquire);

            let seq_after = self.seq.load(Ordering::Relaxed);
            if seq_after == seq_before {
                return val;
            }
        }
    }
}

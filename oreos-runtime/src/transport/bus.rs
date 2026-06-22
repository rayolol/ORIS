use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Watch};
use defmt::trace;

use crate::hal::Lane;
use super::seqlock::SeqLock;

pub struct FastLane<T: Copy> {
    inner: SeqLock<T>,
}

impl<T: Copy> FastLane<T> {
    pub const fn new(data: T) -> Self {
        Self { inner: SeqLock::new(data) }
    }
}

impl<T: Copy> Lane<T> for FastLane<T> {
    fn write(&self, data: T) {
        // trace!("bus fastlane got write");
        self.inner.write(data);
    }

    fn read(&self) -> Option<T> {
        // trace!("bus fastlane read");
        Some(self.inner.read())
    }
}

pub struct SlowLane<T: Copy> {
    inner: Watch<CriticalSectionRawMutex, T, 1>,
}

impl<T: Copy> SlowLane<T> {
    pub const fn new() -> Self {
        Self { inner: Watch::new() }
    }
}

impl<T: Copy> Lane<T> for SlowLane<T> {
    fn write(&self, data: T) {
        trace!("bus slowlane got write");
        self.inner.sender().send(data);
    }

    fn read(&self) -> Option<T> {
        trace!("bus slowlane got read");
        self.inner.receiver().unwrap().try_get()
    }
}

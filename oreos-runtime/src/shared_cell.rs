use critical_section;
use core::cell::UnsafeCell;

/// Copy-able handle to a shared device. Wraps `&'static SharedCell<T>`.
/// The `#[app]` macro rewrites `ctx.field.method()` into `ctx.field.0.with(|d| d.method())`.
#[derive(Copy, Clone)]
pub struct Shared<T: 'static>(pub &'static SharedCell<T>);

impl<T: 'static> core::ops::Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.0.inner.get() }
    }
}

impl<T: 'static> core::ops::DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.inner.get() }
    }
}

pub struct SharedCell<T> {
    inner: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SharedCell<T> {}

impl<T> SharedCell<T> {
    pub const fn new(val: T) -> Self {
        Self { inner: UnsafeCell::new(val) }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        critical_section::with(|_| {
            unsafe { f(&mut *self.inner.get()) }
        })
    }
}
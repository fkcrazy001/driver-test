use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicBool,
};
pub struct Mutex<T> {
    inner: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for Mutex<T> {}
unsafe impl<T> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            inner: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }
    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self.inner.swap(true, core::sync::atomic::Ordering::Acquire) {}
        MutexGuard { mutex: self }
    }
    pub fn unlock(&self) {
        self.inner
            .store(false, core::sync::atomic::Ordering::Release);
    }
    /// get inner on s
    pub unsafe fn force_use(&self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

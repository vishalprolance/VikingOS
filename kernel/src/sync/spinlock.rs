// VikingOS Kernel — Spinlock Implementation
//
// A robust, interrupt-safe atomic spinlock for bare-metal synchronization.
// Prevents deadlocks by disabling hardware interrupts while the lock is held.
// Provides safe RAII guards for concurrent access.

#![allow(dead_code)]

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};
use crate::arch::interrupts;

/// A spinlock providing mutual exclusion.
pub struct SpinLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Create a new unlocked spinlock wrapping `data`.
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Acquire the lock, spinning until it becomes available.
    /// Disables interrupts to prevent deadlocks if an interrupt handler
    /// attempts to acquire the same lock.
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        let interrupt_enabled = interrupts::save_and_disable();
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Pause instruction hints to the CPU that we are spinning, saving power and avoiding pipeline stalls
            core::hint::spin_loop();
        }
        SpinLockGuard {
            lock: self,
            interrupt_enabled,
        }
    }

    /// Attempt to acquire the lock without spinning.
    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        let interrupt_enabled = interrupts::save_and_disable();
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinLockGuard {
                lock: self,
                interrupt_enabled,
            })
        } else {
            interrupts::restore(interrupt_enabled);
            None
        }
    }

    /// Force unlock the spinlock.
    ///
    /// # Safety
    /// Must only be used in emergency contexts (e.g. panic handler) where deadlocks must be avoided.
    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }
}

/// An RAII guard providing mutable access to the protected data.
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    interrupt_enabled: bool,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.lock.store(false, Ordering::Release);
        interrupts::restore(self.interrupt_enabled);
    }
}

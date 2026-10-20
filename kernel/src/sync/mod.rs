// VikingOS Kernel — Synchronization Primitives
//
// Provides spinlocks and synchronization structures for bare-metal `#![no_std]` environment.

#![allow(dead_code)]

// ... (existing imports)
pub mod scheduler;
pub mod spinlock;

#[allow(unused_imports)]
pub use spinlock::{SpinLock, SpinLockGuard};

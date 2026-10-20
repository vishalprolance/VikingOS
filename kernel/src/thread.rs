// VikingOS Kernel — Thread Abstraction
//
// Defines the basic unit of execution within the kernel.
// Each thread has its own stack and execution state.
#![allow(dead_code)]

use crate::memory::PhysFrame;

/// Unique identifier for a thread.
pub type ThreadId = usize;

/// The current execution state of a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,      // Eligible to run
    Running,    // Currently executing on the CPU
    Blocked,    // Waiting for an event (e.g., I/O or Mutex)
    Terminated, // Finished execution, pending cleanup
}

/// A kernel-level thread of execution.
pub struct Thread {
    pub id: ThreadId,
    pub state: ThreadState,
    /// The current top of the stack (saved during context switch).
    pub stack_ptr: usize,
    /// The physical frame allocated for this thread's stack.
    pub kernel_stack: PhysFrame,
}

impl Thread {
    /// Create a new thread structure.
    ///
    /// # Safety
    /// The caller must ensure that `stack_ptr` points to a valid,
    /// pre-initialized stack frame containing the initial context.
    pub unsafe fn new(id: ThreadId, stack_ptr: usize, kernel_stack: PhysFrame) -> Self {
        Self {
            id,
            state: ThreadState::Ready,
            stack_ptr,
            kernel_stack,
        }
    }
}

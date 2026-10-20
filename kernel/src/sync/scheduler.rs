// VikingOS Kernel — Round-Robin Scheduler
//
// Manages the execution of multiple kernel threads.
// The scheduler is triggered by the LAPIC timer to ensure preemption.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use crate::sync::SpinLock;
use crate::thread::{Thread, ThreadId, ThreadState};
use crate::memory::{pmm, PhysFrame};
use crate::arch::context::switch_context;

/// The global scheduler state.
pub struct Scheduler {
    /// The queue of threads ready to run.
    pub ready_queue: VecDeque<ThreadId>,
    /// All existing threads, indexed by their ID.
    pub threads: Vec<Thread>,
    /// The ID of the thread currently executing on the CPU.
    pub current_thread: Option<ThreadId>,
    /// Counter for assigning unique thread IDs.
    pub next_id: ThreadId,
}

impl Scheduler {
    /// Create a new, empty scheduler.
    pub const fn new() -> Self {
        panic!("Scheduler cannot be const-initialized")
    }
}

/// Use a dynamic initialization pattern for the global scheduler.
pub static SCHEDULER: SpinLock<Option<Scheduler>> = SpinLock::new(None);

/// Initialize the global scheduler.
pub fn init() {
    let mut sched = SCHEDULER.lock();
    *sched = Some(Scheduler {
        ready_queue: VecDeque::new(),
        threads: Vec::new(),
        current_thread: None,
        next_id: 0,
    });
}

/// Helper to spawn a kernel thread.
pub fn spawn_thread(f: fn()) {
    let frame = pmm::allocate_frame().expect("Scheduler: Failed to allocate kernel stack frame");
    let mut sched = SCHEDULER.lock();
    if let Some(ref mut s) = *sched {
        let pid = s.next_id;
        s.next_id += 1;

        // Set up the initial stack frame for the thread
        let stack_top = (crate::memory::phys_to_virt(frame).as_u64() + 4096) as usize;
        let mut rsp = stack_top;
        unsafe {
            rsp -= 8;
            core::ptr::write(rsp as *mut usize, f as usize);
            for _ in 0..6 {
                rsp -= 8;
                core::ptr::write(rsp as *mut usize, 0);
            }
        }

        let thread = unsafe { Thread::new(pid, rsp, PhysFrame::new(frame)) };
        s.threads.push(thread);
        s.ready_queue.push_back(pid);
    }
}

impl Scheduler {
    /// Spawn a kernel thread.
    pub fn spawn_thread(&mut self, f: fn()) {
        let frame = pmm::allocate_frame().expect("Scheduler: Failed to allocate kernel stack frame");
        let pid = self.next_id;
        self.next_id += 1;

        let stack_top = (crate::memory::phys_to_virt(frame).as_u64() + 4096) as usize;
        let mut rsp = stack_top;
        unsafe {
            rsp -= 8;
            core::ptr::write(rsp as *mut usize, f as usize);
            for _ in 0..6 {
                rsp -= 8;
                core::ptr::write(rsp as *mut usize, 0);
            }
        }

        let thread = unsafe { Thread::new(pid, rsp, PhysFrame::new(frame)) };
        self.threads.push(thread);
        self.ready_queue.push_back(pid);
    }

    /// Spawn a user process.
    ///
    /// # Arguments
    /// * `entry_point` - The virtual address where the user process starts.
    /// * `user_stack` - The virtual address of the user-mode stack.
    /// * `cr3` - The physical address of the process's page table.
    pub fn spawn_process(&mut self, entry_point: crate::memory::VirtAddr, user_stack: crate::memory::VirtAddr, cr3: crate::memory::PhysAddr) {
        // 1. Allocate a kernel stack for the process
        let frame = pmm::allocate_frame().expect("Scheduler: Failed to allocate kernel stack frame");

        // 2. Create the Process object
        let pid = self.next_id;
        self.next_id += 1;

        let mut process = unsafe {
            crate::process::Process::new(
                crate::process::ProcessId(pid as u64),
                cr3,
                PhysFrame::new(frame),
                user_stack,
            )
        };

        // 3. Prepare the stack for the first transition to Ring 3
        let stack_ptr = process.prepare_user_transition(entry_point);

        // 4. We need to wrap this in a Thread structure for the scheduler to manage it.
        let thread = unsafe { Thread::new(pid, stack_ptr, PhysFrame::new(frame)) };

        self.threads.push(thread);
        self.ready_queue.push_back(pid);
    }

    /// Trigger a context switch to the next ready thread.
    pub fn schedule(&mut self) {
        let current_id = match self.current_thread {
            Some(id) => id,
            None => {
                // First time running the scheduler: just pick the first thread.
                if let Some(next_id) = self.ready_queue.pop_front() {
                    self.current_thread = Some(next_id);
                    let next_thread = &mut self.threads[next_id];
                    let stack_ptr = next_thread.stack_ptr;

                    // Since there is no "old" thread yet, we jump straight in.
                    unsafe {
                        core::arch::asm!(
                            "mov rsp, {0}",
                            "pop r15",
                            "pop r14",
                            "pop r13",
                            "pop r12",
                            "pop rbx",
                            "pop rbp",
                            "ret",
                            in(reg) stack_ptr,
                        );
                    }
                }
                return;
            }
        };

        // 1. Update current thread state
        if let Some(thread) = self.threads.get_mut(current_id) {
            thread.state = ThreadState::Ready;
            self.ready_queue.push_back(current_id);
        }

        // 2. Pick next thread
        if let Some(next_id) = self.ready_queue.pop_front() {
            self.current_thread = Some(next_id);

            let old_stack_ptr_addr = &mut self.threads[current_id].stack_ptr as *mut usize;
            let new_stack_ptr = self.threads[next_id].stack_ptr;

            unsafe {
                switch_context(old_stack_ptr_addr, new_stack_ptr);
            }
        }
    }
}

// VikingOS Kernel — Process Management
//
// Implements the Process abstraction, tracking execution state,
// address spaces (CR3), and stacks for user-mode execution.

#![allow(dead_code)]

use crate::memory::{pmm, PhysAddr, VirtAddr};
use crate::thread::ThreadState;

/// A unique identifier for a process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProcessId(pub u64);

/// Represents an embedded application binary.
pub struct UserBinary {
    pub name: &'static str,
    pub code: &'static [u8],
    pub entry_point: u64,
}

/// Represents a user-mode process in VikingOS.
pub struct Process {
    pub pid: ProcessId,
    /// The physical address of the PML4 table for this process.
    pub cr3: PhysAddr,
    /// The kernel stack used when this process is executing in ring 0.
    pub kernel_stack: pmm::PhysFrame,
    /// The user stack pointer.
    pub user_stack_ptr: VirtAddr,
    pub state: ThreadState,
}

impl Process {
    /// Create a new user process from an embedded binary.
    pub unsafe fn load_binary(
        pid: ProcessId,
        _binary: &UserBinary,
    ) -> Result<Self, &'static str> {
        // 1. Allocate a new PML4 for the process
        let cr3_frame = pmm::allocate_zeroed_frame().ok_or("Failed to allocate PML4")?;

        // 2. Allocate user stack
        let _stack_frame = pmm::allocate_zeroed_frame().ok_or("Failed to allocate user stack")?;
        let user_stack_ptr = VirtAddr::new(0x7FFFFFFFF000);

        // 3. Create the process object
        Ok(Self {
            pid,
            cr3: cr3_frame,
            kernel_stack: pmm::allocate_frame().ok_or("Failed to allocate kernel stack")?.into(),
            user_stack_ptr,
            state: ThreadState::Ready,
        })
    }

    /// Create a new user process.
    ///
    /// # Safety
    /// The caller must ensure that the provided page tables are valid
    /// and that the user stack is correctly mapped.
    pub unsafe fn new(
        pid: ProcessId,
        cr3: PhysAddr,
        kernel_stack: pmm::PhysFrame,
        user_stack_ptr: VirtAddr
    ) -> Self {
        Self {
            pid,
            cr3,
            kernel_stack,
            user_stack_ptr,
            state: ThreadState::Ready,
        }
    }

    /// Set up the interrupt return frame on the kernel stack to transition to Ring 3.
    ///
    /// This prepares the stack so that `iretq` will:
    /// 1. Switch to the User Code Segment (CS).
    /// 2. Switch to the User Stack Segment (SS).
    /// 3. Restore the User Stack Pointer (RSP).
    /// 4. Restore the RFLAGS register.
    /// 5. Jump to the User Entry Point (RIP).
    pub fn prepare_user_transition(&mut self, entry_point: VirtAddr) -> usize {
        // Calculate the top of the kernel stack
        let stack_base = crate::memory::phys_to_virt(self.kernel_stack.addr());
        let mut rsp = stack_base.as_u64() + 4096;

        // The iretq stack frame consists of 5 values (8 bytes each):
        // [Bottom]
        // SS    (User Data Selector)
        // RSP   (User Stack Pointer)
        // RFLAGS
        // CS    (User Code Selector)
        // RIP   (User Entry Point)
        // [Top]

        unsafe {
            // 1. RIP (Instruction Pointer)
            core::ptr::write(rsp as *mut u64, entry_point.as_u64());
            rsp -= 8;

            // 2. CS (Code Segment Selector)
            core::ptr::write(rsp as *mut u64, crate::arch::gdt::USER_CODE_SELECTOR as u64);
            rsp -= 8;

            // 3. RFLAGS (Enable interrupts)
            core::ptr::write(rsp as *mut u64, 0x202);
            rsp -= 8;

            // 4. RSP (User Stack Pointer)
            core::ptr::write(rsp as *mut u64, self.user_stack_ptr.as_u64());
            rsp -= 8;

            // 5. SS (Stack Segment Selector)
            core::ptr::write(rsp as *mut u64, crate::arch::gdt::USER_DATA_SELECTOR as u64);
            rsp -= 8;
        }

        rsp as usize
    }

    /// Switches the CPU to use this process's address space.
    pub fn activate(&self) {
        let phys_addr = self.cr3.as_u64();
        unsafe {
            // Load the process's root page table into CR3
            core::arch::asm!("mov cr3, {}", in(reg) phys_addr);
        }
    }
}

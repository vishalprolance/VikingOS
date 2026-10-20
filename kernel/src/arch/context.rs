// VikingOS Kernel — Low-Level Context Switching
//
// Provides the assembly primitives required to save and restore
// CPU register state during a thread switch.

/// Performs a context switch from the current thread to a new thread.
///
/// # Arguments
/// * `old_stack` - A mutable reference to the current thread's stack pointer.
///                The current register state will be saved here.
/// * `new_stack` - The stack pointer of the thread to switch to.
///                Its saved register state will be restored.
///
/// # Safety
/// This function is extremely low-level and manipulates the stack pointer directly.
/// It must only be called from a context where it is safe to switch stacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn switch_context(old_stack: *mut usize, new_stack: usize) {
    unsafe {
        core::arch::asm!(
            // 1. Save current context (Callee-saved registers)
            // According to System V AMD64 ABI, these must be preserved across calls.
            "push rbp",
            "push rbx",
            "push r12",
            "push r13",
            "push r14",
            "push r15",

            // 2. Save current RSP to old_stack memory location
            "mov [{0}], rsp",

            // 3. Restore new RSP
            "mov rsp, {1}",

            // 4. Restore new context
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop rbx",
            "pop rbp",

            // Return to the address pointed to by the new stack's RIP
            "ret",
            in(reg) old_stack,
            in(reg) new_stack,
            options(preserves_flags)
        );
    }
}

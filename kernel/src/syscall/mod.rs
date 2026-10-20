// VikingOS Kernel — Syscall Dispatcher
//
// This module handles the transition from user-mode (Ring 3) to
// kernel-mode (Ring 0) via the 'syscall' instruction.

use crate::drivers::console::Console;
use crate::sync::scheduler::SCHEDULER;
use crate::memory::heap::ALLOCATOR;
use core::alloc::{Layout, GlobalAlloc};

#[derive(Debug)]
pub enum SyscallNumber {
    Print = 0,
    Exit = 1,
    Malloc = 2,
    GetStats = 3,
    Kill = 4,
}

impl SyscallNumber {
    pub fn from_usize(num: usize) -> Option<Self> {
        match num {
            0 => Some(SyscallNumber::Print),
            1 => Some(SyscallNumber::Exit),
            2 => Some(SyscallNumber::Malloc),
            3 => Some(SyscallNumber::GetStats),
            4 => Some(SyscallNumber::Kill),
            _ => None,
        }
    }
}

/// The main entry point for system calls.
///
/// This function is called from the assembly syscall handler.
/// The handler is responsible for saving all user registers onto the
/// kernel stack before calling this dispatcher.
pub fn syscall_dispatcher(num: usize, arg1: usize, arg2: usize, _arg3: usize) -> isize {
    let syscall = match SyscallNumber::from_usize(num) {
        Some(s) => s,
        None => return -1, // ENOSYS: Function not implemented
    };

    match syscall {
        SyscallNumber::Print => {
            // arg1: buffer_ptr, arg2: length
            // Safety: We trust the user pointer for this basic implementation,
            // but in a production kernel, we would validate that the pointer
            // resides in user-space memory.
            let buf = unsafe { core::slice::from_raw_parts(arg1 as *const u8, arg2) };
            let msg = core::str::from_utf8(buf).unwrap_or("Invalid UTF-8");

            Console::print(msg);
            arg2 as isize
        }
        SyscallNumber::Exit => {
            // arg1: exit_code
            // Terminate the current process/thread.
            let mut sched = SCHEDULER.lock();
            if let Some(ref mut s) = *sched {
                if let Some(current_id) = s.current_thread {
                    if let Some(thread) = s.threads.iter_mut().find(|t| t.id == current_id) {
                        thread.state = crate::thread::ThreadState::Terminated;
                    }
                }
            }
            0
        }
        SyscallNumber::Malloc => {
            // arg1: size
            let size = arg1;
            if size == 0 {
                return 0;
            }

            let layout = Layout::from_size_align(size, 8).unwrap();
            let ptr = unsafe { ALLOCATOR.alloc(layout) };

            if ptr.is_null() {
                -1 // ENOMEM
            } else {
                ptr as isize
            }
        }
        SyscallNumber::GetStats => {
            // For simplicity, we return the free physical memory in KB as the result
            let stats = crate::memory::pmm::get_stats();
            (stats.free_bytes / 1024) as isize
        }
        SyscallNumber::Kill => {
            // arg1: pid
            let pid_to_kill = arg1 as u64;
            let mut sched = SCHEDULER.lock();
            if let Some(ref mut s) = *sched {
                for thread in s.threads.iter_mut() {
                    if thread.id as u64 == pid_to_kill {
                        thread.state = crate::thread::ThreadState::Terminated;
                    }
                }
            }
            0
        }
    }
}

/// Configures the CPU MSRs for the 'syscall' instruction.
pub unsafe fn init_syscall_msrs() {
    // IA32_STAR: Segment selectors for kernel and user
    // Bits [47:32]: Kernel CS = 0x08, Kernel SS = 0x10
    // Bits [63:48]: User CS/SS base = 0x13 (SYSRET: SS = 0x18|3 = 0x1B, CS = 0x20|3 = 0x23)
    let star_val: u64 = (0x0013u64 << 48) | (0x0008u64 << 32);
    write_msr(0xC0000081, star_val);

    // IA32_LSTAR: Address of the syscall handler
    let lstar_val = syscall_dispatcher as *const () as u64;
    write_msr(0xC0000082, lstar_val);

    // IA32_FMASK: Mask RFLAGS (disable interrupts on syscall)
    write_msr(0xC0000084, 0x200);
}

fn write_msr(msr: u32, val: u64) {
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") (val as u32),
            in("edx") ((val >> 32) as u32),
            options(nomem, nostack, preserves_flags),
        );
    }
}

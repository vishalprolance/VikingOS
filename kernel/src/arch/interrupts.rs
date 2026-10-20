// VikingOS Kernel — CPU Interrupt Control
//
// Provides primitives for enabling and disabling hardware interrupts.

#![allow(dead_code)]

/// Enable hardware interrupts (sti).
#[inline(always)]
pub fn enable() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Disable hardware interrupts (cli).
#[inline(always)]
pub fn disable() {
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Disable the legacy 8259 Programmable Interrupt Controller (PIC).
///
/// Masks all IRQ lines on both Master (0x21) and Slave (0xA1) PICs.
/// This prevents legacy PIC timer IRQ 0 from firing on CPU exception Vector 8 (Double Fault).
pub fn disable_pic() {
    unsafe {
        crate::arch::io::outb(0x21, 0xFF);
        crate::arch::io::outb(0xA1, 0xFF);
    }
}

/// Disables interrupts and returns the previous state.
/// Returns `true` if interrupts were enabled before the call.
#[inline(always)]
pub fn save_and_disable() -> bool {
    let flags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {0}",
            out(reg) flags,
            options(preserves_flags),
            options(nomem, nostack),
        );
    }
    // Bit 9 of RFLAGS is the Interrupt Flag (IF)
    (flags & (1 << 9)) != 0
}

/// Restores the interrupt state based on the provided boolean.
#[inline(always)]
pub fn restore(enabled: bool) {
    if enabled {
        enable();
    } else {
        disable();
    }
}

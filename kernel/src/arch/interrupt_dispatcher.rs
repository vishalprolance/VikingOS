// VikingOS Kernel — Hardware Interrupt Dispatcher
//
// This module handles the transition from low-level interrupt trampolines
// to high-level Rust handler logic.

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};
use crate::arch::apic::LAPIC;
use crate::kprintln;
use crate::drivers::keyboard::Keyboard;

/// Hardware Vector Mapping
pub const IRQ_TIMER: u8 = 32;
pub const IRQ_KEYBOARD: u8 = 33;
pub const IRQ_MOUSE: u8 = 44;

static TICKS: AtomicU64 = AtomicU64::new(0);

/// The main entry point for all hardware interrupts (vectors 32..255).
///
/// This function is called by the assembly trampolines after they have
/// saved the CPU state.
#[unsafe(no_mangle)]
pub extern "C" fn rust_hardware_interrupt_dispatcher(vector: u8) {
    match vector {
        IRQ_TIMER => handle_timer_interrupt(),
        IRQ_KEYBOARD => {
            Keyboard::handle_interrupt();
        }
        IRQ_MOUSE => {
            crate::drivers::mouse::handle_interrupt();
        }
        _ => {
            kprintln!("[Interrupt] Unhandled hardware interrupt: vector {}", vector);
        }
    }

    // Signal End of Interrupt (EOI) to the LAPIC
    LAPIC.send_eoi();
}

/// Handler for the LAPIC timer interrupt.
fn handle_timer_interrupt() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Retrieve current timer tick count (100Hz = 10ms per tick).
pub fn get_ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

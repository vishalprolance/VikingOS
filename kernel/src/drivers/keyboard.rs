// VikingOS Kernel — PS/2 Keyboard Driver
//
// Handles keyboard input via I/O ports 0x60 and 0x64.

#![allow(dead_code)]

use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::arch::io::inb;
use crate::kprintln;
use crate::sync::SpinLock;

/// Keyboard controller ports.
const KBD_DATA_PORT: u16 = 0x60;
const KBD_STATUS_PORT: u16 = 0x64;

pub const KEY_ALT_F4: char = '\x04';

static KEY_QUEUE: SpinLock<Option<VecDeque<char>>> = SpinLock::new(None);
static ALT_HELD: AtomicBool = AtomicBool::new(false);

/// A simple mapping from PS/2 scan codes to ASCII.
/// This is a partial map for demonstration.
fn scan_code_to_ascii(scancode: u8) -> Option<char> {
    match scancode {
        0x01 => Some('\x1B'), // Esc
        0x02 => Some('1'), 0x03 => Some('2'), 0x04 => Some('3'), 0x05 => Some('4'),
        0x06 => Some('5'), 0x07 => Some('6'), 0x08 => Some('7'), 0x09 => Some('8'),
        0x0A => Some('9'), 0x0B => Some('0'), 0x0C => Some('-'), 0x0D => Some('='),
        0x0E => Some('\x08'), // Backspace
        0x0F => Some('\t'),   // Tab
        0x10 => Some('q'), 0x11 => Some('w'), 0x12 => Some('e'), 0x13 => Some('r'),
        0x14 => Some('t'), 0x15 => Some('y'), 0x16 => Some('u'), 0x17 => Some('i'),
        0x18 => Some('o'), 0x19 => Some('p'), 0x1A => Some('['), 0x1B => Some(']'),
        0x1C => Some('\n'), // Enter
        0x1E => Some('a'), 0x1F => Some('s'), 0x20 => Some('d'), 0x21 => Some('f'),
        0x22 => Some('g'), 0x23 => Some('h'), 0x24 => Some('j'), 0x25 => Some('k'),
        0x26 => Some('l'), 0x27 => Some(';'), 0x28 => Some('\''),
        0x2C => Some('z'), 0x2D => Some('x'), 0x2E => Some('c'), 0x2F => Some('v'),
        0x30 => Some('b'), 0x31 => Some('n'), 0x32 => Some('m'), 0x33 => Some(','),
        0x34 => Some('.'), 0x35 => Some('/'), 0x39 => Some(' '),
        _ => None,
    }
}

pub struct Keyboard;

impl Keyboard {
    /// Initialize the keyboard controller.
    pub fn init() {
        ALT_HELD.store(false, Ordering::SeqCst);
        let mut q = KEY_QUEUE.lock();
        *q = Some(VecDeque::new());
    }

    /// Read a scancode from the keyboard data port, redirecting mouse bytes.
    ///
    /// # Safety
    /// Reading from I/O ports can have arbitrary side effects.
    pub unsafe fn read_scancode() -> Option<u8> {
        unsafe {
            let status = inb(KBD_STATUS_PORT);
            if (status & 0x01) != 0 {
                let data = inb(KBD_DATA_PORT);
                if (status & 0x20) != 0 {
                    // AUX byte = Mouse! Forward to Mouse driver
                    crate::drivers::mouse::MOUSE.lock().process_byte(data);
                    return None;
                } else {
                    return Some(data);
                }
            }
        }
        None
    }

    /// Poll keyboard controller directly.
    pub fn poll() {
        Self::handle_interrupt();
    }

    /// Handle a keyboard interrupt.
    pub fn handle_interrupt() {
        unsafe {
            if let Some(scancode) = Self::read_scancode() {
                // Check Alt key press (0x38) and release (0xB8)
                if scancode == 0x38 {
                    ALT_HELD.store(true, Ordering::SeqCst);
                    return;
                } else if scancode == 0xB8 {
                    ALT_HELD.store(false, Ordering::SeqCst);
                    return;
                }

                // Check F4 press (0x3E) while Alt is held -> Alt+F4
                if scancode == 0x3E && ALT_HELD.load(Ordering::SeqCst) {
                    kprintln!("[Keyboard] Alt+F4 detected -> Close active window");
                    if let Some(ref mut queue) = *KEY_QUEUE.lock() {
                        queue.push_back(KEY_ALT_F4);
                    }
                    return;
                }

                // Only handle "press" events (scancodes > 0x80 are "release" events)
                if scancode < 0x80 {
                    if let Some(c) = scan_code_to_ascii(scancode) {
                        kprintln!("[Keyboard] Received: {}", c);
                        if let Some(ref mut queue) = *KEY_QUEUE.lock() {
                            queue.push_back(c);
                        }
                    } else {
                        kprintln!("[Keyboard] Received unknown scancode: {:#04x}", scancode);
                    }
                }
            }
        }
    }

    /// Retrieve the next key from the queue, if available.
    pub fn pop_key() -> Option<char> {
        if let Some(ref mut queue) = *KEY_QUEUE.lock() {
            queue.pop_front()
        } else {
            None
        }
    }
}

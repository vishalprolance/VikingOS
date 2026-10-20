// VikingOS Kernel — PS/2 Mouse Driver
//
// Handles PS/2 mouse initialization, 3-byte packet parsing,
// coordinate tracking, and cursor rendering.

#![allow(dead_code)]

use crate::arch::io::{inb, outb};
use crate::drivers::gui::Canvas;

const PS2_DATA_PORT: u16 = 0x60;
const PS2_STATUS_PORT: u16 = 0x64;
const PS2_COMMAND_PORT: u16 = 0x64;

#[derive(Debug, Clone, Copy, Default)]
pub struct MouseState {
    pub x: usize,
    pub y: usize,
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
}

pub struct Mouse {
    state: MouseState,
    max_x: usize,
    max_y: usize,
    packet: [u8; 3],
    packet_index: usize,
}

impl Mouse {
    pub const fn new() -> Self {
        Self {
            state: MouseState {
                x: 100,
                y: 100,
                left_button: false,
                right_button: false,
                middle_button: false,
            },
            max_x: 1024,
            max_y: 768,
            packet: [0; 3],
            packet_index: 0,
        }
    }

    pub fn set_bounds(&mut self, width: usize, height: usize) {
        self.max_x = width.saturating_sub(1);
        self.max_y = height.saturating_sub(1);
        if self.state.x > self.max_x {
            self.state.x = self.max_x;
        }
        if self.state.y > self.max_y {
            self.state.y = self.max_y;
        }
    }

    /// Initialize the PS/2 auxiliary mouse device and 8042 controller.
    pub fn init(&mut self) {
        unsafe {
            // Wait for controller input buffer to be clear
            self.wait_write();
            // Enable keyboard interface
            outb(PS2_COMMAND_PORT, 0xAE);

            // Enable auxiliary mouse device
            self.wait_write();
            outb(PS2_COMMAND_PORT, 0xA8);

            // Enable interrupts in command byte
            self.wait_write();
            outb(PS2_COMMAND_PORT, 0x20); // Get Compaq status byte
            self.wait_read();
            let mut status = inb(PS2_DATA_PORT) | 0x03; // Enable IRQ 1 (KBD) and IRQ 12 (Mouse)
            status &= !0x30; // Enable keyboard and mouse clock lines
            status |= 0x40; // Translation enabled

            self.wait_write();
            outb(PS2_COMMAND_PORT, 0x60); // Set command byte
            self.wait_write();
            outb(PS2_DATA_PORT, status);

            // Tell mouse to use default settings
            self.mouse_write(0xF6);
            let _ = self.mouse_read(); // ACK

            // Enable data streaming
            self.mouse_write(0xF4);
            let _ = self.mouse_read(); // ACK
        }
    }

    /// Poll mouse data directly from port 0x60 if available.
    pub fn poll(&mut self) -> Option<MouseState> {
        unsafe {
            let status = inb(PS2_STATUS_PORT);
            if (status & 0x01) != 0 && (status & 0x20) != 0 {
                let byte = inb(PS2_DATA_PORT);
                return self.process_byte(byte);
            }
        }
        None
    }

    unsafe fn wait_write(&self) {
        let mut timeout = 100_000;
        while timeout > 0 {
            if unsafe { inb(PS2_STATUS_PORT) } & 0x02 == 0 {
                break;
            }
            timeout -= 1;
        }
    }

    unsafe fn wait_read(&self) {
        let mut timeout = 100_000;
        while timeout > 0 {
            if unsafe { inb(PS2_STATUS_PORT) } & 0x01 != 0 {
                break;
            }
            timeout -= 1;
        }
    }

    unsafe fn mouse_write(&self, val: u8) {
        unsafe {
            self.wait_write();
            outb(PS2_COMMAND_PORT, 0xD4); // Tell controller next byte goes to mouse
            self.wait_write();
            outb(PS2_DATA_PORT, val);
        }
    }

    unsafe fn mouse_read(&self) -> u8 {
        unsafe {
            self.wait_read();
            inb(PS2_DATA_PORT)
        }
    }

    /// Feed a byte received from the mouse data port.
    pub fn process_byte(&mut self, byte: u8) -> Option<MouseState> {
        // Sync check: bit 3 of the first packet byte must always be 1
        if self.packet_index == 0 && (byte & 0x08) == 0 {
            return None; // Drop unsynchronized byte
        }

        self.packet[self.packet_index] = byte;
        self.packet_index += 1;

        if self.packet_index == 3 {
            self.packet_index = 0;
            self.parse_packet();
            return Some(self.state);
        }

        None
    }

    fn parse_packet(&mut self) {
        let flags = self.packet[0];
        let mut dx = self.packet[1] as i32;
        let mut dy = self.packet[2] as i32;

        // Apply 9-bit signs
        if (flags & 0x10) != 0 {
            dx -= 256;
        }
        if (flags & 0x20) != 0 {
            dy -= 256;
        }

        // PS/2 Y is positive upwards, screen coordinates are positive downwards
        let new_x = (self.state.x as i32 + dx).clamp(0, self.max_x as i32) as usize;
        let new_y = (self.state.y as i32 - dy).clamp(0, self.max_y as i32) as usize;

        self.state.x = new_x;
        self.state.y = new_y;
        self.state.left_button = (flags & 0x01) != 0;
        self.state.right_button = (flags & 0x02) != 0;
        self.state.middle_button = (flags & 0x04) != 0;
    }

    pub fn state(&self) -> MouseState {
        self.state
    }

    /// Render the mouse pointer arrow on the canvas.
    pub fn draw(&self, canvas: &mut Canvas) {
        draw_cursor(canvas, self.state.x, self.state.y);
    }
}

/// Standalone helper to draw cursor bitmap at specified coordinates.
pub fn draw_cursor(canvas: &mut Canvas, x: usize, y: usize) {
    // A classic 12x16 arrow cursor bitmap
    #[rustfmt::skip]
    const CURSOR_BITMAP: [&str; 16] = [
        "#               ",
        "##              ",
        "#.#             ",
        "#..#            ",
        "#...#           ",
        "#....#          ",
        "#.....#         ",
        "#......#        ",
        "#.......#       ",
        "#.....####      ",
        "#..#..#         ",
        "#.# #..#        ",
        "##   #..#       ",
        "#     #..#      ",
        "       ##       ",
        "                ",
    ];

    for (row, line) in CURSOR_BITMAP.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            match ch {
                '#' => canvas.draw_pixel(x + col, y + row, 0x000000), // Black outline
                '.' => canvas.draw_pixel(x + col, y + row, 0xFFFFFF), // White fill
                _ => {}
            }
        }
    }
}

pub static MOUSE: crate::sync::SpinLock<Mouse> = crate::sync::SpinLock::new(Mouse::new());

/// Handle a PS/2 mouse hardware interrupt.
pub fn handle_interrupt() {
    unsafe {
        let status = inb(PS2_STATUS_PORT);
        if (status & 0x01) != 0 && (status & 0x20) != 0 {
            let byte = inb(PS2_DATA_PORT);
            MOUSE.lock().process_byte(byte);
        }
    }
}

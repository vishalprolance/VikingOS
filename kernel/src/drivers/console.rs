// VikingOS Kernel — Text Console
//
// Provides a high-level interface for printing text to the framebuffer.
// Manages the cursor position and handles line wrapping and scrolling.

use core::fmt::{self, Write};
use crate::drivers::framebuffer::Framebuffer;
use crate::drivers::font::draw_char;
use crate::sync::SpinLock;

/// A text console that renders to a framebuffer.
pub struct Console {
    fb: Framebuffer,
    cursor_x: usize,
    cursor_y: usize,
    char_width: usize,
    char_height: usize,
    fg_color: u32,
    bg_color: u32,
}

impl Console {
    /// Create a new console.
    pub fn new(fb: Framebuffer) -> Self {
        Self {
            fb,
            cursor_x: 0,
            cursor_y: 0,
            char_width: 8,
            char_height: 8,
            fg_color: 0xFFFFFFFF, // White
            bg_color: 0x00000000, // Black
        }
    }

    /// Print a string to the global console (and serial port).
    pub fn print(s: &str) {
        let mut lock = CONSOLE.lock();
        if let Some(ref mut console) = *lock {
            let _ = console.write_str(s);
        }
        crate::serial::SerialPort::com1().write_str_serial(s);
    }

    /// Clears the entire screen and resets the cursor.
    pub fn clear(&mut self) {
        self.fb.clear(self.bg_color);
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    /// Advances the cursor to the next line.
    pub fn newline(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += self.char_height;

        // Simple scrolling: clear screen if we hit the bottom
        if self.cursor_y + self.char_height > self.fb.height {
            self.clear();
        }
    }

    /// Renders a single character and advances the cursor.
    pub fn put_char(&mut self, c: char) {
        // Handle newline explicitly
        if c == '\n' {
            self.newline();
            return;
        }

        // Handle carriage return
        if c == '\r' {
            self.cursor_x = 0;
            return;
        }

        // Line wrapping
        if self.cursor_x + self.char_width > self.fb.width {
            self.newline();
        }

        // Draw the character
        draw_char(&mut self.fb, self.cursor_x, self.cursor_y, c, self.fg_color);

        // Advance cursor
        self.cursor_x += self.char_width;

        // If we reached the end of the line, move to next
        if self.cursor_x + self.char_width > self.fb.width {
            self.newline();
        }
    }
}

/// Implement core::fmt::Write so the console can be used with write! and writeln!
impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.put_char(c);
        }
        Ok(())
    }
}

/// Global console protected by a SpinLock.
pub static CONSOLE: SpinLock<Option<Console>> = SpinLock::new(None);

/// Initialize the global console.
pub fn init(fb: Framebuffer) {
    let mut lock = CONSOLE.lock();
    *lock = Some(Console::new(fb));
}

/// Helper function to print a formatted string to the global console and serial.
pub fn _kprintln(args: fmt::Arguments) {
    let mut lock = CONSOLE.lock();
    if let Some(ref mut console) = *lock {
        let _ = console.write_fmt(args);
        let _ = console.write_str("\n");
    }
    // Also mirror to COM1 serial port for virtual machines & diagnostics
    crate::serial::_print(args);
    crate::serial::_print(core::format_args!("\n"));
}

#[macro_export]
macro_rules! kprintln {
    () => {
        $crate::drivers::console::_kprintln(core::format_args!("\n"))
    };
    ($($arg:tt)*) => {
        $crate::drivers::console::_kprintln(core::format_args!($($arg)*))
    };
}

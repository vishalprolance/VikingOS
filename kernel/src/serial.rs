// VikingOS Kernel — Serial Port Driver
//
// Provides basic UART serial output via x86 I/O port 0x3F8 (COM1).
// Used for kernel diagnostics before a framebuffer console is available.

use core::fmt::{self, Write};
use crate::sync::SpinLock;
use crate::arch::io::{inb, outb};

/// Standard PC COM1 serial port base address.
const COM1_PORT: u16 = 0x3F8;

/// Line Status Register offset — bit 5 indicates transmit buffer empty.
const LSR_OFFSET: u16 = 5;

/// Transmit Holding Register Empty bit mask.
const LSR_THRE: u8 = 0x20;

/// Global synchronized serial port handle.
pub static SERIAL: SpinLock<SerialPort> = SpinLock::new(SerialPort::com1());

/// Initialize the global COM1 serial port.
pub fn init() {
    SERIAL.lock().init();
}

/// Print formatted arguments to the global serial port in a thread-safe manner.
pub fn _print(args: fmt::Arguments) {
    let mut serial = SERIAL.lock();
    let _ = serial.write_fmt(args);
}

/// Formatted print macro sending output to COM1 serial console.
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*))
    };
}

/// Formatted print macro sending output with newline to COM1 serial console.
// This is now deprecated in favor of the Graphical Console kprintln!
// Keep it here for early boot diagnostics.
#[allow(unused)]
macro_rules! serial_kprintln {
    () => {
        $crate::kprint!("\n")
    };
    ($($arg:tt)*) => {
        $crate::kprint!("{}\n", format_args!($($arg)*))
    };
}

/// A minimal UART serial port driver for x86.
pub struct SerialPort {
    port: u16,
}

impl SerialPort {
    /// Create a new serial port handle for the given base I/O port.
    pub const fn new(port: u16) -> Self {
        Self { port }
    }

    /// Create a handle for COM1 (0x3F8).
    pub const fn com1() -> Self {
        Self::new(COM1_PORT)
    }

    /// Initialize the serial port hardware.
    ///
    /// Configures: 115200 baud, 8N1, FIFO enabled, RTS/DSR set.
    pub fn init(&self) {
        unsafe {
            // Disable all interrupts
            outb(self.port + 1, 0x00);
            // Enable DLAB (Divisor Latch Access Bit) to set baud rate
            outb(self.port + 3, 0x80);
            // Set divisor to 1 (115200 baud)
            outb(self.port + 0, 0x01); // Low byte
            outb(self.port + 1, 0x00); // High byte
            // 8 bits, no parity, one stop bit (8N1), disable DLAB
            outb(self.port + 3, 0x03);
            // Enable FIFO, clear them, 14-byte threshold
            outb(self.port + 2, 0xC7);
            // IRQs enabled, RTS/DSR set, auxiliary output 2
            outb(self.port + 4, 0x0B);
            // Enable loopback mode for self-test
            outb(self.port + 4, 0x1E);
            // Send test byte
            outb(self.port + 0, 0xAE);
            // Check that we receive the same byte back
            if inb(self.port + 0) != 0xAE {
                // Serial port is faulty — nothing we can do, but continue anyway
                // since this is our only output mechanism
            }
            // Disable loopback, set OUT1 and OUT2 bits
            outb(self.port + 4, 0x0F);
        }
    }

    /// Write a single byte, waiting for the transmit buffer to be ready.
    pub fn write_byte(&self, byte: u8) {
        unsafe {
            // Spin until the transmit holding register is empty
            while (inb(self.port + LSR_OFFSET) & LSR_THRE) == 0 {
                core::hint::spin_loop();
            }
            outb(self.port, byte);
        }
    }

    /// Write a string, converting `\n` to `\r\n` for terminal compatibility.
    pub fn write_str_serial(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

/// Implement `core::fmt::Write` so we can use `write!` / `writeln!` macros.
impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str_serial(s);
        Ok(())
    }
}

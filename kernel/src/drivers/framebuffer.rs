// VikingOS Kernel — Framebuffer Driver
//
// Provides low-level access to the linear framebuffer provided by Limine.

/// A linear framebuffer for graphical output.
#[derive(Clone, Copy, Debug)]
pub struct Framebuffer {
    /// Virtual address of the framebuffer start.
    pub address: usize,
    pub width: usize,
    pub height: usize,
    /// Bits per pixel (typically 32).
    pub bpp: usize,
    /// The distance in bytes between the start of one scanline and the next.
    pub pitch: usize,
}

impl Framebuffer {
    /// Create a new Framebuffer from a Limine framebuffer response.
    ///
    /// # Safety
    /// The provided address must be a valid virtual or physical address.
    pub unsafe fn new(address: usize, width: usize, height: usize, bpp: usize, pitch: usize) -> Self {
        let virt_addr = if address >= 0xFFFF_8000_0000_0000 {
            address
        } else {
            crate::memory::phys_to_virt(crate::memory::PhysAddr::new(address as u64)).as_u64() as usize
        };

        Self {
            address: virt_addr,
            width,
            height,
            bpp,
            pitch,
        }
    }
}

impl Framebuffer {
    /// Draw a single pixel at the specified coordinates.
    ///
    /// # Arguments
    /// * `x` - X coordinate.
    /// * `y` - Y coordinate.
    /// * `color` - Color as a 32-bit integer (0xRRGGBB).
    pub fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }

        // Calculate the offset into the framebuffer.
        // Offset = (y * pitch) + (x * bytes_per_pixel)
        let bpp_bytes = self.bpp / 8;
        let offset = (y * self.pitch) + (x * bpp_bytes);

        unsafe {
            let ptr = (self.address + offset) as *mut u8;

            // We assume 32-bit RGBA/BGRX.
            // Limine usually provides the format, but for simplicity,
            // we'll write the 4 bytes of the u32 color.
            let color_bytes = color.to_le_bytes();
            core::ptr::write_bytes(ptr, color_bytes[0], 1);
            core::ptr::write_bytes(ptr.add(1), color_bytes[1], 1);
            core::ptr::write_bytes(ptr.add(2), color_bytes[2], 1);
            core::ptr::write_bytes(ptr.add(3), color_bytes[3], 1);
        }
    }

    /// Fill the entire screen with a single color.
    pub fn clear(&mut self, color: u32) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, color);
            }
        }
    }
}

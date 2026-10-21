// VikingOS Kernel — GUI Engine & Window Server (Milestone 10)
//
// Provides a desktop graphical user interface with 2D rendering primitives,
// interactive window management, taskbar with system tray, start menu,
// and UI widgets.

#![allow(dead_code)]

use crate::drivers::framebuffer::Framebuffer;
use crate::drivers::font;
use alloc::vec::Vec;

/// A high-level drawing canvas operating on an off-screen double buffer
/// to ensure atomic, 100% flicker-free rendering.
pub struct Canvas<'a> {
    fb: &'a mut Framebuffer,
    backbuffer: Vec<u32>,
}

impl<'a> Canvas<'a> {
    pub fn new(fb: &'a mut Framebuffer) -> Self {
        let size = fb.width * fb.height;
        let mut backbuffer = Vec::with_capacity(size);
        backbuffer.resize(size, 0);
        Self { fb, backbuffer }
    }

    #[inline(always)]
    pub fn width(&self) -> usize {
        self.fb.width
    }

    #[inline(always)]
    pub fn height(&self) -> usize {
        self.fb.height
    }

    /// Draw a single pixel directly into the memory backbuffer.
    #[inline(always)]
    pub fn draw_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.fb.width && y < self.fb.height {
            self.backbuffer[y * self.fb.width + x] = color;
        }
    }

    /// Draw a filled rectangle in the backbuffer.
    pub fn draw_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let width = self.fb.width;
        let height = self.fb.height;
        if x >= width || y >= height {
            return;
        }
        let max_w = w.min(width - x);
        let max_h = h.min(height - y);

        for row in y..(y + max_h) {
            let row_offset = row * width;
            for col in x..(x + max_w) {
                self.backbuffer[row_offset + col] = color;
            }
        }
    }

    /// Draw a 1-pixel rectangle outline.
    pub fn draw_rect_outline(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        if w == 0 || h == 0 {
            return;
        }
        // Top and bottom horizontal lines
        self.draw_rect(x, y, w, 1, color);
        self.draw_rect(x, y + h.saturating_sub(1), w, 1, color);
        // Left and right vertical lines
        self.draw_rect(x, y, 1, h, color);
        self.draw_rect(x + w.saturating_sub(1), y, 1, h, color);
    }

    /// Draw a vertical gradient between two 24-bit colors into the backbuffer.
    pub fn draw_gradient_v(&mut self, x: usize, y: usize, w: usize, h: usize, top_col: u32, bot_col: u32) {
        let width = self.fb.width;
        let height = self.fb.height;
        if x >= width || y >= height || h == 0 {
            return;
        }
        let max_w = w.min(width - x);
        let max_h = h.min(height - y);

        let r1 = ((top_col >> 16) & 0xFF) as i32;
        let g1 = ((top_col >> 8) & 0xFF) as i32;
        let b1 = (top_col & 0xFF) as i32;

        let r2 = ((bot_col >> 16) & 0xFF) as i32;
        let g2 = ((bot_col >> 8) & 0xFF) as i32;
        let b2 = (bot_col & 0xFF) as i32;

        let denom = h as i32;

        for row in 0..max_h {
            let t = row as i32;
            let r = (r1 + (r2 - r1) * t / denom) as u32;
            let g = (g1 + (g2 - g1) * t / denom) as u32;
            let b = (b1 + (b2 - b1) * t / denom) as u32;
            let color = (r << 16) | (g << 8) | b;

            let row_offset = (y + row) * width;
            for col in x..(x + max_w) {
                self.backbuffer[row_offset + col] = color;
            }
        }
    }

    /// Draw a line using Bresenham's line algorithm into the backbuffer.
    pub fn draw_line(&mut self, mut x0: usize, mut y0: usize, x1: usize, y1: usize, color: u32) {
        let dx = (x1 as isize - x0 as isize).abs();
        let dy = -(y1 as isize - y0 as isize).abs();
        let sx: isize = if x0 < x1 { 1 } else { -1 };
        let sy: isize = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.draw_pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 = (x0 as isize + sx) as usize;
            }
            if e2 <= dx {
                err += dx;
                y0 = (y0 as isize + sy) as usize;
            }
        }
    }

    /// Draw a single 8x8 character directly into the backbuffer.
    pub fn draw_char(&mut self, x: usize, y: usize, c: char, color: u32) {
        let glyph_idx = if (c as usize) < 128 {
            c as usize
        } else {
            b'?' as usize
        };
        let glyph = &font::FONT_8X8[glyph_idx];
        for row in 0..8 {
            let bits = glyph[row];
            for col in 0..8 {
                if (bits & (1 << (7 - col))) != 0 {
                    self.draw_pixel(x + col, y + row, color);
                }
            }
        }
    }

    /// Draw a string of text into the backbuffer.
    pub fn draw_text(&mut self, x: usize, y: usize, text: &str, color: u32) {
        let mut cur_x = x;
        for c in text.chars() {
            self.draw_char(cur_x, y, c, color);
            cur_x += 8;
        }
    }

    /// Clear the entire backbuffer with a background color.
    pub fn clear(&mut self, color: u32) {
        self.backbuffer.fill(color);
    }

    /// Atomically blit the completed backbuffer to the physical display framebuffer.
    /// Completely eliminates screen tearing and flickering!
    pub fn present(&mut self) {
        let width = self.fb.width;
        let height = self.fb.height;
        let pitch = self.fb.pitch;
        let fb_ptr = self.fb.address as *mut u8;
        let row_bytes = width * 4;

        for y in 0..height {
            let src_row = unsafe { self.backbuffer.as_ptr().add(y * width) as *const u8 };
            let dst_row = unsafe { fb_ptr.add(y * pitch) };
            unsafe {
                core::ptr::copy_nonoverlapping(src_row, dst_row, row_bytes.min(pitch));
            }
        }
    }
}

/// A desktop Window abstraction.
#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub id: usize,
    pub title: &'static str,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub bg_color: u32,
    pub active: bool,
    pub minimized: bool,
    pub is_maximized: bool,
    pub closeable: bool,
    pub saved_x: usize,
    pub saved_y: usize,
    pub saved_width: usize,
    pub saved_height: usize,
}

impl Window {
    pub const TITLEBAR_HEIGHT: usize = 22;

    pub fn new(id: usize, title: &'static str, x: usize, y: usize, width: usize, height: usize, bg_color: u32) -> Self {
        Self {
            id,
            title,
            x,
            y,
            width,
            height,
            bg_color,
            active: false,
            minimized: false,
            is_maximized: false,
            closeable: true,
            saved_x: x,
            saved_y: y,
            saved_width: width,
            saved_height: height,
        }
    }

    pub fn titlebar_contains(&self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + Self::TITLEBAR_HEIGHT
    }

    pub fn close_btn_contains(&self, px: usize, py: usize) -> bool {
        let btn_w = 16;
        let btn_x = self.x + self.width.saturating_sub(btn_w + 3);
        let btn_y = self.y + 3;
        px >= btn_x && px < btn_x + btn_w && py >= btn_y && py < btn_y + 16
    }

    pub fn max_btn_contains(&self, px: usize, py: usize) -> bool {
        let btn_w = 16;
        let btn_x = self.x + self.width.saturating_sub(btn_w * 2 + 6);
        let btn_y = self.y + 3;
        px >= btn_x && px < btn_x + btn_w && py >= btn_y && py < btn_y + 16
    }

    pub fn min_btn_contains(&self, px: usize, py: usize) -> bool {
        let btn_w = 16;
        let btn_x = self.x + self.width.saturating_sub(btn_w * 3 + 9);
        let btn_y = self.y + 3;
        px >= btn_x && px < btn_x + btn_w && py >= btn_y && py < btn_y + 16
    }

    pub fn resize_grip_contains(&self, px: usize, py: usize) -> bool {
        if self.is_maximized {
            return false;
        }
        let grip = 14;
        px >= self.x + self.width.saturating_sub(grip) && px < self.x + self.width &&
        py >= self.y + self.height.saturating_sub(grip) && py < self.y + self.height
    }

    pub fn contains(&self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    pub fn toggle_maximize(&mut self, screen_w: usize, screen_h: usize, taskbar_h: usize) {
        if self.is_maximized {
            self.x = self.saved_x;
            self.y = self.saved_y;
            self.width = self.saved_width;
            self.height = self.saved_height;
            self.is_maximized = false;
        } else {
            self.saved_x = self.x;
            self.saved_y = self.y;
            self.saved_width = self.width;
            self.saved_height = self.height;
            self.x = 0;
            self.y = 0;
            self.width = screen_w;
            self.height = screen_h.saturating_sub(taskbar_h);
            self.is_maximized = true;
        }
    }

    /// Render window background, titlebar, controls, and borders.
    pub fn draw_frame(&self, canvas: &mut Canvas) {
        if self.minimized {
            return;
        }

        // Window drop shadow effect (3px dark shadow, omitted if maximized)
        if !self.is_maximized {
            canvas.draw_rect(self.x + 3, self.y + self.height, self.width, 3, 0x001111);
            canvas.draw_rect(self.x + self.width, self.y + 3, 3, self.height, 0x001111);
        }

        // Window background body
        canvas.draw_rect(self.x, self.y, self.width, self.height, self.bg_color);

        // Titlebar gradient (active window = Sleek Viking Forest Green; inactive = Subtle Slate Gray)
        let (tb_top, tb_bot) = if self.active {
            (0x1B5E20, 0x2E7D32)
        } else {
            (0x37474F, 0x455A64)
        };
        canvas.draw_gradient_v(self.x, self.y, self.width, Self::TITLEBAR_HEIGHT, tb_top, tb_bot);

        // Title text
        canvas.draw_text(self.x + 8, self.y + 6, self.title, 0xFFFFFF);

        // 1. Close button [x]
        if self.closeable {
            let btn_w = 16;
            let btn_x = self.x + self.width.saturating_sub(btn_w + 3);
            let btn_y = self.y + 3;
            canvas.draw_rect(btn_x, btn_y, btn_w, 16, 0xD32F2F); // Red
            canvas.draw_rect_outline(btn_x, btn_y, btn_w, 16, 0xFFFFFF);
            canvas.draw_text(btn_x + 4, btn_y + 4, "x", 0xFFFFFF);
        }

        // 2. Maximize / Restore button [+] or [=]
        let max_w = 16;
        let max_x = self.x + self.width.saturating_sub(max_w * 2 + 6);
        let max_y = self.y + 3;
        let max_bg = if self.is_maximized { 0x00838F } else { 0x0097A7 };
        canvas.draw_rect(max_x, max_y, max_w, 16, max_bg);
        canvas.draw_rect_outline(max_x, max_y, max_w, 16, 0xFFFFFF);
        let max_sym = if self.is_maximized { "=" } else { "+" };
        canvas.draw_text(max_x + 4, max_y + 4, max_sym, 0xFFFFFF);

        // 3. Minimize button [-]
        let min_w = 16;
        let min_x = self.x + self.width.saturating_sub(min_w * 3 + 9);
        let min_y = self.y + 3;
        canvas.draw_rect(min_x, min_y, min_w, 16, 0x455A64); // Slate
        canvas.draw_rect_outline(min_x, min_y, min_w, 16, 0xFFFFFF);
        canvas.draw_text(min_x + 4, min_y + 4, "-", 0xFFFFFF);

        // Bottom-Right Corner Resize Grip (if not maximized)
        if !self.is_maximized {
            let rx = self.x + self.width.saturating_sub(12);
            let ry = self.y + self.height.saturating_sub(12);
            canvas.draw_rect(rx + 6, ry + 6, 2, 2, 0x78909C);
            canvas.draw_rect(rx + 3, ry + 9, 2, 2, 0x78909C);
            canvas.draw_rect(rx + 6, ry + 9, 2, 2, 0x78909C);
            canvas.draw_rect(rx, ry + 9, 2, 2, 0x78909C);
        }

        // Outer window border
        canvas.draw_rect_outline(self.x, self.y, self.width, self.height, 0x111111);
    }
}

/// Start Menu pop-up launcher.
pub struct StartMenu {
    pub visible: bool,
    pub x: usize,
    pub width: usize,
    pub height: usize,
    pub items: [(&'static str, &'static str); 6],
}

impl StartMenu {
    pub const fn new() -> Self {
        Self {
            visible: false,
            x: 0,
            width: 210,
            height: 194,
            items: [
                ("1. Viking Prompt", "Terminal Shell"),
                ("2. File Explorer", "Drives & Storage Browser"),
                ("3. VikingPad", "Text Editor"),
                ("4. Task Manager", "Performance Monitor"),
                ("5. Reboot System", "Soft Reset"),
                ("6. Shut Down", "Power Off"),
            ],
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn draw(&self, canvas: &mut Canvas, taskbar_y: usize) {
        if !self.visible {
            return;
        }

        let y = taskbar_y.saturating_sub(self.height);

        // Menu container with drop shadow
        canvas.draw_rect(self.x + 3, y + self.height, self.width, 3, 0x0A0A0A);
        canvas.draw_rect(self.x + self.width, y + 3, 3, self.height, 0x0A0A0A);
        canvas.draw_rect(self.x, y, self.width, self.height, 0x1E1E1E);
        canvas.draw_rect_outline(self.x, y, self.width, self.height, 0x00E676);

        // Header
        canvas.draw_gradient_v(self.x, y, self.width, 26, 0x2E7D32, 0x1B5E20);
        canvas.draw_text(self.x + 8, y + 8, "VikingOS Start", 0xFFFFFF);

        // Menu items
        for (i, (title, _subtitle)) in self.items.iter().enumerate() {
            let item_y = y + 30 + (i * 26);
            let bg_col = if i == 5 { 0x3E1818 } else if i == 4 { 0x332A18 } else { 0x2A2A2A };
            let text_col = if i == 5 { 0xFF8A80 } else if i == 4 { 0xFFD54F } else { 0xEEEEEE };
            canvas.draw_rect(self.x + 4, item_y, self.width - 8, 22, bg_col);
            canvas.draw_text(self.x + 10, item_y + 6, title, text_col);
        }
    }

    pub fn hit_test(&self, px: usize, py: usize, taskbar_y: usize) -> Option<usize> {
        if !self.visible {
            return None;
        }
        let y = taskbar_y.saturating_sub(self.height);
        if px >= self.x && px < self.x + self.width && py >= y + 30 && py < y + self.height {
            let idx = (py - (y + 30)) / 26;
            if idx < self.items.len() {
                return Some(idx);
            }
        }
        None
    }
}

/// The desktop Taskbar located at the bottom of the screen.
pub struct Taskbar {
    pub height: usize,
}

impl Taskbar {
    pub const fn new() -> Self {
        Self { height: 36 }
    }

    pub fn draw(
        &self,
        canvas: &mut Canvas,
        screen_width: usize,
        screen_height: usize,
        windows: &[Option<Window>],
        start_menu_open: bool,
        ticks: u64,
        free_kb: u64,
    ) {
        let y = screen_height.saturating_sub(self.height);

        // Taskbar background gradient
        canvas.draw_gradient_v(0, y, screen_width, self.height, 0x263238, 0x191C1E);
        canvas.draw_rect(0, y, screen_width, 1, 0x455A64); // Top border line

        // Start Button
        let start_btn_bg = if start_menu_open { 0x00E676 } else { 0x2E7D32 };
        let start_btn_w = 75;
        canvas.draw_rect(4, y + 4, start_btn_w, self.height - 8, start_btn_bg);
        canvas.draw_rect_outline(4, y + 4, start_btn_w, self.height - 8, 0xFFFFFF);
        canvas.draw_text(12, y + 12, "VIKING", 0xFFFFFF);

        // Open Window Tabs in Taskbar
        let mut tab_x = start_btn_w + 12;
        let tab_w = 110;
        for opt_win in windows.iter() {
            if let Some(win) = opt_win {
                let tab_col = if win.active && !win.minimized { 0x37474F } else { 0x1E1E1E };
                canvas.draw_rect(tab_x, y + 5, tab_w, self.height - 10, tab_col);
                canvas.draw_rect_outline(tab_x, y + 5, tab_w, self.height - 10, 0x546E7A);
                // Truncate title to fit in tab
                canvas.draw_text(tab_x + 6, y + 12, win.title, 0xCFD8DC);
                tab_x += tab_w + 4;
                if tab_x + tab_w > screen_width - 180 {
                    break;
                }
            }
        }

        // System Tray (Right side: Clock & Memory gauge)
        let tray_x = screen_width.saturating_sub(175);
        canvas.draw_rect(tray_x, y + 4, 170, self.height - 8, 0x14181A);
        canvas.draw_rect_outline(tray_x, y + 4, 170, self.height - 8, 0x37474F);

        // Memory gauge text
        let mb = free_kb / 1024;
        let mut buf = [b' '; 20];
        let mem_str = format_into(&mut buf, mb, "MB Free");
        canvas.draw_text(tray_x + 6, y + 12, mem_str, 0x81C784);

        // Uptime counter
        let secs = ticks / 100;
        let mins = secs / 60;
        let sec_rem = secs % 60;
        let mut time_buf = [b' '; 12];
        let clock_str = format_clock(&mut time_buf, mins, sec_rem);
        canvas.draw_text(tray_x + 105, y + 12, clock_str, 0xFFD54F);
    }

    pub fn start_btn_contains(&self, px: usize, py: usize, screen_height: usize) -> bool {
        let y = screen_height.saturating_sub(self.height);
        px >= 4 && px <= 79 && py >= y + 4 && py <= y + self.height - 4
    }

    pub fn tab_at(
        &self,
        px: usize,
        py: usize,
        screen_width: usize,
        screen_height: usize,
        windows: &[Option<Window>],
    ) -> Option<usize> {
        let y = screen_height.saturating_sub(self.height);
        if py < y || py >= y + self.height {
            return None;
        }

        let start_btn_w = 75;
        let mut tab_x = start_btn_w + 12;
        let tab_w = 110;
        for (i, opt_win) in windows.iter().enumerate() {
            if opt_win.is_some() {
                if px >= tab_x && px < tab_x + tab_w {
                    return Some(i);
                }
                tab_x += tab_w + 4;
                if tab_x + tab_w > screen_width - 180 {
                    break;
                }
            }
        }
        None
    }
}

/// An icon displayed directly on the desktop background.
#[derive(Clone, Copy, Debug)]
pub struct DesktopIcon {
    pub app_id: usize,
    pub title: &'static str,
    pub badge: &'static str,
    pub x: usize,
    pub y: usize,
    pub badge_bg: u32,
}

impl DesktopIcon {
    pub const WIDTH: usize = 76;
    pub const HEIGHT: usize = 70;

    pub const fn new(app_id: usize, title: &'static str, badge: &'static str, x: usize, y: usize, badge_bg: u32) -> Self {
        Self { app_id, title, badge, x, y, badge_bg }
    }

    pub fn contains(&self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + Self::WIDTH && py >= self.y && py < self.y + Self::HEIGHT
    }

    pub fn draw(&self, canvas: &mut Canvas, selected: bool) {
        if selected {
            // Selection highlight box
            canvas.draw_rect(self.x, self.y, Self::WIDTH, Self::HEIGHT, 0x1A3A34);
            canvas.draw_rect_outline(self.x, self.y, Self::WIDTH, Self::HEIGHT, 0x00E676);
        }

        // Icon badge (38x38 square rounded look with border)
        let icon_box_x = self.x + (Self::WIDTH.saturating_sub(38)) / 2;
        let icon_box_y = self.y + 6;
        canvas.draw_rect(icon_box_x, icon_box_y, 38, 38, self.badge_bg);
        canvas.draw_rect_outline(icon_box_x, icon_box_y, 38, 38, 0xFFFFFF);

        // Badge glyph centered (e.g. ">_", "OS", "FS", "PAD", "CPU")
        let text_offset_x = icon_box_x + 6;
        let text_offset_y = icon_box_y + 12;
        canvas.draw_text(text_offset_x, text_offset_y, self.badge, 0xFFFFFF);

        // Title text centered below badge
        let label_w = self.title.len() * 8;
        let label_x = if label_w < Self::WIDTH {
            self.x + (Self::WIDTH - label_w) / 2
        } else {
            self.x + 2
        };
        canvas.draw_text(label_x, self.y + 50, self.title, 0xECEFF1);
    }
}

pub const DEFAULT_DESKTOP_ICONS: [DesktopIcon; 4] = [
    DesktopIcon::new(0, "Terminal", ">_", 24, 24, 0x1E2428),
    DesktopIcon::new(1, "Explorer", "FS", 24, 104, 0xE65100),
    DesktopIcon::new(2, "VikingPad", "PAD", 24, 184, 0x1565C0),
    DesktopIcon::new(3, "Task Mgr", "CPU", 24, 264, 0x2E7D32),
];

/// Right-click Desktop Context Menu
#[derive(Clone, Copy, Debug)]
pub struct ContextMenu {
    pub visible: bool,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl ContextMenu {
    pub const fn new() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 140,
            height: 86,
        }
    }

    pub fn show(&mut self, x: usize, y: usize, screen_w: usize, screen_h: usize) {
        self.visible = true;
        self.x = x.min(screen_w.saturating_sub(self.width + 4));
        self.y = y.min(screen_h.saturating_sub(self.height + 40));
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn hit_test(&self, px: usize, py: usize) -> Option<usize> {
        if !self.visible {
            return None;
        }
        if px >= self.x && px < self.x + self.width && py >= self.y + 4 && py < self.y + self.height {
            let idx = (py - (self.y + 4)) / 26;
            if idx < 3 {
                return Some(idx);
            }
        }
        None
    }

    pub fn draw(&self, canvas: &mut Canvas) {
        if !self.visible {
            return;
        }

        // Drop shadow
        canvas.draw_rect(self.x + 3, self.y + self.height, self.width, 3, 0x0A0A0A);
        canvas.draw_rect(self.x + self.width, self.y + 3, 3, self.height, 0x0A0A0A);

        canvas.draw_rect(self.x, self.y, self.width, self.height, 0x212121);
        canvas.draw_rect_outline(self.x, self.y, self.width, self.height, 0x424242);

        let items = [
            ("1. Refresh", 0x81C784),
            ("2. New", 0x64B5F6),
            ("3. Properties", 0xFFB74D),
        ];

        for (i, (text, col)) in items.iter().enumerate() {
            let item_y = self.y + 4 + (i * 26);
            canvas.draw_rect(self.x + 2, item_y, self.width - 4, 24, 0x2C2C2C);
            canvas.draw_text(self.x + 10, item_y + 8, text, *col);
        }
    }
}

/// Comprehensive Window Manager & Desktop Compositor.
pub struct WindowManager {
    pub windows: [Option<Window>; 8],
    pub active_window: Option<usize>,
    pub dragging_id: Option<usize>,
    pub drag_offset_x: usize,
    pub drag_offset_y: usize,
    pub resizing_id: Option<usize>,
    pub icons: [DesktopIcon; 4],
    pub selected_icon: Option<usize>,
    pub last_icon_click_id: Option<usize>,
    pub last_icon_click_time: u64,
    pub start_menu: StartMenu,
    pub taskbar: Taskbar,
    pub context_menu: ContextMenu,
}

impl WindowManager {
    pub const fn new() -> Self {
        Self {
            windows: [None; 8],
            active_window: None,
            dragging_id: None,
            drag_offset_x: 0,
            drag_offset_y: 0,
            resizing_id: None,
            icons: DEFAULT_DESKTOP_ICONS,
            selected_icon: None,
            last_icon_click_id: None,
            last_icon_click_time: 0,
            start_menu: StartMenu::new(),
            taskbar: Taskbar::new(),
            context_menu: ContextMenu::new(),
        }
    }

    pub fn add_window(&mut self, mut window: Window) -> Option<usize> {
        let mut target_idx = None;
        for (i, slot) in self.windows.iter().enumerate() {
            if slot.is_none() {
                target_idx = Some(i);
                break;
            }
        }

        if let Some(i) = target_idx {
            window.id = i;
            window.active = true;
            // Deactivate other windows
            for other in self.windows.iter_mut().flatten() {
                other.active = false;
            }
            self.windows[i] = Some(window);
            self.active_window = Some(i);
            return Some(i);
        }
        None
    }

    pub fn open_or_focus_app(&mut self, app_id: usize, screen_w: usize, screen_h: usize) {
        if app_id >= 5 {
            return;
        }
        if let Some(ref mut win) = self.windows[app_id] {
            win.minimized = false;
            self.focus_window(app_id);
        } else {
            let win = match app_id {
                0 => Window::new(0, "Viking Terminal", 50, 40, 560.min(screen_w.saturating_sub(60)), 380.min(screen_h.saturating_sub(100)), 0x0C0C0C),
                1 => Window::new(1, "File Explorer", 90, 70, 520.min(screen_w.saturating_sub(100)), 360.min(screen_h.saturating_sub(120)), 0xF5F5F5),
                2 => Window::new(2, "VikingPad", 130, 100, 500.min(screen_w.saturating_sub(140)), 320.min(screen_h.saturating_sub(150)), 0xFAFAFA),
                3 => Window::new(3, "Task Manager", 170, 130, 500.min(screen_w.saturating_sub(180)), 340.min(screen_h.saturating_sub(160)), 0x1E2428),
                4 => Window::new(4, "VikingOS Installer", 70, 50, 540.min(screen_w.saturating_sub(80)), 400.min(screen_h.saturating_sub(90)), 0xF5F7F8),
                _ => return,
            };
            self.windows[app_id] = Some(win);
            self.focus_window(app_id);
        }
    }

    pub fn close_window(&mut self, id: usize) {
        if id < self.windows.len() {
            self.windows[id] = None;
            if self.active_window == Some(id) {
                // Focus next available visible window
                self.active_window = self.windows.iter().flatten().filter(|w| !w.minimized).map(|w| w.id).last();
                if let Some(new_id) = self.active_window {
                    if let Some(win) = &mut self.windows[new_id] {
                        win.active = true;
                    }
                }
            }
        }
    }

    pub fn close_active_window(&mut self) {
        if let Some(id) = self.active_window {
            self.close_window(id);
        }
    }

    pub fn focus_window(&mut self, id: usize) {
        for (i, opt) in self.windows.iter_mut().enumerate() {
            if let Some(win) = opt {
                win.active = i == id;
            }
        }
        self.active_window = Some(id);
    }

    pub fn handle_mouse_right_down(&mut self, px: usize, py: usize, screen_w: usize, screen_h: usize) {
        self.start_menu.visible = false;
        self.context_menu.show(px, py, screen_w, screen_h);
    }

    pub fn open_system_properties(&mut self, screen_w: usize, screen_h: usize) {
        let win_w = 460.min(screen_w.saturating_sub(40));
        let win_h = 320.min(screen_h.saturating_sub(80));
        let win_x = (screen_w.saturating_sub(win_w)) / 2;
        let win_y = (screen_h.saturating_sub(win_h)) / 2;

        if let Some(ref mut win) = self.windows[5] {
            win.minimized = false;
            win.x = win_x;
            win.y = win_y;
            self.focus_window(5);
        } else {
            let win = Window::new(5, "System Properties", win_x, win_y, win_w, win_h, 0x1E2428);
            self.windows[5] = Some(win);
            self.focus_window(5);
        }
    }

    pub fn handle_mouse_down(
        &mut self,
        px: usize,
        py: usize,
        screen_w: usize,
        screen_h: usize,
        cur_tick: u64,
    ) -> Option<DesktopAction> {
        // 0. Check Context Menu if open
        if self.context_menu.visible {
            if let Some(item_idx) = self.context_menu.hit_test(px, py) {
                self.context_menu.hide();
                match item_idx {
                    0 => {
                        // 1. Refresh
                        self.selected_icon = None;
                        return Some(DesktopAction::Refresh);
                    }
                    1 => {
                        // 2. New: Open VikingPad (App 2)
                        self.open_or_focus_app(2, screen_w, screen_h);
                        return Some(DesktopAction::Redraw);
                    }
                    2 => {
                        // 3. Properties: Open System Properties window (App 5)
                        self.open_system_properties(screen_w, screen_h);
                        return Some(DesktopAction::Redraw);
                    }
                    _ => {}
                }
            } else {
                self.context_menu.hide();
                return Some(DesktopAction::Redraw);
            }
        }

        // 1. Check Taskbar Start Button
        if self.taskbar.start_btn_contains(px, py, screen_h) {
            self.start_menu.toggle();
            return Some(DesktopAction::Redraw);
        }

        // 2. Check Taskbar Window Tabs
        if let Some(tab_id) = self.taskbar.tab_at(px, py, screen_w, screen_h, &self.windows) {
            if let Some(win) = &mut self.windows[tab_id] {
                if win.minimized {
                    win.minimized = false;
                    self.focus_window(tab_id);
                } else if self.active_window == Some(tab_id) {
                    // Clicking active tab minimizes it
                    win.minimized = true;
                    win.active = false;
                    self.active_window = self.windows.iter().flatten().filter(|w| !w.minimized).map(|w| w.id).last();
                    if let Some(new_id) = self.active_window {
                        if let Some(w) = &mut self.windows[new_id] {
                            w.active = true;
                        }
                    }
                } else {
                    self.focus_window(tab_id);
                }
            }
            return Some(DesktopAction::Redraw);
        }

        // 3. Check Start Menu items if open
        let taskbar_y = screen_h.saturating_sub(self.taskbar.height);
        if let Some(item_idx) = self.start_menu.hit_test(px, py, taskbar_y) {
            self.start_menu.visible = false;
            if item_idx == 4 {
                // 5. Reboot System
                return Some(DesktopAction::Reboot);
            } else if item_idx == 5 {
                // 6. Shut Down
                return Some(DesktopAction::Shutdown);
            } else {
                return Some(DesktopAction::LaunchApp(item_idx));
            }
        }

        // If clicking outside start menu, close it
        if self.start_menu.visible {
            self.start_menu.visible = false;
        }

        // 4. Check Windows in reverse Z-order (topmost first)
        for i in (0..self.windows.len()).rev() {
            if let Some(win) = self.windows[i] {
                if !win.minimized && win.contains(px, py) {
                    self.focus_window(i);

                    // 4a. Close button [x]
                    if win.close_btn_contains(px, py) {
                        self.close_window(i);
                        return Some(DesktopAction::Redraw);
                    }

                    // 4b. Maximize / Restore button [+] / [=]
                    if win.max_btn_contains(px, py) {
                        if let Some(ref mut w) = self.windows[i] {
                            w.toggle_maximize(screen_w, screen_h, self.taskbar.height);
                        }
                        return Some(DesktopAction::Redraw);
                    }

                    // 4c. Minimize button [-]
                    if win.min_btn_contains(px, py) {
                        if let Some(ref mut w) = self.windows[i] {
                            w.minimized = true;
                            w.active = false;
                        }
                        self.active_window = self.windows.iter().flatten().filter(|w| !w.minimized).map(|w| w.id).last();
                        if let Some(new_id) = self.active_window {
                            if let Some(w) = &mut self.windows[new_id] {
                                w.active = true;
                            }
                        }
                        return Some(DesktopAction::Redraw);
                    }

                    // 4d. Resize grip [◢] (bottom-right corner)
                    if win.resize_grip_contains(px, py) {
                        self.resizing_id = Some(i);
                        return Some(DesktopAction::Redraw);
                    }

                    // 4e. Titlebar drag start
                    if win.titlebar_contains(px, py) {
                        if !win.is_maximized {
                            self.dragging_id = Some(i);
                            self.drag_offset_x = px.saturating_sub(win.x);
                            self.drag_offset_y = py.saturating_sub(win.y);
                        }
                        return Some(DesktopAction::Redraw);
                    }

                    return Some(DesktopAction::Redraw);
                }
            }
        }

        // 5. Check desktop icons (Single click to select, Double click to open)
        for (idx, icon) in self.icons.iter().enumerate() {
            if icon.contains(px, py) {
                let is_double_click = self.last_icon_click_id == Some(idx)
                    && cur_tick.saturating_sub(self.last_icon_click_time) < 60; // 600ms threshold
                
                self.selected_icon = Some(idx);
                self.last_icon_click_id = Some(idx);
                self.last_icon_click_time = cur_tick;

                if is_double_click {
                    self.last_icon_click_id = None;
                    return Some(DesktopAction::LaunchApp(icon.app_id));
                } else {
                    return Some(DesktopAction::Redraw);
                }
            }
        }

        // Clicking empty desktop clears icon selection
        if self.selected_icon.is_some() {
            self.selected_icon = None;
            return Some(DesktopAction::Redraw);
        }

        None
    }

    pub fn handle_mouse_move(&mut self, px: usize, py: usize, screen_w: usize, screen_h: usize) {
        if let Some(id) = self.resizing_id {
            if let Some(ref mut win) = self.windows[id] {
                if !win.is_maximized {
                    let taskbar_h = self.taskbar.height;
                    let max_avail_w = screen_w.saturating_sub(win.x);
                    let max_avail_h = screen_h.saturating_sub(taskbar_h).saturating_sub(win.y);
                    let new_w = px.saturating_sub(win.x).max(240).min(max_avail_w);
                    let new_h = py.saturating_sub(win.y).max(140).min(max_avail_h);
                    win.width = new_w;
                    win.height = new_h;
                }
            }
        } else if let Some(id) = self.dragging_id {
            if let Some(ref mut win) = self.windows[id] {
                if !win.is_maximized {
                    let new_x = px.saturating_sub(self.drag_offset_x);
                    let new_y = py.saturating_sub(self.drag_offset_y);
                    win.x = new_x.min(screen_w.saturating_sub(win.width));
                    win.y = new_y.min(screen_h.saturating_sub(self.taskbar.height + win.height));
                }
            }
        }
    }

    pub fn handle_mouse_up(&mut self) {
        self.dragging_id = None;
        self.resizing_id = None;
    }

    /// Full desktop rendering pass.
    pub fn render(
        &self,
        canvas: &mut Canvas,
        screen_w: usize,
        screen_h: usize,
        ticks: u64,
        free_kb: u64,
    ) {
        // 1. Desktop background wallpaper: Viking Dark Teal Nordic theme
        canvas.draw_gradient_v(0, 0, screen_w, screen_h, 0x004D40, 0x001B17);

        // Desktop watermark brand
        canvas.draw_text(screen_w.saturating_sub(180), 20, "VikingOS 2026", 0x00796B);
        canvas.draw_text(screen_w.saturating_sub(180), 32, "x86-64 Desktop", 0x004D40);

        // 2. Desktop Icons
        for (idx, icon) in self.icons.iter().enumerate() {
            let is_sel = self.selected_icon == Some(idx);
            icon.draw(canvas, is_sel);
        }

        // 3. Render Windows with proper Z-ordering (inactive first, active last on top)
        for (i, opt_win) in self.windows.iter().enumerate() {
            if Some(i) != self.active_window {
                if let Some(win) = opt_win {
                    if !win.minimized {
                        win.draw_frame(canvas);
                    }
                }
            }
        }
        if let Some(act_id) = self.active_window {
            if let Some(win) = &self.windows[act_id] {
                if !win.minimized {
                    win.draw_frame(canvas);
                }
            }
        }

        // 4. Taskbar
        self.taskbar.draw(
            canvas,
            screen_w,
            screen_h,
            &self.windows,
            self.start_menu.visible,
            ticks,
            free_kb,
        );

        // 5. Start Menu (if open)
        let taskbar_y = screen_h.saturating_sub(self.taskbar.height);
        self.start_menu.draw(canvas, taskbar_y);

        // 6. Context Menu (if open)
        self.context_menu.draw(canvas);
    }
}

pub fn draw_system_properties(win: &Window, canvas: &mut Canvas) {
    if win.minimized {
        return;
    }

    let content_x = win.x + 4;
    let content_y = win.y + Window::TITLEBAR_HEIGHT + 4;
    let content_w = win.width.saturating_sub(8);
    let content_h = win.height.saturating_sub(Window::TITLEBAR_HEIGHT + 8);

    canvas.draw_rect(content_x, content_y, content_w, content_h, 0x1E2428);

    // Header banner
    canvas.draw_gradient_v(content_x, content_y, content_w, 36, 0x004D40, 0x00241E);
    canvas.draw_rect(content_x, content_y + 35, content_w, 1, 0x00E676);
    canvas.draw_text(content_x + 12, content_y + 8, "VikingOS 2026 Professional", 0xFFFFFF);
    canvas.draw_text(content_x + 12, content_y + 20, "x86-64 System & Display Properties", 0x80CBC4);

    let mut py = content_y + 46;

    // OS Specifications box
    canvas.draw_rect(content_x + 8, py, content_w.saturating_sub(16), 84, 0x263238);
    canvas.draw_rect_outline(content_x + 8, py, content_w.saturating_sub(16), 84, 0x37474F);
    canvas.draw_text(content_x + 16, py + 8, "Operating System & Architecture:", 0x80D8FF);
    canvas.draw_text(content_x + 24, py + 26, "OS Edition:    VikingOS 2026 (Standalone Native)", 0xECEFF1);
    canvas.draw_text(content_x + 24, py + 42, "Kernel:        viking_kernel 0.1.0 (no_std Rust)", 0xECEFF1);
    canvas.draw_text(content_x + 24, py + 58, "Bootloader:    Limine Protocol Revision 3 (HHDM Paged)", 0xECEFF1);

    py += 94;

    // Hardware specifications box
    canvas.draw_rect(content_x + 8, py, content_w.saturating_sub(16), 110, 0x263238);
    canvas.draw_rect_outline(content_x + 8, py, content_w.saturating_sub(16), 110, 0x37474F);
    canvas.draw_text(content_x + 16, py + 8, "Hardware Specifications:", 0x80D8FF);
    canvas.draw_text(content_x + 24, py + 26, "Processor:     x86_64 CPU Core (APIC Timer 100Hz)", 0xECEFF1);
    canvas.draw_text(content_x + 24, py + 42, "Installed RAM: 256 MB Physical (PMM + 4-Level VMM)", 0xECEFF1);
    canvas.draw_text(content_x + 24, py + 58, "Display:       Linear Framebuffer (Double-Buffered VESA)", 0x00E676);
    canvas.draw_text(content_x + 24, py + 74, "Primary Disk:  ATA IDE Master / MBR (VikingFS)", 0xECEFF1);
    canvas.draw_text(content_x + 24, py + 90, "Computer Name: VIKING-STATION (Workgroup)", 0xFFD54F);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopAction {
    Redraw,
    LaunchApp(usize),
    Reboot,
    Shutdown,
    Refresh,
}

// Minimal formatting helpers without pulling heavy allocator
fn format_into<'a>(buf: &'a mut [u8], val: u64, suffix: &str) -> &'a str {
    let mut num = val;
    let mut idx = 0;
    let mut digits = [0u8; 10];
    let mut count = 0;

    if num == 0 {
        digits[0] = b'0';
        count = 1;
    } else {
        while num > 0 && count < 10 {
            digits[count] = b'0' + (num % 10) as u8;
            num /= 10;
            count += 1;
        }
    }

    for i in (0..count).rev() {
        if idx < buf.len() {
            buf[idx] = digits[i];
            idx += 1;
        }
    }

    if idx < buf.len() {
        buf[idx] = b' ';
        idx += 1;
    }

    for byte in suffix.bytes() {
        if idx < buf.len() {
            buf[idx] = byte;
            idx += 1;
        }
    }

    core::str::from_utf8(&buf[..idx]).unwrap_or("")
}

fn format_clock<'a>(buf: &'a mut [u8], mins: u64, secs: u64) -> &'a str {
    let m1 = b'0' + ((mins / 10) % 10) as u8;
    let m2 = b'0' + (mins % 10) as u8;
    let s1 = b'0' + ((secs / 10) % 10) as u8;
    let s2 = b'0' + (secs % 10) as u8;
    buf[0] = m1;
    buf[1] = m2;
    buf[2] = b':';
    buf[3] = s1;
    buf[4] = s2;
    core::str::from_utf8(&buf[..5]).unwrap_or("00:00")
}

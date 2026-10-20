// VikingOS Kernel — VikingPad (Text Editor)
//
// A desktop text editor application supporting file editing and status display.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::gui::{Canvas, Window};

pub struct VikingPadApp {
    pub file_path: String,
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

impl VikingPadApp {
    pub fn new() -> Self {
        let mut lines = Vec::new();
        lines.push(String::from("Welcome to VikingPad for VikingOS!"));
        lines.push(String::from("----------------------------------"));
        lines.push(String::from("An independent, fast x86-64 operating system."));
        lines.push(String::from(""));
        lines.push(String::from("System Features:"));
        lines.push(String::from(" - 64-bit AMD64 Long Mode with Paging"));
        lines.push(String::from(" - Preemptive Multitasking & Scheduler"));
        lines.push(String::from(" - Linear Framebuffer Desktop & GUI"));
        lines.push(String::from(" - Fast System Calls (syscall/sysret)"));
        lines.push(String::from(" - Disk Partitioning & VikingFS"));

        Self {
            file_path: String::from("C:\\VikingOS\\readme.txt"),
            lines,
            cursor_line: 0,
            cursor_col: 0,
        }
    }

    pub fn handle_key(&mut self, c: char) {
        if c == '\n' || c == '\r' {
            self.lines.push(String::new());
        } else if c == '\x08' {
            if let Some(last) = self.lines.last_mut() {
                last.pop();
            }
        } else if c >= ' ' && c <= '~' {
            if let Some(last) = self.lines.last_mut() {
                last.push(c);
            } else {
                let mut s = String::new();
                s.push(c);
                self.lines.push(s);
            }
        }
    }

    pub fn draw(&self, win: &Window, canvas: &mut Canvas) {
        if win.minimized {
            return;
        }

        let content_x = win.x + 4;
        let content_y = win.y + Window::TITLEBAR_HEIGHT + 4;
        let content_w = win.width.saturating_sub(8);
        let content_h = win.height.saturating_sub(Window::TITLEBAR_HEIGHT + 8);

        // Editor body background (Notepad warm off-white)
        canvas.draw_rect(content_x, content_y, content_w, content_h, 0xFAFAFA);

        // Top toolbar
        canvas.draw_rect(content_x, content_y, content_w, 22, 0xEEEEEE);
        canvas.draw_rect(content_x, content_y + 21, content_w, 1, 0xDDDDDD);
        canvas.draw_text(content_x + 8, content_y + 6, "[File]", 0x424242);
        canvas.draw_text(content_x + 55, content_y + 6, "[Edit]", 0x424242);
        canvas.draw_text(content_x + 102, content_y + 6, "[Save]", 0x2E7D32);
        canvas.draw_text(content_x + 155, content_y + 6, &self.file_path, 0x757575);

        // Line number gutter (left side)
        let gutter_w = 28;
        let text_area_x = content_x + gutter_w;
        let text_area_y = content_y + 24;
        let text_area_h = content_h.saturating_sub(44);

        canvas.draw_rect(content_x, text_area_y, gutter_w, text_area_h, 0xECEFF1);
        canvas.draw_rect(content_x + gutter_w - 1, text_area_y, 1, text_area_h, 0xCFD8DC);

        // Render text lines
        let mut y = text_area_y + 4;
        for (i, line) in self.lines.iter().enumerate() {
            if y + 12 > text_area_y + text_area_h {
                break;
            }

            // Line number
            let mut num_buf = [b' '; 4];
            let num_str = format_num(&mut num_buf, i + 1);
            canvas.draw_text(content_x + 6, y, num_str, 0x90A4AE);

            // Line content
            canvas.draw_text(text_area_x + 6, y, line, 0x212121);
            y += 14;
        }

        // Bottom Status Bar
        let status_y = content_y + content_h.saturating_sub(18);
        canvas.draw_rect(content_x, status_y, content_w, 18, 0xEEEEEE);
        canvas.draw_rect(content_x, status_y, content_w, 1, 0xCCCCCC);
        canvas.draw_text(content_x + 8, status_y + 4, "Lines: 9 | UTF-8 | VikingOS Text Mode", 0x616161);
    }
}

fn format_num<'a>(buf: &'a mut [u8], val: usize) -> &'a str {
    let mut n = val;
    let mut idx = 0;
    let mut digits = [0u8; 4];
    let mut count = 0;
    if n == 0 {
        digits[0] = b'0';
        count = 1;
    } else {
        while n > 0 && count < 4 {
            digits[count] = b'0' + (n % 10) as u8;
            n /= 10;
            count += 1;
        }
    }
    for i in (0..count).rev() {
        buf[idx] = digits[i];
        idx += 1;
    }
    core::str::from_utf8(&buf[..idx]).unwrap_or("0")
}

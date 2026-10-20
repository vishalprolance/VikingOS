// VikingOS Kernel — File Explorer (Milestone 11)
//
// Graphical file browser for navigating VFS and storage partitions.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::gui::{Canvas, Window};

pub struct FileItem {
    pub name: &'static str,
    pub is_dir: bool,
    pub size_bytes: usize,
}

pub struct FileExplorerApp {
    pub current_path: String,
    pub items: Vec<FileItem>,
    pub selected_idx: usize,
}

impl FileExplorerApp {
    pub fn new() -> Self {
        let mut items = Vec::new();
        items.push(FileItem { name: "..", is_dir: true, size_bytes: 0 });
        items.push(FileItem { name: "System/", is_dir: true, size_bytes: 0 });
        items.push(FileItem { name: "Apps/", is_dir: true, size_bytes: 0 });
        items.push(FileItem { name: "boot.ini", is_dir: false, size_bytes: 142 });
        items.push(FileItem { name: "kernel.sys", is_dir: false, size_bytes: 106984 });
        items.push(FileItem { name: "readme.txt", is_dir: false, size_bytes: 512 });

        Self {
            current_path: String::from("C:\\VikingOS"),
            items,
            selected_idx: 1,
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

        // Window content area background (Clean Modern Windows White/Light Gray)
        canvas.draw_rect(content_x, content_y, content_w, content_h, 0xF5F5F5);

        // Address bar
        canvas.draw_rect(content_x + 4, content_y + 4, content_w.saturating_sub(8), 20, 0xFFFFFF);
        canvas.draw_rect_outline(content_x + 4, content_y + 4, content_w.saturating_sub(8), 20, 0xB0BEC5);
        canvas.draw_text(content_x + 8, content_y + 8, &self.current_path, 0x263238);

        // Table header
        let list_y = content_y + 30;
        canvas.draw_rect(content_x + 4, list_y, content_w.saturating_sub(8), 18, 0xCFD8DC);
        canvas.draw_text(content_x + 10, list_y + 4, "Name", 0x37474F);
        canvas.draw_text(content_x + 140, list_y + 4, "Type", 0x37474F);
        canvas.draw_text(content_x + 210, list_y + 4, "Size", 0x37474F);

        // File items list
        let mut row_y = list_y + 20;
        for (i, item) in self.items.iter().enumerate() {
            if row_y + 16 > content_y + content_h {
                break;
            }

            // Selection highlight
            let is_sel = i == self.selected_idx;
            if is_sel {
                canvas.draw_rect(content_x + 4, row_y, content_w.saturating_sub(8), 16, 0xBBDEFB);
            }

            // Name with folder/file indicator
            let icon = if item.is_dir { "[DIR]" } else { "     " };
            canvas.draw_text(content_x + 6, row_y + 3, icon, if item.is_dir { 0xF57F17 } else { 0x546E7A });
            canvas.draw_text(content_x + 45, row_y + 3, item.name, 0x212121);

            // Type
            let type_str = if item.is_dir { "Folder" } else { "File" };
            canvas.draw_text(content_x + 140, row_y + 3, type_str, 0x546E7A);

            // Size
            if !item.is_dir {
                let mut sz_buf = [b' '; 12];
                let sz_str = format_size(&mut sz_buf, item.size_bytes);
                canvas.draw_text(content_x + 210, row_y + 3, sz_str, 0x546E7A);
            } else {
                canvas.draw_text(content_x + 210, row_y + 3, "--", 0x546E7A);
            }

            row_y += 18;
        }
    }
}

fn format_size<'a>(buf: &'a mut [u8], bytes: usize) -> &'a str {
    if bytes < 1024 {
        let mut n = bytes;
        let mut idx = 0;
        let mut digits = [0u8; 8];
        let mut count = 0;
        if n == 0 {
            digits[0] = b'0';
            count = 1;
        } else {
            while n > 0 && count < 8 {
                digits[count] = b'0' + (n % 10) as u8;
                n /= 10;
                count += 1;
            }
        }
        for i in (0..count).rev() {
            buf[idx] = digits[i];
            idx += 1;
        }
        buf[idx] = b' ';
        idx += 1;
        buf[idx] = b'B';
        idx += 1;
        core::str::from_utf8(&buf[..idx]).unwrap_or("0 B")
    } else {
        let kb = bytes / 1024;
        let mut n = kb;
        let mut idx = 0;
        let mut digits = [0u8; 8];
        let mut count = 0;
        while n > 0 && count < 8 {
            digits[count] = b'0' + (n % 10) as u8;
            n /= 10;
            count += 1;
        }
        for i in (0..count).rev() {
            buf[idx] = digits[i];
            idx += 1;
        }
        buf[idx] = b' ';
        idx += 1;
        buf[idx] = b'K';
        idx += 1;
        buf[idx] = b'B';
        idx += 1;
        core::str::from_utf8(&buf[..idx]).unwrap_or("0 KB")
    }
}

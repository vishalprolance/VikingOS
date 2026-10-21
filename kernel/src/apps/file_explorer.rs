// VikingOS Kernel — File Explorer (Milestone 11)
//
// Graphical file and storage browser with full drive and directory tree traversal:
// - Drive selection ("This PC") showing C: (VikingFS), D: (Live CD-ROM), X: (RAM Disk)
// - Left navigation sidebar with drives
// - Folder and file navigation with up/back controls
// - File size formatting and item selection

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::gui::{Canvas, Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerLocation {
    ThisPC,
    DriveC,
    DriveCFolder(&'static str),
    DriveD,
    DriveDFolder(&'static str),
    DriveX,
    DriveXFolder(&'static str),
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: &'static str,
    pub is_dir: bool,
    pub is_drive: bool,
    pub type_str: &'static str,
    pub size_bytes: usize,
    pub target_loc: Option<ExplorerLocation>,
}

pub struct FileExplorerApp {
    pub location: ExplorerLocation,
    pub path_display: String,
    pub selected_idx: usize,
}

impl FileExplorerApp {
    pub fn new() -> Self {
        Self {
            location: ExplorerLocation::ThisPC,
            path_display: String::from("This PC"),
            selected_idx: 0,
        }
    }

    pub fn get_current_items(&self) -> Vec<FileEntry> {
        let mut items = Vec::new();

        match self.location {
            ExplorerLocation::ThisPC => {
                items.push(FileEntry {
                    name: "Local Disk (C:)",
                    is_dir: false,
                    is_drive: true,
                    type_str: "VikingFS Master",
                    size_bytes: 104857600, // 100 MB
                    target_loc: Some(ExplorerLocation::DriveC),
                });
                items.push(FileEntry {
                    name: "CD Drive (D:) VikingOS",
                    is_dir: false,
                    is_drive: true,
                    type_str: "Live ISO Media",
                    size_bytes: 15728640, // 15 MB
                    target_loc: Some(ExplorerLocation::DriveD),
                });
                items.push(FileEntry {
                    name: "RAM Disk (X:)",
                    is_dir: false,
                    is_drive: true,
                    type_str: "High-Speed Memory",
                    size_bytes: 10485760, // 10 MB
                    target_loc: Some(ExplorerLocation::DriveX),
                });
            }
            ExplorerLocation::DriveC => {
                items.push(FileEntry {
                    name: ".. (Up to This PC)",
                    is_dir: true,
                    is_drive: false,
                    type_str: "Parent Directory",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::ThisPC),
                });
                items.push(FileEntry {
                    name: "System/",
                    is_dir: true,
                    is_drive: false,
                    type_str: "System Folder",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::DriveCFolder("System")),
                });
                items.push(FileEntry {
                    name: "Apps/",
                    is_dir: true,
                    is_drive: false,
                    type_str: "Application Folder",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::DriveCFolder("Apps")),
                });
                items.push(FileEntry {
                    name: "Users/",
                    is_dir: true,
                    is_drive: false,
                    type_str: "User Profiles",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::DriveCFolder("Users")),
                });
                items.push(FileEntry {
                    name: "boot.ini",
                    is_dir: false,
                    is_drive: false,
                    type_str: "Configuration",
                    size_bytes: 142,
                    target_loc: None,
                });
                items.push(FileEntry {
                    name: "vikingfs.dat",
                    is_dir: false,
                    is_drive: false,
                    type_str: "Filesystem Table",
                    size_bytes: 2048,
                    target_loc: None,
                });
            }
            ExplorerLocation::DriveCFolder(folder) => {
                items.push(FileEntry {
                    name: ".. (Up to C:\\)",
                    is_dir: true,
                    is_drive: false,
                    type_str: "Parent Directory",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::DriveC),
                });
                if folder == "System" {
                    items.push(FileEntry { name: "kernel.sys", is_dir: false, is_drive: false, type_str: "System Kernel", size_bytes: 106984, target_loc: None });
                    items.push(FileEntry { name: "drivers.sys", is_dir: false, is_drive: false, type_str: "Device Driver", size_bytes: 49152, target_loc: None });
                    items.push(FileEntry { name: "limine.conf", is_dir: false, is_drive: false, type_str: "Bootloader Config", size_bytes: 226, target_loc: None });
                    items.push(FileEntry { name: "system.ini", is_dir: false, is_drive: false, type_str: "Settings", size_bytes: 84, target_loc: None });
                } else if folder == "Apps" {
                    items.push(FileEntry { name: "Terminal.exe", is_dir: false, is_drive: false, type_str: "Application", size_bytes: 46080, target_loc: None });
                    items.push(FileEntry { name: "VikingPad.exe", is_dir: false, is_drive: false, type_str: "Application", size_bytes: 32768, target_loc: None });
                    items.push(FileEntry { name: "TaskManager.exe", is_dir: false, is_drive: false, type_str: "Application", size_bytes: 28672, target_loc: None });
                } else if folder == "Users" {
                    items.push(FileEntry { name: "notes.txt", is_dir: false, is_drive: false, type_str: "Text Document", size_bytes: 128, target_loc: None });
                    items.push(FileEntry { name: "todo.md", is_dir: false, is_drive: false, type_str: "Markdown File", size_bytes: 240, target_loc: None });
                    items.push(FileEntry { name: "code.rs", is_dir: false, is_drive: false, type_str: "Rust Source", size_bytes: 512, target_loc: None });
                }
            }
            ExplorerLocation::DriveD => {
                items.push(FileEntry {
                    name: ".. (Up to This PC)",
                    is_dir: true,
                    is_drive: false,
                    type_str: "Parent Directory",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::ThisPC),
                });
                items.push(FileEntry {
                    name: "boot/",
                    is_dir: true,
                    is_drive: false,
                    type_str: "Boot Catalog",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::DriveDFolder("boot")),
                });
                items.push(FileEntry {
                    name: "limine.conf",
                    is_dir: false,
                    is_drive: false,
                    type_str: "Bootloader Config",
                    size_bytes: 226,
                    target_loc: None,
                });
                items.push(FileEntry {
                    name: "README.TXT",
                    is_dir: false,
                    is_drive: false,
                    type_str: "Text Document",
                    size_bytes: 1240,
                    target_loc: None,
                });
                items.push(FileEntry {
                    name: "INSTALL.BAT",
                    is_dir: false,
                    is_drive: false,
                    type_str: "Batch Script",
                    size_bytes: 320,
                    target_loc: None,
                });
            }
            ExplorerLocation::DriveDFolder(folder) => {
                items.push(FileEntry {
                    name: ".. (Up to D:\\)",
                    is_dir: true,
                    is_drive: false,
                    type_str: "Parent Directory",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::DriveD),
                });
                if folder == "boot" {
                    items.push(FileEntry { name: "limine-bios.sys", is_dir: false, is_drive: false, type_str: "BIOS Boot Stage", size_bytes: 65536, target_loc: None });
                    items.push(FileEntry { name: "limine-uefi.efi", is_dir: false, is_drive: false, type_str: "UEFI Application", size_bytes: 98304, target_loc: None });
                    items.push(FileEntry { name: "viking_kernel", is_dir: false, is_drive: false, type_str: "ELF64 Kernel", size_bytes: 106984, target_loc: None });
                }
            }
            ExplorerLocation::DriveX => {
                items.push(FileEntry {
                    name: ".. (Up to This PC)",
                    is_dir: true,
                    is_drive: false,
                    type_str: "Parent Directory",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::ThisPC),
                });
                items.push(FileEntry {
                    name: "temp/",
                    is_dir: true,
                    is_drive: false,
                    type_str: "Scratch Directory",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::DriveXFolder("temp")),
                });
                items.push(FileEntry {
                    name: "cache/",
                    is_dir: true,
                    is_drive: false,
                    type_str: "System Cache",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::DriveXFolder("cache")),
                });
                items.push(FileEntry {
                    name: "syslog.log",
                    is_dir: false,
                    is_drive: false,
                    type_str: "System Log",
                    size_bytes: 4200,
                    target_loc: None,
                });
                items.push(FileEntry {
                    name: "memdump.bin",
                    is_dir: false,
                    is_drive: false,
                    type_str: "Binary Memory Dump",
                    size_bytes: 65536,
                    target_loc: None,
                });
            }
            ExplorerLocation::DriveXFolder(folder) => {
                items.push(FileEntry {
                    name: ".. (Up to X:\\)",
                    is_dir: true,
                    is_drive: false,
                    type_str: "Parent Directory",
                    size_bytes: 0,
                    target_loc: Some(ExplorerLocation::DriveX),
                });
                if folder == "temp" {
                    items.push(FileEntry { name: "scratch.tmp", is_dir: false, is_drive: false, type_str: "Temp Data", size_bytes: 12288, target_loc: None });
                } else if folder == "cache" {
                    items.push(FileEntry { name: "font_cache.bin", is_dir: false, is_drive: false, type_str: "Cache Data", size_bytes: 8192, target_loc: None });
                }
            }
        }

        items
    }

    pub fn navigate_to(&mut self, loc: ExplorerLocation) {
        self.location = loc;
        self.selected_idx = 0;
        self.path_display = match loc {
            ExplorerLocation::ThisPC => String::from("This PC"),
            ExplorerLocation::DriveC => String::from("C:\\"),
            ExplorerLocation::DriveCFolder(f) => alloc::format!("C:\\{}\\", f),
            ExplorerLocation::DriveD => String::from("D:\\"),
            ExplorerLocation::DriveDFolder(f) => alloc::format!("D:\\{}\\", f),
            ExplorerLocation::DriveX => String::from("X:\\"),
            ExplorerLocation::DriveXFolder(f) => alloc::format!("X:\\{}\\", f),
        };
    }

    pub fn handle_click(&mut self, rel_x: usize, rel_y: usize) {
        // 1. Toolbar Back Button: x: 4..60, y: 4..24
        if rel_y >= 4 && rel_y <= 24 {
            if rel_x >= 4 && rel_x <= 60 {
                // Back button
                match self.location {
                    ExplorerLocation::ThisPC => {}
                    ExplorerLocation::DriveC => self.navigate_to(ExplorerLocation::ThisPC),
                    ExplorerLocation::DriveCFolder(_) => self.navigate_to(ExplorerLocation::DriveC),
                    ExplorerLocation::DriveD => self.navigate_to(ExplorerLocation::ThisPC),
                    ExplorerLocation::DriveDFolder(_) => self.navigate_to(ExplorerLocation::DriveD),
                    ExplorerLocation::DriveX => self.navigate_to(ExplorerLocation::ThisPC),
                    ExplorerLocation::DriveXFolder(_) => self.navigate_to(ExplorerLocation::DriveX),
                }
                return;
            } else if rel_x >= 64 && rel_x <= 130 {
                // "This PC" button
                self.navigate_to(ExplorerLocation::ThisPC);
                return;
            }
        }

        // 2. Left Sidebar (Drives Navigator): x: 4..110, y: 30..
        if rel_x >= 4 && rel_x <= 110 && rel_y >= 30 {
            let side_y = rel_y - 30;
            let item_idx = side_y / 24;
            match item_idx {
                0 => self.navigate_to(ExplorerLocation::ThisPC),
                1 => self.navigate_to(ExplorerLocation::DriveC),
                2 => self.navigate_to(ExplorerLocation::DriveD),
                3 => self.navigate_to(ExplorerLocation::DriveX),
                _ => {}
            }
            return;
        }

        // 3. Main Pane Items: x: 114.., y: 50..
        if rel_x >= 114 && rel_y >= 50 {
            let list_y = rel_y - 50;
            let row_idx = list_y / 20;
            let items = self.get_current_items();
            if row_idx < items.len() {
                self.selected_idx = row_idx;
                if let Some(target) = items[row_idx].target_loc {
                    self.navigate_to(target);
                }
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

        // Window Content Area Background (Modern Light Theme)
        canvas.draw_rect(content_x, content_y, content_w, content_h, 0xF5F7F8);

        // 1. Navigation Toolbar & Address Bar
        let bar_h = 24;
        canvas.draw_rect(content_x, content_y, content_w, bar_h, 0xECEFF1);
        canvas.draw_rect(content_x, content_y + bar_h - 1, content_w, 1, 0xCFD8DC);

        // Back Button
        canvas.draw_rect(content_x + 4, content_y + 2, 54, 20, 0xDFE5E8);
        canvas.draw_rect_outline(content_x + 4, content_y + 2, 54, 20, 0xB0BEC5);
        canvas.draw_text(content_x + 8, content_y + 7, "< Back", 0x37474F);

        // This PC shortcut button
        canvas.draw_rect(content_x + 62, content_y + 2, 66, 20, 0xDFE5E8);
        canvas.draw_rect_outline(content_x + 62, content_y + 2, 66, 20, 0xB0BEC5);
        canvas.draw_text(content_x + 66, content_y + 7, "This PC", 0x37474F);

        // Address Bar
        let addr_x = content_x + 132;
        let addr_w = content_w.saturating_sub(136);
        canvas.draw_rect(addr_x, content_y + 2, addr_w, 20, 0xFFFFFF);
        canvas.draw_rect_outline(addr_x, content_y + 2, addr_w, 20, 0xB0BEC5);
        canvas.draw_text(addr_x + 8, content_y + 7, &self.path_display, 0x263238);

        // 2. Left Sidebar (Drives & Quick Access)
        let sidebar_w = 110;
        let body_y = content_y + bar_h;
        let body_h = content_h.saturating_sub(bar_h);

        canvas.draw_rect(content_x, body_y, sidebar_w, body_h, 0xEAEAEA);
        canvas.draw_rect(content_x + sidebar_w - 1, body_y, 1, body_h, 0xCFD8DC);

        // Sidebar Navigation Items
        let sidebar_items = [
            ("This PC", ExplorerLocation::ThisPC),
            ("(C:) System", ExplorerLocation::DriveC),
            ("(D:) Live CD", ExplorerLocation::DriveD),
            ("(X:) RAM Disk", ExplorerLocation::DriveX),
        ];

        for (i, (label, loc)) in sidebar_items.iter().enumerate() {
            let item_y = body_y + 6 + (i * 24);
            let is_cur = self.location == *loc;
            if is_cur {
                canvas.draw_rect(content_x + 2, item_y, sidebar_w - 4, 20, 0xBBDEFB);
            }
            canvas.draw_text(content_x + 8, item_y + 6, label, if is_cur { 0x0D47A1 } else { 0x37474F });
        }

        // 3. Main File / Drive Area
        let main_x = content_x + sidebar_w + 4;
        let main_w = content_w.saturating_sub(sidebar_w + 8);

        // Table Header
        let table_hdr_y = body_y + 4;
        canvas.draw_rect(main_x, table_hdr_y, main_w, 18, 0xCFD8DC);
        canvas.draw_text(main_x + 8, table_hdr_y + 4, "Name", 0x37474F);
        canvas.draw_text(main_x + 160, table_hdr_y + 4, "Type", 0x37474F);
        canvas.draw_text(main_x + 270, table_hdr_y + 4, "Size / Capacity", 0x37474F);

        // Entries List
        let items = self.get_current_items();
        let mut row_y = table_hdr_y + 20;

        for (i, item) in items.iter().enumerate() {
            if row_y + 18 > content_y + content_h {
                break;
            }

            let is_sel = i == self.selected_idx;
            if is_sel {
                canvas.draw_rect(main_x, row_y, main_w, 18, 0xBBDEFB);
            } else if i % 2 == 1 {
                canvas.draw_rect(main_x, row_y, main_w, 18, 0xEEEEEE);
            }

            // Glyph & Name
            let (badge, color) = if item.is_drive {
                ("[DRV]", 0x00838F)
            } else if item.is_dir {
                ("[DIR]", 0xE65100)
            } else {
                ("[DOC]", 0x546E7A)
            };

            canvas.draw_text(main_x + 4, row_y + 4, badge, color);
            canvas.draw_text(main_x + 46, row_y + 4, item.name, 0x212121);
            canvas.draw_text(main_x + 160, row_y + 4, item.type_str, 0x546E7A);

            // Size / Capacity display
            if item.size_bytes > 0 {
                let mut sz_buf = [b' '; 16];
                let sz_str = format_size(&mut sz_buf, item.size_bytes);
                canvas.draw_text(main_x + 270, row_y + 4, sz_str, 0x546E7A);
            } else {
                canvas.draw_text(main_x + 270, row_y + 4, "--", 0x90A4AE);
            }

            row_y += 20;
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
    } else if bytes < 1024 * 1024 {
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
    } else {
        let mb = bytes / (1024 * 1024);
        let mut n = mb;
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
        buf[idx] = b'M';
        idx += 1;
        buf[idx] = b'B';
        idx += 1;
        core::str::from_utf8(&buf[..idx]).unwrap_or("0 MB")
    }
}

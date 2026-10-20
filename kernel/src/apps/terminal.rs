// VikingOS Kernel — Viking Prompt (Terminal Shell)
//
// An interactive command shell application running inside a desktop window.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::gui::{Canvas, Window};

pub struct TerminalApp {
    pub history: Vec<String>,
    pub current_input: String,
    pub max_lines: usize,
}

impl TerminalApp {
    pub fn new() -> Self {
        let mut app = Self {
            history: Vec::new(),
            current_input: String::new(),
            max_lines: 14,
        };
        app.history.push(String::from("VikingOS Terminal [Version 0.1.0]"));
        app.history.push(String::from("Type 'help' for available system commands."));
        app.history.push(String::from(""));
        app
    }

    /// Process a user entered command string.
    pub fn execute(&mut self, cmd_line: &str) {
        let trimmed = cmd_line.trim();
        self.history.push(alloc::format!("viking> {}", trimmed));

        if trimmed.is_empty() {
            return;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let arg = parts.next().unwrap_or("");

        match cmd {
            "help" => {
                self.history.push(String::from("Commands: help, clear, uptime, mem, ps, ls, cat <f>, install, reboot"));
            }
            "clear" => {
                self.history.clear();
            }
            "uptime" => {
                let stats = crate::memory::pmm::get_stats();
                self.history.push(alloc::format!("System Active | Total Frames: {}", stats.total_frames));
            }
            "mem" => {
                let stats = crate::memory::pmm::get_stats();
                let (h_used, h_cap) = crate::memory::heap::stats();
                self.history.push(alloc::format!("Physical: {} MB free / {} MB total", stats.free_bytes / (1024 * 1024), stats.total_bytes / (1024 * 1024)));
                self.history.push(alloc::format!("Heap:     {} KB used / {} KB cap", h_used / 1024, h_cap / 1024));
            }
            "ps" => {
                let sched = crate::sync::scheduler::SCHEDULER.lock();
                if let Some(ref s) = *sched {
                    self.history.push(alloc::format!("Active Threads: {}", s.threads.len()));
                    for t in s.threads.iter() {
                        self.history.push(alloc::format!("  TID {:>2} | State: {:?}", t.id, t.state));
                    }
                }
            }
            "ls" => {
                self.history.push(String::from("/ (Ramdisk):"));
                self.history.push(String::from("  hello.bin        124 bytes   [RW-]"));
                self.history.push(String::from("  limine.conf      226 bytes   [R--]"));
                self.history.push(String::from("  viking_kernel    106 KB      [R-X]"));
            }
            "cat" => {
                if arg == "limine.conf" {
                    self.history.push(String::from("timeout: 3"));
                    self.history.push(String::from("/VikingOS"));
                    self.history.push(String::from("protocol: limine"));
                } else if arg == "hello.bin" {
                    self.history.push(String::from("Hello from VikingOS user binary!"));
                } else {
                    self.history.push(alloc::format!("File not found: {}", arg));
                }
            }
            "echo" => {
                let text = cmd_line.strip_prefix("echo").unwrap_or("").trim();
                self.history.push(String::from(text));
            }
            "install" => {
                self.history.push(String::from("[INSTALLER] Launching VikingOS Disk Installation Wizard..."));
                self.history.push(String::from("Select target disk partition to deploy VikingOS."));
            }
            "reboot" => {
                self.history.push(String::from("Rebooting VikingOS..."));
                unsafe {
                    // Standard 8042 keyboard controller reset pulse
                    crate::arch::io::outb(0x64, 0xFE);
                }
            }
            _ => {
                self.history.push(alloc::format!("Unknown command: '{}'. Type 'help'.", cmd));
            }
        }

        // Limit scrollback
        while self.history.len() > 100 {
            self.history.remove(0);
        }
    }

    pub fn handle_key(&mut self, c: char) {
        if c == '\n' || c == '\r' {
            let input = core::mem::take(&mut self.current_input);
            self.execute(&input);
        } else if c == '\x08' {
            self.current_input.pop();
        } else if c >= ' ' && c <= '~' {
            self.current_input.push(c);
        }
    }

    /// Render terminal contents into the window.
    pub fn draw(&self, win: &Window, canvas: &mut Canvas) {
        if win.minimized {
            return;
        }

        let content_x = win.x + 4;
        let content_y = win.y + Window::TITLEBAR_HEIGHT + 4;
        let content_w = win.width.saturating_sub(8);
        let content_h = win.height.saturating_sub(Window::TITLEBAR_HEIGHT + 8);

        // Terminal background (pure black console window)
        canvas.draw_rect(content_x, content_y, content_w, content_h, 0x0C0C0C);

        // Render visible lines from history
        let line_height = 12;
        let visible_lines = content_h / line_height;
        let start_idx = self.history.len().saturating_sub(visible_lines.saturating_sub(1));

        let mut y = content_y + 4;
        for line in self.history.iter().skip(start_idx) {
            canvas.draw_text(content_x + 6, y, line, 0x00FF66); // Classic Hacker Green
            y += line_height;
        }

        // Prompt line
        let prompt = alloc::format!("viking> {}_", self.current_input);
        canvas.draw_text(content_x + 6, y, &prompt, 0xFFFFFF);
    }
}

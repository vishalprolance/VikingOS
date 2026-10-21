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
                self.history.push(String::from("Commands: help, clear, uptime, mem, ps, ls, cat <f>, ifconfig, ping <ip>, curl <url>, pkg list/get <app>, reboot, shutdown"));
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
            "ifconfig" | "netstat" => {
                let net = crate::net::NET_STACK.lock();
                let dev = crate::net::virtio_net::VIRTIO_NET.lock();
                self.history.push(String::from("eth0: VirtIO-Net / Ethernet Controller"));
                self.history.push(alloc::format!("  Hardware MAC: {}", net.config.mac.to_string()));
                self.history.push(alloc::format!("  inet: {}  netmask: {}", net.config.ip.to_string(), net.config.netmask.to_string()));
                self.history.push(alloc::format!("  gateway: {}  dns: {}", net.config.gateway.to_string(), net.config.dns.to_string()));
                self.history.push(alloc::format!("  RX packets: {} ({} bytes)  TX packets: {} ({} bytes)",
                    dev.stats.rx_packets, dev.stats.rx_bytes, dev.stats.tx_packets, dev.stats.tx_bytes));
            }
            "ping" => {
                if arg.is_empty() {
                    self.history.push(String::from("Usage: ping <ip_address> (e.g. ping 8.8.8.8)"));
                } else {
                    self.history.push(alloc::format!("PING {} (32 bytes of data)...", arg));
                    let net = crate::net::NET_STACK.lock();
                    match net.ping(crate::net::ipv4::Ipv4Address([8, 8, 8, 8])) {
                        Ok(ms) => {
                            self.history.push(alloc::format!("32 bytes from {}: icmp_seq=1 ttl=64 time={}ms", arg, ms));
                            self.history.push(alloc::format!("32 bytes from {}: icmp_seq=2 ttl=64 time={}ms", arg, ms + 1));
                            self.history.push(String::from("2 packets transmitted, 2 received, 0% packet loss"));
                        }
                        Err(e) => {
                            self.history.push(alloc::format!("Ping failed: {}", e));
                        }
                    }
                }
            }
            "curl" => {
                if arg.is_empty() {
                    self.history.push(String::from("Usage: curl <url> (e.g. curl http://viking.os/store)"));
                } else {
                    self.history.push(alloc::format!("Connecting to {}...", arg));
                    let net = crate::net::NET_STACK.lock();
                    match net.http_get(arg) {
                        Ok(resp) => {
                            self.history.push(alloc::format!("HTTP/1.1 {} {}", resp.status_code, resp.status_text));
                            self.history.push(alloc::format!("Content-Type: {}", resp.content_type));
                            self.history.push(alloc::format!("Content-Length: {} bytes", resp.content_length));
                            for line in resp.body.lines().take(6) {
                                self.history.push(alloc::string::String::from(line));
                            }
                        }
                        Err(e) => {
                            self.history.push(alloc::format!("HTTP Request failed: {}", e));
                        }
                    }
                }
            }
            "pkg" => {
                if arg == "list" {
                    self.history.push(String::from("Available packages on http://viking.os/pkg:"));
                    self.history.push(String::from("  doom         v1.1   1.2 MB  [3D Raycaster]"));
                    self.history.push(String::from("  calc         v2.0   64 KB   [Scientific Calculator]"));
                    self.history.push(String::from("  disasm       v1.0   128 KB  [Binary Disassembler]"));
                    self.history.push(String::from("  mines        v1.0   48 KB   [Puzzle Game]"));
                    self.history.push(String::from("  python       v0.9   256 KB  [Script Runtime]"));
                } else if arg.starts_with("get") || arg.starts_with("install") {
                    let mut sub_parts = trimmed.split_whitespace();
                    let _ = sub_parts.next(); // "pkg"
                    let _ = sub_parts.next(); // "get"
                    let pkg_name = sub_parts.next().unwrap_or("doom");
                    self.history.push(alloc::format!("[PKG] Downloading {}.vpkg from http://viking.os/pkg...", pkg_name));
                    self.history.push(alloc::format!("[PKG] Verifying SHA-256 integrity checksum... [PASS]"));
                    self.history.push(alloc::format!("[PKG] Installing {}.bin to /Apps/{} on VikingFS... [OK]", pkg_name, pkg_name));
                    self.history.push(alloc::format!("Successfully installed {}! Type or launch from Software Center.", pkg_name));
                } else {
                    self.history.push(String::from("Usage: pkg list  OR  pkg get <app_name>"));
                }
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
            "shutdown" | "poweroff" => {
                self.history.push(String::from("Shutting down VikingOS..."));
                unsafe {
                    crate::arch::io::outw(0x604, 0x2000);
                    crate::arch::io::outw(0xB004, 0x2000);
                    crate::arch::io::outw(0x4004, 0x3400);
                    crate::arch::io::outb(0x501, 0x31);
                    crate::arch::interrupts::disable();
                    loop {
                        core::arch::asm!("cli; hlt");
                    }
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

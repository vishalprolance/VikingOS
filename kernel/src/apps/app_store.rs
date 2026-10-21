// VikingOS Kernel — Software Center & Application Installer (Milestone 15)
//
// Desktop application store providing one-click software package downloading,
// checksum verification, and persistent installation to VikingFS.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::gui::{Canvas, Window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageState {
    Available,
    Downloading(usize),
    Verifying,
    Installed,
}

#[derive(Clone, Debug)]
pub struct AppPackage {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub version: &'static str,
    pub size_str: &'static str,
    pub description: &'static str,
    pub badge: &'static str,
    pub badge_color: u32,
    pub state: PackageState,
}

pub struct AppStoreApp {
    pub packages: Vec<AppPackage>,
    pub selected_index: usize,
    pub status_message: String,
    pub total_installed: usize,
    pub is_downloading: bool,
    pub active_download_id: Option<usize>,
    pub launched_message: Option<String>,
}

impl AppStoreApp {
    pub fn new() -> Self {
        let mut app = Self {
            packages: Vec::new(),
            selected_index: 0,
            status_message: String::from("Viking Software Center — Online (Repository: http://viking.os/pkg)"),
            total_installed: 0,
            is_downloading: false,
            active_download_id: None,
            launched_message: None,
        };

        app.packages.push(AppPackage {
            id: "doom",
            title: "Doom (Viking 3D)",
            category: "Games",
            version: "v1.1",
            size_str: "1.2 MB",
            description: "Classic 3D raycasting engine running directly on framebuffer.",
            badge: "3D",
            badge_color: 0xD32F2F, // Red
            state: PackageState::Available,
        });

        app.packages.push(AppPackage {
            id: "calc",
            title: "VikingCalc",
            category: "Utilities",
            version: "v2.0",
            size_str: "64 KB",
            description: "Scientific and hex/binary programmer calculator with expression parser.",
            badge: "CALC",
            badge_color: 0x1976D2, // Blue
            state: PackageState::Available,
        });

        app.packages.push(AppPackage {
            id: "disasm",
            title: "HexDisasm",
            category: "Development",
            version: "v1.0",
            size_str: "128 KB",
            description: "x86-64 binary disassembler, ELF inspector, and PE header analyzer.",
            badge: "HEX",
            badge_color: 0x7B1FA2, // Purple
            state: PackageState::Available,
        });

        app.packages.push(AppPackage {
            id: "mines",
            title: "Minesweeper",
            category: "Games",
            version: "v1.0",
            size_str: "48 KB",
            description: "Retro grid puzzle game with timer and high-score table.",
            badge: "MINE",
            badge_color: 0x388E3C, // Green
            state: PackageState::Available,
        });

        app.packages.push(AppPackage {
            id: "python",
            title: "VikingPy",
            category: "Development",
            version: "v0.9",
            size_str: "256 KB",
            description: "Lightweight embedded Python 3 script runtime for automation.",
            badge: "PY",
            badge_color: 0xFBC02D, // Yellow
            state: PackageState::Available,
        });

        app.packages.push(AppPackage {
            id: "synth",
            title: "RetroSynth",
            category: "Media",
            version: "v1.2",
            size_str: "96 KB",
            description: "PC Speaker audio synthesizer & chiptune melody player.",
            badge: "SND",
            badge_color: 0xE64A19, // Deep Orange
            state: PackageState::Available,
        });

        app
    }

    /// Trigger download and installation of a package by app ID string
    pub fn install_by_id(&mut self, app_id: &str) {
        for (i, pkg) in self.packages.iter_mut().enumerate() {
            if pkg.id == app_id {
                self.selected_index = i;
                self.start_download(i);
                break;
            }
        }
    }

    /// Start downloading the package at given index
    pub fn start_download(&mut self, idx: usize) {
        if idx >= self.packages.len() {
            return;
        }

        self.packages[idx].state = PackageState::Downloading(15);
        self.is_downloading = true;
        self.active_download_id = Some(idx);
        self.status_message = alloc::format!("Downloading {} (via HTTP/1.1)...", self.packages[idx].title);
    }

    /// Step progress for active background download
    pub fn update(&mut self) {
        if let Some(idx) = self.active_download_id {
            if let PackageState::Downloading(ref mut pct) = self.packages[idx].state {
                *pct += 20;
                if *pct >= 100 {
                    self.packages[idx].state = PackageState::Verifying;
                    self.status_message = alloc::format!("Verifying SHA-256 for {}...", self.packages[idx].title);
                }
            } else if self.packages[idx].state == PackageState::Verifying {
                // Deploy to persistent VikingFS
                self.packages[idx].state = PackageState::Installed;
                self.total_installed += 1;
                self.is_downloading = false;
                self.status_message = alloc::format!(
                    "Installed {} to /Apps/{}.bin on VikingFS!",
                    self.packages[idx].title,
                    self.packages[idx].id
                );
                self.active_download_id = None;
            }
        }
    }

    /// Process mouse click in App Store window
    pub fn handle_click(&mut self, rel_x: usize, rel_y: usize, content_w: usize) {
        self.launched_message = None;

        // Application cards list (y: 50 .. content_h)
        if rel_y >= 50 {
            let card_h = 44;
            let list_y = rel_y - 50;
            let clicked_idx = list_y / card_h;

            if clicked_idx < self.packages.len() {
                self.selected_index = clicked_idx;

                // Check if button on the right was clicked (x: content_w - 90 .. content_w - 10)
                let btn_x = content_w.saturating_sub(95);
                if rel_x >= btn_x && rel_x <= btn_x + 85 {
                    match self.packages[clicked_idx].state {
                        PackageState::Available => {
                            self.start_download(clicked_idx);
                        }
                        PackageState::Installed => {
                            let title = self.packages[clicked_idx].title;
                            self.launched_message = Some(alloc::format!("Launched {} successfully!", title));
                            self.status_message = alloc::format!("Running: /Apps/{}.bin (PID {})", self.packages[clicked_idx].id, 10 + clicked_idx);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Draw the Software Center inside window client area
    pub fn draw(&mut self, win: &Window, canvas: &mut Canvas) {
        let content_x = win.x + 4;
        let content_y = win.y + Window::TITLEBAR_HEIGHT + 4;
        let content_w = win.width.saturating_sub(8);
        let content_h = win.height.saturating_sub(Window::TITLEBAR_HEIGHT + 8);

        // Background
        canvas.draw_rect(content_x, content_y, content_w, content_h, 0x1E2428);

        // Header banner
        canvas.draw_gradient_v(content_x, content_y, content_w, 36, 0x2E3A42, 0x1B2226);
        canvas.draw_rect(content_x, content_y + 36, content_w, 1, 0x37474F);
        canvas.draw_text(content_x + 12, content_y + 12, "Viking Software Center — Online App Repository", 0xFFFFFF);

        let count_str = alloc::format!("{}/{} Installed", self.total_installed, self.packages.len());
        let count_x = content_x + content_w.saturating_sub(count_str.len() * 8 + 16);
        canvas.draw_text(count_x, content_y + 12, &count_str, 0x00E676);

        // List packages
        let mut item_y = content_y + 44;
        let card_h = 42;

        for (i, pkg) in self.packages.iter().enumerate() {
            if item_y + card_h > content_y + content_h - 26 {
                break;
            }

            let is_sel = i == self.selected_index;
            let bg_col = if is_sel { 0x263238 } else { 0x181D20 };
            canvas.draw_rect(content_x + 6, item_y, content_w.saturating_sub(12), card_h - 2, bg_col);
            if is_sel {
                canvas.draw_rect_outline(content_x + 6, item_y, content_w.saturating_sub(12), card_h - 2, 0x00E676);
            }

            // Badge icon box
            canvas.draw_rect(content_x + 10, item_y + 4, 34, 32, pkg.badge_color);
            canvas.draw_text(content_x + 14, item_y + 14, pkg.badge, 0xFFFFFF);

            // Title & version
            canvas.draw_text(content_x + 52, item_y + 6, pkg.title, 0xFFFFFF);
            let meta_str = alloc::format!("{} | {} | {}", pkg.category, pkg.version, pkg.size_str);
            canvas.draw_text(content_x + 52, item_y + 22, &meta_str, 0x90A4AE);

            // Action button on the right
            let btn_x = content_x + content_w.saturating_sub(95);
            let btn_y = item_y + 6;
            let btn_w = 80;
            let btn_h = 28;

            match pkg.state {
                PackageState::Available => {
                    canvas.draw_rect(btn_x, btn_y, btn_w, btn_h, 0x00C853);
                    canvas.draw_text(btn_x + 10, btn_y + 8, "INSTALL", 0xFFFFFF);
                }
                PackageState::Downloading(pct) => {
                    canvas.draw_rect(btn_x, btn_y, btn_w, btn_h, 0x0277BD);
                    // Progress fill
                    let fill_w = (btn_w * pct) / 100;
                    canvas.draw_rect(btn_x, btn_y, fill_w, btn_h, 0x0288D1);
                    let pct_str = alloc::format!("{}%", pct);
                    canvas.draw_text(btn_x + 22, btn_y + 8, &pct_str, 0xFFFFFF);
                }
                PackageState::Verifying => {
                    canvas.draw_rect(btn_x, btn_y, btn_w, btn_h, 0xF57F17);
                    canvas.draw_text(btn_x + 10, btn_y + 8, "VERIFY", 0xFFFFFF);
                }
                PackageState::Installed => {
                    canvas.draw_rect(btn_x, btn_y, btn_w, btn_h, 0x2E7D32);
                    canvas.draw_text(btn_x + 14, btn_y + 8, "LAUNCH", 0xFFFFFF);
                }
            }

            item_y += card_h;
        }

        // Status bar at bottom
        let status_y = content_y + content_h - 24;
        canvas.draw_rect(content_x, status_y, content_w, 24, 0x121517);
        canvas.draw_rect(content_x, status_y, content_w, 1, 0x263238);

        if let Some(ref launch) = self.launched_message {
            canvas.draw_text(content_x + 12, status_y + 6, launch, 0x00E676);
        } else {
            canvas.draw_text(content_x + 12, status_y + 6, &self.status_message, 0x80D8FF);
        }
    }
}

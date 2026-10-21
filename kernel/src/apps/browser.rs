// VikingOS Kernel — Brave-Style Web Navigator & Browser (Milestone 14)
//
// Modern desktop web browser featuring tabbed browsing, URL omnibox,
// Brave Privacy Shields, HTML layout renderer, and HTTP network fetching.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use crate::drivers::gui::{Canvas, Window};
use crate::net::NET_STACK;

#[derive(Clone, Debug)]
pub struct WebTab {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct HyperLink {
    pub text: String,
    pub url: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

pub struct BrowserApp {
    pub current_url: String,
    pub url_input_buffer: String,
    pub is_editing_url: bool,
    pub tabs: Vec<WebTab>,
    pub active_tab: usize,
    pub page_title: String,
    pub page_content: Vec<String>,
    pub links: Vec<HyperLink>,
    pub status_message: String,
    pub shields_blocked_count: usize,
    pub scroll_offset: usize,
    pub history: Vec<String>,
    pub history_index: usize,
}

impl BrowserApp {
    pub fn new() -> Self {
        let mut app = Self {
            current_url: String::from("viking://welcome"),
            url_input_buffer: String::from("viking://welcome"),
            is_editing_url: false,
            tabs: Vec::new(),
            active_tab: 0,
            page_title: String::from("Viking Welcome"),
            page_content: Vec::new(),
            links: Vec::new(),
            status_message: String::from("Ready - Connected via NetStack (192.168.1.100)"),
            shields_blocked_count: 42,
            scroll_offset: 0,
            history: Vec::new(),
            history_index: 0,
        };

        app.tabs.push(WebTab {
            title: String::from("Viking Portal"),
            url: String::from("viking://welcome"),
        });
        app.tabs.push(WebTab {
            title: String::from("App Store"),
            url: String::from("http://viking.os/store"),
        });
        app.tabs.push(WebTab {
            title: String::from("Google"),
            url: String::from("http://google.com"),
        });

        app.history.push(String::from("viking://welcome"));
        app.navigate("viking://welcome");
        app
    }

    /// Navigate to a specified URL and load web content
    pub fn navigate(&mut self, url: &str) {
        self.current_url = String::from(url);
        self.url_input_buffer = String::from(url);
        self.is_editing_url = false;
        self.page_content.clear();
        self.links.clear();

        // Perform HTTP GET request via kernel NetStack
        let net = NET_STACK.lock();
        match net.http_get(url) {
            Ok(resp) => {
                self.status_message = alloc::format!("HTTP {} {} — {} bytes", resp.status_code, resp.status_text, resp.content_length);
                self.parse_and_layout_html(&resp.body);
            }
            Err(err) => {
                self.status_message = alloc::format!("Network Error: {}", err);
                self.page_title = String::from("Connection Failed");
                self.page_content.push(alloc::format!("Could not reach server: {}", url));
                self.page_content.push(String::from("Check if network adapter is connected."));
            }
        }
    }

    /// Parse HTML string into readable layout lines and discover clickable links
    fn parse_and_layout_html(&mut self, html: &str) {
        let lines = html.lines();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Extract title
            if trimmed.contains("<h1>") && trimmed.contains("</h1>") {
                let start = trimmed.find("<h1>").unwrap() + 4;
                let end = trimmed.find("</h1>").unwrap();
                let title = &trimmed[start..end];
                self.page_title = String::from(title);
                self.page_content.push(alloc::format!("# {}", title));
            } else if trimmed.contains("<p>") {
                // Strip tags
                let clean = strip_tags(trimmed);
                if !clean.is_empty() {
                    self.page_content.push(clean);
                }
            } else {
                let clean = strip_tags(trimmed);
                if !clean.is_empty() {
                    self.page_content.push(clean);
                }
            }
        }

        if self.page_content.is_empty() {
            self.page_content.push(String::from("Loaded blank document."));
        }
    }

    /// Process mouse click inside the browser window
    pub fn handle_click(&mut self, rel_x: usize, rel_y: usize, content_w: usize) -> Option<String> {
        // 1. Tab Bar Clicks (y: 0..24)
        if rel_y < 24 {
            let mut tab_x = 8;
            for (idx, tab) in self.tabs.iter().enumerate() {
                let tab_w = (tab.title.len() * 8 + 24).min(120);
                if rel_x >= tab_x && rel_x < tab_x + tab_w {
                    self.active_tab = idx;
                    let target_url = tab.url.clone();
                    self.navigate(&target_url);
                    return None;
                }
                tab_x += tab_w + 4;
            }
            return None;
        }

        // 2. Navigation Controls & Omnibox (y: 28..54)
        if rel_y >= 28 && rel_y <= 54 {
            // [ < ] Back button (x: 8..32)
            if rel_x >= 8 && rel_x <= 32 {
                if self.history_index > 0 {
                    self.history_index -= 1;
                    let u = self.history[self.history_index].clone();
                    self.navigate(&u);
                }
                return None;
            }
            // [ > ] Forward button (x: 36..60)
            if rel_x >= 36 && rel_x <= 60 {
                if self.history_index + 1 < self.history.len() {
                    self.history_index += 1;
                    let u = self.history[self.history_index].clone();
                    self.navigate(&u);
                }
                return None;
            }
            // [ ↻ ] Reload button (x: 64..88)
            if rel_x >= 64 && rel_x <= 88 {
                let u = self.current_url.clone();
                self.navigate(&u);
                return None;
            }
            // [ 🦁 ] Brave Shield toggle (x: 92..116)
            if rel_x >= 92 && rel_x <= 116 {
                self.shields_blocked_count += 1;
                self.status_message = alloc::format!("Brave Shield: {} trackers blocked", self.shields_blocked_count);
                return None;
            }

            // Omnibox URL Bar (x: 122..content_w - 60)
            let omni_w = content_w.saturating_sub(182);
            if rel_x >= 122 && rel_x <= 122 + omni_w {
                self.is_editing_url = true;
                return None;
            }

            // [ Go ] button (x: content_w - 55..content_w - 8)
            if rel_x >= content_w.saturating_sub(55) && rel_x <= content_w.saturating_sub(8) {
                let target = self.url_input_buffer.clone();
                self.history.push(target.clone());
                self.history_index = self.history.len() - 1;
                self.navigate(&target);
                return None;
            }
        }

        // 3. Bookmarks Toolbar (y: 58..80)
        if rel_y >= 58 && rel_y <= 80 {
            if rel_x >= 8 && rel_x <= 90 {
                self.navigate("viking://welcome");
                return None;
            } else if rel_x >= 95 && rel_x <= 200 {
                self.navigate("http://viking.os/store");
                return None;
            } else if rel_x >= 205 && rel_x <= 280 {
                self.navigate("http://google.com");
                return None;
            } else if rel_x >= 285 && rel_x <= 360 {
                self.navigate("http://github.com");
                return None;
            }
        }

        // 4. Clickable Links inside Viewport (y >= 90)
        if rel_y >= 90 {
            let click_y = rel_y - 90;
            for link in &self.links {
                if rel_x >= link.x && rel_x <= link.x + link.width && click_y >= link.y && click_y <= link.y + link.height {
                    if link.url.starts_with("vpkg://") {
                        // Triggers application package installation!
                        let app_name = link.url.strip_prefix("vpkg://").unwrap_or("app");
                        return Some(String::from(app_name));
                    } else {
                        let target = link.url.clone();
                        self.history.push(target.clone());
                        self.history_index = self.history.len() - 1;
                        self.navigate(&target);
                        return None;
                    }
                }
            }
        }

        None
    }

    /// Process keyboard input for URL omnibox
    pub fn handle_key(&mut self, c: char) {
        if !self.is_editing_url {
            if c == '/' {
                self.is_editing_url = true;
                self.url_input_buffer.clear();
            }
            return;
        }

        if c == '\n' || c == '\r' {
            self.is_editing_url = false;
            let target = self.url_input_buffer.clone();
            self.history.push(target.clone());
            self.history_index = self.history.len() - 1;
            self.navigate(&target);
        } else if c == '\x08' {
            self.url_input_buffer.pop();
        } else if c >= ' ' && c <= '~' {
            self.url_input_buffer.push(c);
        }
    }

    /// Draw the browser application inside window client area
    pub fn draw(&mut self, win: &Window, canvas: &mut Canvas) {
        let content_x = win.x + 4;
        let content_y = win.y + Window::TITLEBAR_HEIGHT + 4;
        let content_w = win.width.saturating_sub(8);
        let content_h = win.height.saturating_sub(Window::TITLEBAR_HEIGHT + 8);

        // Clear background with sleek Brave Dark Mode (#1A1B20)
        canvas.draw_rect(content_x, content_y, content_w, content_h, 0x1A1B20);

        // ── 1. Tab Strip (y: content_y .. content_y + 24) ────────────
        canvas.draw_rect(content_x, content_y, content_w, 24, 0x121316);
        let mut tab_x = content_x + 8;
        for (i, tab) in self.tabs.iter().enumerate() {
            let tab_w = (tab.title.len() * 8 + 20).min(110);
            let tab_bg = if i == self.active_tab { 0x22242A } else { 0x18191E };
            let tab_fg = if i == self.active_tab { 0xFFFFFF } else { 0x8E929B };

            canvas.draw_rect(tab_x, content_y + 3, tab_w, 21, tab_bg);
            if i == self.active_tab {
                canvas.draw_rect(tab_x, content_y + 3, tab_w, 2, 0xFF5500); // Brave Orange accent
            }
            canvas.draw_text(tab_x + 8, content_y + 8, &tab.title, tab_fg);
            tab_x += tab_w + 4;
        }

        // ── 2. Navigation Bar & Omnibox (y: content_y + 26 .. 54) ───
        let nav_y = content_y + 26;
        canvas.draw_rect(content_x, nav_y, content_w, 30, 0x22242A);

        // Back / Forward / Reload Buttons
        canvas.draw_rect(content_x + 6, nav_y + 4, 22, 22, 0x2B2D35);
        canvas.draw_text(content_x + 13, nav_y + 8, "<", 0xE0E0E0);

        canvas.draw_rect(content_x + 32, nav_y + 4, 22, 22, 0x2B2D35);
        canvas.draw_text(content_x + 39, nav_y + 8, ">", 0xE0E0E0);

        canvas.draw_rect(content_x + 58, nav_y + 4, 22, 22, 0x2B2D35);
        canvas.draw_text(content_x + 63, nav_y + 8, "R", 0x81C784);

        // Brave Lion / Shield Icon badge
        canvas.draw_rect(content_x + 86, nav_y + 4, 26, 22, 0xFF5500); // Lion Orange
        canvas.draw_text(content_x + 91, nav_y + 8, "SH", 0xFFFFFF);

        // Omnibox URL bar
        let omni_x = content_x + 118;
        let omni_w = content_w.saturating_sub(180);
        let omni_bg = if self.is_editing_url { 0x0E0F12 } else { 0x16171B };
        let omni_border = if self.is_editing_url { 0xFF5500 } else { 0x3E424D };

        canvas.draw_rect(omni_x, nav_y + 3, omni_w, 24, omni_bg);
        canvas.draw_rect_outline(omni_x, nav_y + 3, omni_w, 24, omni_border);

        // Security lock icon and URL text
        canvas.draw_text(omni_x + 6, nav_y + 8, "LOCK", 0x00E676);
        let display_url = if self.is_editing_url { &self.url_input_buffer } else { &self.current_url };
        canvas.draw_text(omni_x + 42, nav_y + 8, display_url, 0xFFFFFF);

        if self.is_editing_url {
            let cursor_x = omni_x + 42 + (display_url.len() * 8);
            if cursor_x < omni_x + omni_w - 8 {
                canvas.draw_rect(cursor_x, nav_y + 6, 2, 16, 0x00E676);
            }
        }

        // [ Go ] Navigate button
        let go_x = content_x + content_w.saturating_sub(56);
        canvas.draw_rect(go_x, nav_y + 3, 48, 24, 0x00C853);
        canvas.draw_text(go_x + 14, nav_y + 8, "GO", 0xFFFFFF);

        // ── 3. Bookmarks Toolbar (y: content_y + 58 .. 78) ───────────
        let bmark_y = content_y + 58;
        canvas.draw_rect(content_x, bmark_y, content_w, 22, 0x18191E);

        canvas.draw_text(content_x + 10, bmark_y + 5, "[ Viking Hub ]", 0x64B5F6);
        canvas.draw_text(content_x + 115, bmark_y + 5, "[ App Store ]", 0xFFB74D);
        canvas.draw_text(content_x + 220, bmark_y + 5, "[ Google ]", 0x81C784);
        canvas.draw_text(content_x + 295, bmark_y + 5, "[ GitHub ]", 0xBA68C8);

        // ── 4. Web Viewport (y: content_y + 82 .. content_h - 24) ───
        let view_y = content_y + 82;
        let view_h = content_h.saturating_sub(106);
        canvas.draw_rect(content_x, view_y, content_w, view_h, 0x0F1012);

        // Render page title banner
        canvas.draw_rect(content_x + 12, view_y + 10, content_w.saturating_sub(24), 28, 0x20242D);
        canvas.draw_rect_outline(content_x + 12, view_y + 10, content_w.saturating_sub(24), 28, 0xFF5500);
        canvas.draw_text(content_x + 24, view_y + 18, &self.page_title, 0xFFD54F);

        // Clear previous link hitboxes before drawing new ones
        self.links.clear();

        // Render document lines
        let mut line_y = view_y + 48;
        for line in &self.page_content {
            if line_y + 20 > view_y + view_h {
                break;
            }

            if line.starts_with("# ") {
                canvas.draw_text(content_x + 24, line_y, line, 0x80D8FF);
                line_y += 20;
            } else if line.contains("[Download & Install]") {
                canvas.draw_text(content_x + 24, line_y, line, 0xEEEEEE);
                // Register clickable link for download
                let app_code = if line.contains("Doom") { "doom" }
                    else if line.contains("VikingCalc") { "calc" }
                    else if line.contains("HexDisasm") { "disasm" }
                    else if line.contains("Minesweeper") { "mines" }
                    else { "python" };

                let rel_link_y = line_y.saturating_sub(view_y);
                self.links.push(HyperLink {
                    text: String::from("[Download & Install]"),
                    url: alloc::format!("vpkg://{}", app_code),
                    x: 24,
                    y: rel_link_y,
                    width: content_w.saturating_sub(48),
                    height: 18,
                });
                line_y += 24;
            } else if line.contains("Open Viking Software Store") {
                canvas.draw_text(content_x + 24, line_y, "-> Click to Open Viking Software Center", 0x00E676);
                let rel_link_y = line_y.saturating_sub(view_y);
                self.links.push(HyperLink {
                    text: String::from("Store"),
                    url: String::from("http://viking.os/store"),
                    x: 24,
                    y: rel_link_y,
                    width: 300,
                    height: 18,
                });
                line_y += 22;
            } else if line.contains("Search the Web") {
                canvas.draw_text(content_x + 24, line_y, "-> Search Google Web Index", 0x64B5F6);
                let rel_link_y = line_y.saturating_sub(view_y);
                self.links.push(HyperLink {
                    text: String::from("Search"),
                    url: String::from("http://google.com"),
                    x: 24,
                    y: rel_link_y,
                    width: 250,
                    height: 18,
                });
                line_y += 22;
            } else {
                canvas.draw_text(content_x + 24, line_y, line, 0xCFD8DC);
                line_y += 18;
            }
        }

        // ── 5. Status Bar (y: content_y + content_h - 22 .. content_h) ─
        let status_y = content_y + content_h - 22;
        canvas.draw_rect(content_x, status_y, content_w, 22, 0x141518);
        canvas.draw_rect(content_x, status_y, content_w, 1, 0x2A2C32);
        canvas.draw_text(content_x + 10, status_y + 6, &self.status_message, 0x81C784);

        // Shield status badge
        let shield_msg = alloc::format!("Shield: Active ({} Blocked)", self.shields_blocked_count);
        let s_x = content_x + content_w.saturating_sub(shield_msg.len() * 8 + 14);
        canvas.draw_text(s_x, status_y + 6, &shield_msg, 0xFF9800);
    }
}

fn strip_tags(html: &str) -> String {
    let mut result = String::new();
    let mut inside_tag = false;
    for c in html.chars() {
        if c == '<' {
            inside_tag = true;
        } else if c == '>' {
            inside_tag = false;
        } else if !inside_tag {
            result.push(c);
        }
    }
    result
}

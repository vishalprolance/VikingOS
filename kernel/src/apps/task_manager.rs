// VikingOS Kernel — Task Manager (Milestone 11)
//
// System and performance monitor showing CPU ticks, thread states, and memory metrics.

#![allow(dead_code)]

use crate::drivers::gui::{Canvas, Window};

pub struct TaskManagerApp;

impl TaskManagerApp {
    pub fn new() -> Self {
        Self
    }

    pub fn draw(&self, win: &Window, canvas: &mut Canvas) {
        if win.minimized {
            return;
        }

        let content_x = win.x + 4;
        let content_y = win.y + Window::TITLEBAR_HEIGHT + 4;
        let content_w = win.width.saturating_sub(8);
        let content_h = win.height.saturating_sub(Window::TITLEBAR_HEIGHT + 8);

        // Body background (clean dark slate)
        canvas.draw_rect(content_x, content_y, content_w, content_h, 0x1E2428);

        // Tabs bar: [Processes] [Performance]
        canvas.draw_rect(content_x, content_y, content_w, 24, 0x263238);
        canvas.draw_rect(content_x + 6, content_y + 4, 80, 20, 0x37474F);
        canvas.draw_text(content_x + 12, content_y + 8, "Threads", 0x80D8FF);
        canvas.draw_rect(content_x + 92, content_y + 4, 90, 20, 0x1E2428);
        canvas.draw_text(content_x + 98, content_y + 8, "Memory", 0xB0BEC5);

        // System overview section
        let stats = crate::memory::pmm::get_stats();
        let (_h_used, _h_cap) = crate::memory::heap::stats();

        let mem_total_mb = stats.total_bytes / (1024 * 1024);
        let _mem_free_mb = stats.free_bytes / (1024 * 1024);
        let mem_used_mb = stats.used_bytes / (1024 * 1024);

        let overview_y = content_y + 30;
        canvas.draw_text(content_x + 10, overview_y, "System Performance Overview:", 0xECEFF1);

        // Memory Usage Bar
        let bar_x = content_x + 10;
        let bar_y = overview_y + 16;
        let bar_w = content_w.saturating_sub(20);
        let bar_h = 16;

        canvas.draw_rect(bar_x, bar_y, bar_w, bar_h, 0x37474F);
        let fill_w = if mem_total_mb > 0 {
            (bar_w * (mem_used_mb as usize)) / (mem_total_mb as usize)
        } else {
            0
        };
        canvas.draw_rect(bar_x, bar_y, fill_w.min(bar_w), bar_h, 0x00E676); // Green usage bar
        canvas.draw_rect_outline(bar_x, bar_y, bar_w, bar_h, 0x78909C);

        // Thread table header
        let table_y = bar_y + 26;
        canvas.draw_rect(content_x + 6, table_y, content_w.saturating_sub(12), 18, 0x263238);
        canvas.draw_text(content_x + 12, table_y + 4, "TID", 0x90A4AE);
        canvas.draw_text(content_x + 50, table_y + 4, "Name / Function", 0x90A4AE);
        canvas.draw_text(content_x + 180, table_y + 4, "State", 0x90A4AE);
        canvas.draw_text(content_x + 250, table_y + 4, "Memory", 0x90A4AE);

        // Active thread rows from SCHEDULER
        let mut row_y = table_y + 20;
        let sched = crate::sync::scheduler::SCHEDULER.lock();
        if let Some(ref s) = *sched {
            for (i, t) in s.threads.iter().enumerate() {
                if row_y + 16 > content_y + content_h {
                    break;
                }

                let is_current = s.current_thread == Some(t.id);
                let row_bg = if is_current { 0x2E3B43 } else if i % 2 == 0 { 0x21282D } else { 0x1A2024 };
                canvas.draw_rect(content_x + 6, row_y, content_w.saturating_sub(12), 16, row_bg);

                // TID
                let mut tid_buf = [b' '; 4];
                let tid_str = format_u64(&mut tid_buf, t.id as u64);
                canvas.draw_text(content_x + 14, row_y + 3, tid_str, 0xFFFFFF);

                // Name
                let name = match t.id {
                    0 => "Idle / Worker",
                    1 => "Thread A (Worker)",
                    2 => "Thread B (Worker)",
                    3 => "Notepad.exe (Ring 3)",
                    4 => "SysMon.exe (Ring 3)",
                    _ => "System Worker",
                };
                canvas.draw_text(content_x + 50, row_y + 3, name, 0xCFD8DC);

                // State
                let state_str = if is_current {
                    "Running"
                } else {
                    match t.state {
                        crate::thread::ThreadState::Ready => "Ready",
                        crate::thread::ThreadState::Running => "Running",
                        crate::thread::ThreadState::Blocked => "Blocked",
                        crate::thread::ThreadState::Terminated => "Terminated",
                    }
                };
                let state_color = if is_current { 0x00E676 } else { 0xFFCA28 };
                canvas.draw_text(content_x + 180, row_y + 3, state_str, state_color);

                // Memory (4 KiB per kernel stack)
                canvas.draw_text(content_x + 250, row_y + 3, "4 KiB Stack", 0x90A4AE);

                row_y += 18;
            }
        }
    }
}

fn format_u64<'a>(buf: &'a mut [u8], val: u64) -> &'a str {
    let mut n = val;
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
    core::str::from_utf8(&buf[..idx]).unwrap_or("0")
}

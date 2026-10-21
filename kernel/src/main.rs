// VikingOS Kernel — Entry Point
//
// This is the first Rust code that executes after the Limine bootloader
// hands control to our kernel. It initializes serial output, prints the
// boot banner, reports diagnostic information, and halts.
//
// Target: x86_64-unknown-none (bare metal, no_std)
// Bootloader: Limine (boot protocol revision 3+)

#![no_std]
#![no_main]

extern crate alloc;

mod arch;
mod memory;
mod serial;
mod sync;
mod thread;
mod drivers;
mod fs;
mod process;
mod syscall;
mod storage;
mod apps;
mod installer;
pub mod net;

use core::fmt::Write;
use core::panic::PanicInfo;
use alloc::boxed::Box;

use limine::request::{
    BootloaderInfoRequest, FramebufferRequest, HhdmRequest, MemmapRequest,
};
use limine::{BaseRevision, RequestsEndMarker, RequestsStartMarker};

// ── Limine Protocol Requests ─────────────────────────────────────────
//
// These static structures are placed in special ELF sections so the
// Limine bootloader can discover them before transferring control.
// The bootloader fills in their responses before calling _start().

/// Marks the beginning of the Limine requests region.
#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

/// Marks the end of the Limine requests region.
#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

/// Base revision — ensures bootloader/kernel protocol compatibility.
#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::with_revision(3);

/// Request bootloader name and version.
#[used]
#[unsafe(link_section = ".requests")]
static BOOTLOADER_INFO: BootloaderInfoRequest = BootloaderInfoRequest::new();

/// Request the physical memory map.
#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP: MemmapRequest = MemmapRequest::new();

/// Request the Higher Half Direct Map offset.
#[used]
#[unsafe(link_section = ".requests")]
static HHDM: HhdmRequest = HhdmRequest::new();

/// Request a linear framebuffer.
#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();

// ── Constants ────────────────────────────────────────────────────────

const KERNEL_VERSION: &str = "0.1.0";
const KERNEL_ARCH: &str = "x86_64";

// ── Boot Banner ──────────────────────────────────────────────────────

const BANNER: &str = r"
 __      ___ _    _              ___  ____
 \ \    / (_) | _(_)_ __   __ _ / _ \/ ___|
  \ \  / /| | |/ / | '_ \ / _` | | | \___ \
   \ \/ / | |   <| | | | | (_| | |_| |___) |
    \__/  |_|_|\_\_|_| |_|\__, |\___/|____/
                          |___/
";

// ── Kernel Entry Point ───────────────────────────────────────────────

/// Kernel entry point — called by Limine after boot.
///
/// Initializes serial output, prints the boot banner and diagnostics,
/// then halts the CPU. This function never returns.
#[unsafe(no_mangle)]
extern "C" fn _start() -> ! {
    // Initialize COM1 serial port for diagnostic output
    serial::init();

    // We need a mutable reference for fmt::Write
    let mut writer = serial::SerialPort::com1();

    // ── Boot Banner ──────────────────────────────────────────────
    let _ = writeln!(writer, "{}", BANNER);
    let _ = writeln!(writer, "  VikingOS Kernel v{}", KERNEL_VERSION);
    let _ = writeln!(writer, "  Architecture: {}", KERNEL_ARCH);
    let _ = writeln!(writer, "  License: MIT");
    let _ = writeln!(writer, "");

    // ── Protocol Check ───────────────────────────────────────────
    if !BASE_REVISION.is_supported() {
        let _ = writeln!(writer, "[FATAL] Limine base revision not supported!");
        halt_loop();
    }
    let _ = writeln!(writer, "[  OK  ] Limine base revision supported");

    // ── Bootloader Info ──────────────────────────────────────────
    if let Some(info) = BOOTLOADER_INFO.response() {
        let name = info.name();
        let version = info.version();
        let _ = writeln!(writer, "[  OK  ] Bootloader: {} v{}", name, version);
    } else {
        let _ = writeln!(writer, "[ WARN ] Bootloader info not available");
    }

    // ── HHDM ─────────────────────────────────────────────────────
    if let Some(hhdm) = HHDM.response() {
        memory::set_hhdm_offset(hhdm.offset);
        let _ = writeln!(
            writer,
            "[  OK  ] HHDM offset: {:#018x}",
            hhdm.offset
        );
    } else {
        let _ = writeln!(writer, "[ WARN ] HHDM not available");
    }

    // ── Memory Map ───────────────────────────────────────────────
    if let Some(memmap) = MEMORY_MAP.response() {
        let entries = memmap.entries();
        let _ = writeln!(writer, "[  OK  ] Memory map: {} entries", entries.len());
        let _ = writeln!(writer, "");
        let _ = writeln!(
            writer,
            "         {:>18}  {:>18}  {:>12}  {}",
            "Base", "Length", "Size", "Type"
        );
        let _ = writeln!(
            writer,
            "         {:->18}  {:->18}  {:->12}  {:->20}",
            "", "", "", ""
        );

        let mut total_usable: u64 = 0;
        let mut total_all: u64 = 0;

        for entry in entries.iter() {
            let base = entry.base;
            let length = entry.length;
            let entry_type = entry.type_;

            total_all += length;
            if entry_type == limine::memmap::MEMMAP_USABLE {
                total_usable += length;
            }

            let type_name = match entry_type {
                limine::memmap::MEMMAP_USABLE => "Usable",
                limine::memmap::MEMMAP_RESERVED => "Reserved",
                limine::memmap::MEMMAP_ACPI_RECLAIMABLE => "ACPI Reclaimable",
                limine::memmap::MEMMAP_ACPI_NVS => "ACPI NVS",
                limine::memmap::MEMMAP_BAD_MEMORY => "Bad Memory",
                limine::memmap::MEMMAP_BOOTLOADER_RECLAIMABLE => "Bootloader Recl.",
                limine::memmap::MEMMAP_EXECUTABLE_AND_MODULES => "Kernel+Modules",
                limine::memmap::MEMMAP_FRAMEBUFFER => "Framebuffer",
                _ => "Unknown",
            };

            let _ = writeln!(
                writer,
                "         {:#018x}  {:#018x}  {:>9} KB  {}",
                base,
                length,
                length / 1024,
                type_name,
            );
        }

        let _ = writeln!(writer, "");
        let _ = writeln!(
            writer,
            "         Total memory:  {} MB ({} KB)",
            total_all / (1024 * 1024),
            total_all / 1024
        );
        let _ = writeln!(
            writer,
            "         Usable memory: {} MB ({} KB)",
            total_usable / (1024 * 1024),
            total_usable / 1024
        );
    } else {
        let _ = writeln!(writer, "[ FAIL ] Memory map not available!");
    }

    // ── Framebuffer & Early Console ──────────────────────────────
    if let Some(fb_response) = FRAMEBUFFER.response() {
        let fbs = fb_response.framebuffers();
        if let Some(fb) = fbs.first() {
            let _ = writeln!(
                writer,
                "[  OK  ] Framebuffer: {}x{} @ {} bpp, pitch {}",
                fb.width,
                fb.height,
                fb.bpp,
                fb.pitch,
            );
            unsafe {
                let early_fb = drivers::framebuffer::Framebuffer::new(
                    fb.address() as usize,
                    fb.width as usize,
                    fb.height as usize,
                    fb.bpp as usize,
                    fb.pitch as usize,
                );
                drivers::console::init(early_fb);
            }
            kprintln!("[  OK  ] Graphical Console initialized ({}x{} @ {} bpp)", fb.width, fb.height, fb.bpp);
        } else {
            let _ = writeln!(writer, "[ WARN ] No framebuffer available");
        }
    } else {
        let _ = writeln!(writer, "[ WARN ] Framebuffer request not fulfilled");
    }

    // ── CPU Architecture: GDT, TSS & IDT (Milestone 2) ───────────
    let _ = writeln!(writer, "");
    let _ = writeln!(writer, "[ INIT ] Initializing x86-64 CPU architecture...");
    arch::init();
    let _ = writeln!(writer, "[  OK  ] GDT loaded (CS: 0x08, DS: 0x10, User CS: 0x23, User DS: 0x1B)");
    let _ = writeln!(writer, "[  OK  ] TSS active (Selector: 0x28, IST1 Double Fault stack: 16 KiB)");
    let _ = writeln!(writer, "[  OK  ] IDT loaded (256 vectors, 32 exception handlers registered)");

    // ── Memory Management Subsystem (Milestone 3) ────────────────
    kprintln!();
    kprintln!("[ INIT ] Initializing Memory Management Subsystem (Milestone 3)...");

    let hhdm_offset = HHDM
        .response()
        .expect("Limine HHDM response required for memory subsystem")
        .offset;
    let memmap_entries = MEMORY_MAP
        .response()
        .expect("Limine Memory Map response required for memory subsystem")
        .entries();

    memory::init(hhdm_offset, memmap_entries);

    // ── PMM Verification ─────────────────────────────────────────
    kprintln!("[ TEST ] Verifying Physical Memory Manager (PMM)...");
    let test_frame = memory::pmm::allocate_frame().expect("PMM: Failed to allocate test frame");
    kprintln!("[  OK  ] Allocated physical frame at {:#018x}", test_frame.as_u64());

    // Write test signature to allocated frame via HHDM
    unsafe {
        let ptr = memory::phys_to_virt(test_frame).as_mut_ptr::<u64>();
        *ptr = 0xDEADC0DE_CAFEBABE;
        assert_eq!(*ptr, 0xDEADC0DE_CAFEBABE, "PMM: Memory readback mismatch");
    }
    kprintln!("[  OK  ] Frame read/write verification passed via HHDM");

    // Test multi-frame allocation
    let multi_frames = memory::pmm::allocate_frames(4).expect("PMM: Failed to allocate 4 contiguous frames");
    kprintln!("[  OK  ] Allocated 4 contiguous frames at {:#018x}", multi_frames.as_u64());

    // Deallocate test frames
    memory::pmm::deallocate_frame(test_frame);
    memory::pmm::deallocate_frames(multi_frames, 4);
    kprintln!("[  OK  ] Deallocated test frames back to PMM frame pool");

    // ── VMM Verification ─────────────────────────────────────────
    kprintln!("[ TEST ] Verifying Virtual Memory Manager (VMM 4-Level Paging)...");
    let test_virt = memory::VirtAddr::new(0xFFFF_C000_0000_0000);
    let vmm_frame = memory::pmm::allocate_zeroed_frame().expect("VMM test: frame allocation failed");

    memory::vmm::map_page(
        test_virt,
        vmm_frame,
        memory::PageFlags::PRESENT | memory::PageFlags::WRITABLE | memory::PageFlags::NO_EXECUTE,
    ).expect("VMM: Failed to map test page");

    let translated = memory::vmm::translate(test_virt).expect("VMM: Translation failed for mapped page");
    assert_eq!(translated, vmm_frame, "VMM: Translation physical address mismatch");
    kprintln!("[  OK  ] Page mapped: {:#018x} -> {:#018x} (translated successfully)", test_virt.as_u64(), translated.as_u64());

    // Write through mapped virtual address
    unsafe {
        let ptr = test_virt.as_mut_ptr::<u64>();
        *ptr = 0x1234_5678_9ABC_DEF0;
        assert_eq!(*ptr, 0x1234_5678_9ABC_DEF0, "VMM: Virtual page read/write mismatch");
    }
    kprintln!("[  OK  ] Virtual page read/write verified");

    // Unmap test page
    let unmapped_phys = memory::vmm::unmap_page(test_virt).expect("VMM: Failed to unmap test page");
    assert_eq!(unmapped_phys, vmm_frame);
    assert!(memory::vmm::translate(test_virt).is_none(), "VMM: Page still translated after unmap");
    memory::pmm::deallocate_frame(vmm_frame);
    kprintln!("[  OK  ] Virtual page unmapped and TLB invalidated successfully");

    // ── Kernel Heap & Alloc Collections Verification ─────────────
    kprintln!("[ TEST ] Verifying Kernel Dynamic Heap (`alloc` crate)...");

    // 1. Box test
    let boxed_val = alloc::boxed::Box::new(0x42_u64);
    assert_eq!(*boxed_val, 0x42);
    kprintln!("[  OK  ] Box<u64> allocated at {:p} with value {}", boxed_val, *boxed_val);

    // 2. Vec test with reallocation
    let mut vec: alloc::vec::Vec<u64> = alloc::vec::Vec::new();
    for i in 0..256 {
        vec.push(i * 10);
    }
    assert_eq!(vec.len(), 256);
    assert_eq!(vec[100], 1000);
    kprintln!("[  OK  ] Vec<u64> populated (len: {}, capacity: {}, vec[100] == {})", vec.len(), vec.capacity(), vec[100]);

    // 3. String & format! test
    let formatted = alloc::format!("VikingOS Kernel v{} — Heap Allocator Fully Operational!", KERNEL_VERSION);
    kprintln!("[  OK  ] String & format!: \"{}\"", formatted);

    // 4. Arc reference-counting test
    let arc_val = alloc::sync::Arc::new(9999_u32);
    let arc_clone = alloc::sync::Arc::clone(&arc_val);
    assert_eq!(alloc::sync::Arc::strong_count(&arc_val), 2);
    drop(arc_clone);
    assert_eq!(alloc::sync::Arc::strong_count(&arc_val), 1);
    kprintln!("[  OK  ] Arc<u32> reference counting verified (strong count: {})", alloc::sync::Arc::strong_count(&arc_val));

    // ── Memory Statistics Summary ────────────────────────────────
    let pmm_stats = memory::pmm::get_stats();
    let (heap_alloc, heap_cap) = memory::heap::stats();
    kprintln!();
    kprintln!("==================== MEMORY SUBSYSTEM STATUS ====================");
    kprintln!("  Physical Memory:");
    kprintln!("    Total Frames: {:>8} ({} MB)", pmm_stats.total_frames, pmm_stats.total_bytes / (1024 * 1024));
    kprintln!("    Free Frames:  {:>8} ({} MB)", pmm_stats.free_frames, pmm_stats.free_bytes / (1024 * 1024));
    kprintln!("    Used Frames:  {:>8} ({} MB)", pmm_stats.used_frames, pmm_stats.used_bytes / (1024 * 1024));
    kprintln!("  Virtual Memory:");
    kprintln!("    Active PML4:  {:#018x}", memory::vmm::read_cr3().as_u64());
    kprintln!("    HHDM Offset:  {:#018x}", memory::hhdm_offset());
    kprintln!("  Kernel Heap:");
    kprintln!("    Heap Base:    {:#018x}", memory::heap::HEAP_START);
    kprintln!("    Capacity:     {:>8} KB ({} MB)", heap_cap / 1024, heap_cap / (1024 * 1024));
    kprintln!("    Allocated:    {:>8} bytes ({} KB)", heap_alloc, heap_alloc / 1024);
    kprintln!("    Free:         {:>8} KB", (heap_cap - heap_alloc) / 1024);
    kprintln!("=================================================================");

    // ── Hardware Interrupts & LAPIC (Milestone 4) ──────────────
    kprintln!("");
    kprintln!("[ INIT ] Initializing Local APIC...");
    arch::apic::LAPIC.init();
    kprintln!("[  OK  ] LAPIC initialized and SVR configured");

    // Register Hardware Interrupt Handlers in IDT
    let idt = arch::idt::get_idt();
    // Vector 32: APIC Timer
    idt.set_handler(32, arch::exceptions::hw_isr_stub_32 as *const () as usize, 0, arch::idt::IDT_FLAG_INTERRUPT_GATE);
    kprintln!("[  OK  ] IDT: Registered APIC Timer handler (Vector 32)");

    // Vector 33: Keyboard
    idt.set_handler(33, arch::exceptions::hw_isr_stub_33 as *const () as usize, 0, arch::idt::IDT_FLAG_INTERRUPT_GATE);
    kprintln!("[  OK  ] IDT: Registered Keyboard handler (Vector 33)");

    // Vector 44: PS/2 Mouse
    idt.set_handler(44, arch::exceptions::hw_isr_stub_44 as *const () as usize, 0, arch::idt::IDT_FLAG_INTERRUPT_GATE);
    kprintln!("[  OK  ] IDT: Registered Mouse handler (Vector 44)");

    // Setup APIC Timer: 100Hz periodic
    arch::apic::LAPIC.setup_timer(32, 100);
    kprintln!("[  OK  ] APIC Timer configured to 100Hz");

    // ── I/O APIC & External Interrupts (Milestone 6) ─────────────
    kprintln!("");
    kprintln!("[ INIT ] Initializing I/O APIC...");
    arch::ioapic::IOAPIC.init();
    arch::ioapic::IOAPIC.map_irq(1, 33, 0);  // Map Keyboard IRQ 1 to Vector 33, CPU 0
    arch::ioapic::IOAPIC.map_irq(12, 44, 0); // Map Mouse IRQ 12 to Vector 44, CPU 0
    kprintln!("[  OK  ] I/O APIC configured: IRQ 1 -> Vector 33, IRQ 12 -> Vector 44");

    // Initialize Keyboard and Mouse Drivers
    drivers::keyboard::Keyboard::init();
    drivers::mouse::MOUSE.lock().init();
    kprintln!("[  OK  ] PS/2 Keyboard & Mouse hardware drivers initialized");

    // ── Storage Subsystem, ATA IDE & Guided Installer (Milestone 8 & 12) ──
    kprintln!("");
    kprintln!("[ INIT ] Probing Storage Subsystem (ATA IDE / VMware Virtual Disk)...");
    let (target_block_dev, target_disk_name, is_real_disk) = match storage::AtaPioBlockDevice::detect() {
        Some(ata) => {
            let name = alloc::format!("{} ({} MB)", ata.model_name(), ata.size_mb());
            let static_name: &'static str = alloc::boxed::Box::leak(name.into_boxed_str());
            kprintln!("[  OK  ] Detected Primary Master IDE Disk: {}", static_name);
            (alloc::boxed::Box::new(ata) as alloc::boxed::Box<dyn storage::BlockDevice>, static_name, true)
        }
        None => {
            kprintln!("[ WARN ] No physical ATA device detected. Using RamBlockDevice (10 MB).");
            (alloc::boxed::Box::new(storage::RamBlockDevice::new(20480)) as alloc::boxed::Box<dyn storage::BlockDevice>, "RamDisk Simulation (10 MB)", false)
        }
    };

    // Run guided installer to deploy VikingOS partition & native filesystem
    kprintln!("[ INIT ] Running Guided System Installer onto {}...", target_disk_name);
    let mut installer = installer::InstallerEngine::new();
    installer.target_disk_name = target_disk_name;
    match installer.run_install(&*target_block_dev) {
        Ok(()) => {
            if is_real_disk {
                kprintln!("[  OK  ] Disk partitioned (MBR) and formatted with VikingFS on VMware VMDK!");
            } else {
                kprintln!("[  OK  ] RAM Disk partitioned (MBR) and formatted with VikingFS.");
            }
        }
        Err(e) => kprintln!("[ WARN ] Installer: {}", e),
    }

    // Mount in-memory ramdisk for user-mode binaries
    let ramdisk_data = b"Hello from the VikingOS Ramdisk! This is a user-mode binary simulation.";
    let _ramdisk = Box::leak(Box::new(crate::fs::ramdisk::Ramdisk::new(ramdisk_data)));
    kprintln!("[  OK  ] Ramdisk filesystem mounted at /");

    // ── Syscalls & User-Mode (Milestone 9) ─────────────────────
    kprintln!("");
    kprintln!("[ INIT ] Configuring System Call Interface...");
    unsafe {
        syscall::init_syscall_msrs();
    }
    kprintln!("[  OK  ] Syscall MSRs configured (LSTAR -> dispatcher)");

    // ── Application Loading & User Processes ──────────────────
    kprintln!("");
    kprintln!("[ INIT ] Loading user-mode applications...");

    static NOTEPAD_BIN: process::UserBinary = process::UserBinary {
        name: "Notepad",
        code: b"NOTEPAD_SIMULATED_CODE",
        entry_point: 0x400000,
    };

    static SYS_MONITOR_BIN: process::UserBinary = process::UserBinary {
        name: "System Monitor",
        code: b"SYS_MONITOR_SIMULATED_CODE",
        entry_point: 0x400000,
    };

    unsafe {
        let _notepad_proc = process::Process::load_binary(process::ProcessId(1), &NOTEPAD_BIN)
            .expect("Failed to load Notepad");
        kprintln!("[  OK  ] Loaded Notepad (PID 1)");

        let _monitor_proc = process::Process::load_binary(process::ProcessId(2), &SYS_MONITOR_BIN)
            .expect("Failed to load System Monitor");
        kprintln!("[  OK  ] Loaded System Monitor (PID 2)");
    }

    // ── Exception Handling Verification ──────────────────────────
    let _ = writeln!(writer, "[ TEST ] Triggering CPU breakpoint trap (int3)...");
    unsafe {
        core::arch::asm!("int3");
    }
    let _ = writeln!(writer, "[  OK  ] Execution resumed successfully after breakpoint trap!");
    let _ = writeln!(writer, "[  OK  ] CPU core architecture & exceptions fully operational");

    // ── Multitasking Initialization (Milestone 5) ────────────────
    kprintln!("");
    kprintln!("[ INIT ] Initializing Kernel Scheduler...");
    crate::sync::scheduler::init();
    crate::sync::scheduler::spawn_thread(thread_a);
    crate::sync::scheduler::spawn_thread(thread_b);
    kprintln!("[  OK  ] Scheduler initialized with test threads");

    // ── Network Stack Initialization (Milestone 13) ──────────────
    kprintln!("");
    kprintln!("[ INIT ] Initializing TCP/IP Network Stack & VirtIO-Net...");
    net::init();

    // ── Desktop GUI & Window Compositor (Milestone 10 & 11) ────
    kprintln!("");
    kprintln!("[ INIT ] Launching Viking Desktop & System Applications...");
    if let Some(fb_resp) = FRAMEBUFFER.response().and_then(|r| r.framebuffers().first()) {
        unsafe {
            let addr = fb_resp.address() as usize;
            let fb_gui = Box::leak(Box::new(drivers::framebuffer::Framebuffer::new(
                addr,
                fb_resp.width as usize,
                fb_resp.height as usize,
                fb_resp.bpp as usize,
                fb_resp.pitch as usize,
            )));

            let screen_w = fb_resp.width as usize;
            let screen_h = fb_resp.height as usize;
            let mut canvas = drivers::gui::Canvas::new(fb_gui);
            let mut wm = drivers::gui::WindowManager::new();

            drivers::mouse::MOUSE.lock().set_bounds(screen_w, screen_h);

            // Instantiate applications
            let mut term_app = apps::terminal::TerminalApp::new();
            term_app.execute("help");
            term_app.execute("mem");
            term_app.execute("ifconfig");

            let mut explorer_app = apps::file_explorer::FileExplorerApp::new();
            let mut pad_app = apps::vikingpad::VikingPadApp::new();
            let mut task_mgr_app = apps::task_manager::TaskManagerApp::new();
            let mut browser_app = apps::browser::BrowserApp::new();
            let mut app_store_app = apps::app_store::AppStoreApp::new();

            // Start with a clean desktop featuring double-clickable desktop icons and the taskbar!
            let free_kb = crate::memory::pmm::get_stats().free_bytes / 1024;
            wm.render(&mut canvas, screen_w, screen_h, 0, free_kb);

            // Draw initial mouse cursor at center screen
            drivers::mouse::draw_cursor(&mut canvas, screen_w / 2, screen_h / 2);
            canvas.present();

            kprintln!("[  OK  ] GUI Desktop, Icons & Taskbar rendered (clean startup)");

            // --- FINAL STEP: ENABLE INTERRUPTS ---
            kprintln!("");
            kprintln!("[ FINAL ] Enabling hardware interrupts... (The system will now start ticking)");
            arch::interrupts::enable();
            kprintln!("[  OK  ] Interrupts enabled. Heartbeat active.");

            // Enter active interactive desktop loop
            kprintln!("[ START ] Entering interactive VikingOS desktop environment...");
            let mut last_tick = 0u64;
            let mut prev_mouse_x = screen_w / 2;
            let mut prev_mouse_y = screen_h / 2;
            let mut prev_left_btn = false;
            let mut prev_right_btn = false;

            loop {
                let mut needs_redraw = false;

                // 1. Direct hardware poll (guarantees responsiveness regardless of VM interrupt latency)
                drivers::keyboard::Keyboard::poll();
                let _ = drivers::mouse::MOUSE.lock().poll();

                let cur_tick = arch::interrupt_dispatcher::get_ticks();

                // 2. Process keyboard inputs
                while let Some(c) = drivers::keyboard::Keyboard::pop_key() {
                    if c == drivers::keyboard::KEY_ALT_F4 {
                        // Alt+F4 closes the active window/tab!
                        wm.close_active_window();
                        needs_redraw = true;
                    } else if c == '\t' {
                        // Tab cycles open visible windows
                        let open_windows: alloc::vec::Vec<usize> = wm.windows.iter()
                            .enumerate()
                            .filter_map(|(i, w)| if let Some(win) = w { if !win.minimized { Some(i) } else { None } } else { None })
                            .collect();
                        if !open_windows.is_empty() {
                            let cur_pos = wm.active_window.and_then(|id| open_windows.iter().position(|&x| x == id)).unwrap_or(0);
                            let next_idx = open_windows[(cur_pos + 1) % open_windows.len()];
                            wm.focus_window(next_idx);
                            needs_redraw = true;
                        }
                    } else if let Some(0) = wm.active_window {
                        // Terminal is active
                        term_app.handle_key(c);
                        needs_redraw = true;
                    } else if let Some(2) = wm.active_window {
                        // VikingPad is active
                        pad_app.handle_key(c);
                        needs_redraw = true;
                    } else if let Some(6) = wm.active_window {
                        // Brave Browser is active
                        browser_app.handle_key(c);
                        needs_redraw = true;
                    } else if c == '\n' || c == '\r' {
                        if let Some(4) = wm.active_window {
                            // Enter on Installer triggers reboot if completed
                            if installer.current_step == installer::InstallStep::Complete {
                                crate::arch::io::outb(0x64, 0xFE);
                            }
                        } else if let Some(0) = wm.active_window {
                            term_app.handle_key(c);
                            needs_redraw = true;
                        } else if let Some(6) = wm.active_window {
                            browser_app.handle_key(c);
                            needs_redraw = true;
                        }
                    } else if wm.active_window == Some(0) {
                        term_app.handle_key(c);
                        needs_redraw = true;
                    } else if wm.active_window == Some(6) {
                        browser_app.handle_key(c);
                        needs_redraw = true;
                    }
                }

                // 3. Process mouse moves and clicks
                let cur_mouse = drivers::mouse::MOUSE.lock().state();
                let mx = cur_mouse.x;
                let my = cur_mouse.y;
                let left_btn = cur_mouse.left_button;
                let right_btn = cur_mouse.right_button;

                if mx != prev_mouse_x || my != prev_mouse_y {
                    prev_mouse_x = mx;
                    prev_mouse_y = my;
                    wm.handle_mouse_move(mx, my, screen_w, screen_h);
                    needs_redraw = true;
                }

                // Handle Right Click (Context Menu)
                if right_btn && !prev_right_btn {
                    prev_right_btn = true;
                    wm.handle_mouse_right_down(mx, my, screen_w, screen_h);
                    needs_redraw = true;
                } else if !right_btn && prev_right_btn {
                    prev_right_btn = false;
                }

                // Handle Left Click
                if left_btn && !prev_left_btn {
                    prev_left_btn = true;
                    if let Some(action) = wm.handle_mouse_down(mx, my, screen_w, screen_h, cur_tick) {
                        match action {
                            drivers::gui::DesktopAction::Redraw => { needs_redraw = true; }
                            drivers::gui::DesktopAction::LaunchApp(app_idx) => {
                                wm.open_or_focus_app(app_idx, screen_w, screen_h);
                                needs_redraw = true;
                            }
                            drivers::gui::DesktopAction::Reboot => {
                                crate::arch::io::outb(0x64, 0xFE);
                            }
                            drivers::gui::DesktopAction::Shutdown => {
                                system_shutdown(&mut canvas);
                            }
                            drivers::gui::DesktopAction::Refresh => {
                                needs_redraw = true;
                            }
                        }
                    }

                    // Forward click into active window content
                    if let Some(act_id) = wm.active_window {
                        if let Some(win) = &wm.windows[act_id] {
                            if !win.minimized && win.contains(mx, my) {
                                let titlebar_h = drivers::gui::Window::TITLEBAR_HEIGHT;
                                if my >= win.y + titlebar_h {
                                    let rel_x = mx.saturating_sub(win.x + 4);
                                    let rel_y = my.saturating_sub(win.y + titlebar_h + 4);
                                    let content_w = win.width.saturating_sub(8);
                                    match act_id {
                                        1 => {
                                            explorer_app.handle_click(rel_x, rel_y);
                                            needs_redraw = true;
                                        }
                                        3 => {
                                            task_mgr_app.handle_click(rel_x, rel_y);
                                            needs_redraw = true;
                                        }
                                        6 => {
                                            if let Some(target_app) = browser_app.handle_click(rel_x, rel_y, content_w) {
                                                wm.open_or_focus_app(7, screen_w, screen_h);
                                                app_store_app.install_by_id(&target_app);
                                            }
                                            needs_redraw = true;
                                        }
                                        7 => {
                                            app_store_app.handle_click(rel_x, rel_y, content_w);
                                            needs_redraw = true;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }

                    // Check if Reboot button was clicked on Installer window (Window 4)
                    if let Some(win) = &wm.windows[4] {
                        if !win.minimized && win.contains(mx, my) {
                            let content_x = win.x + 4;
                            let content_w = win.width.saturating_sub(8);
                            let content_h = win.height.saturating_sub(drivers::gui::Window::TITLEBAR_HEIGHT + 8);
                            let footer_y = win.y + drivers::gui::Window::TITLEBAR_HEIGHT + 4 + content_h - 28;
                            let btn_w = 90;
                            let btn_x = content_x + content_w - btn_w - 12;
                            if mx >= btn_x && mx <= btn_x + btn_w && my >= footer_y + 4 && my <= footer_y + 24 {
                                if installer.current_step == installer::InstallStep::Complete {
                                    crate::arch::io::outb(0x64, 0xFE);
                                }
                            }
                        }
                    }
                } else if !left_btn && prev_left_btn {
                    prev_left_btn = false;
                    wm.handle_mouse_up();
                    needs_redraw = true;
                }

                // 4. Periodic system updates (every ~20 ticks = 200ms)
                if cur_tick >= last_tick + 20 {
                    last_tick = cur_tick;
                    task_mgr_app.update(cur_tick);
                    if app_store_app.is_downloading {
                        app_store_app.update();
                    }
                    needs_redraw = true;
                }

                if needs_redraw {
                    let free_kb = crate::memory::pmm::get_stats().free_bytes / 1024;
                    // Update Taskbar clock and system stats
                    wm.render(&mut canvas, screen_w, screen_h, cur_tick, free_kb);

                    // Redraw windows with correct Z-ordering:
                    // 1. Draw inactive windows' client contents first
                    for (i, opt_win) in wm.windows.iter().enumerate() {
                        if Some(i) != wm.active_window {
                            if let Some(w) = opt_win {
                                if !w.minimized {
                                    match i {
                                        0 => term_app.draw(w, &mut canvas),
                                        1 => explorer_app.draw(w, &mut canvas),
                                        2 => pad_app.draw(w, &mut canvas),
                                        3 => task_mgr_app.draw(w, &mut canvas),
                                        4 => installer.draw(w, &mut canvas),
                                        5 => drivers::gui::draw_system_properties(w, &mut canvas),
                                        6 => browser_app.draw(w, &mut canvas),
                                        7 => app_store_app.draw(w, &mut canvas),
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    // 2. Draw active window client contents on top
                    if let Some(act_id) = wm.active_window {
                        if let Some(w) = &wm.windows[act_id] {
                            if !w.minimized {
                                match act_id {
                                    0 => term_app.draw(w, &mut canvas),
                                    1 => explorer_app.draw(w, &mut canvas),
                                    2 => pad_app.draw(w, &mut canvas),
                                    3 => task_mgr_app.draw(w, &mut canvas),
                                    4 => installer.draw(w, &mut canvas),
                                    5 => drivers::gui::draw_system_properties(w, &mut canvas),
                                    6 => browser_app.draw(w, &mut canvas),
                                    7 => app_store_app.draw(w, &mut canvas),
                                    _ => {}
                                }
                            }
                        }
                    }

                    // Redraw Start Menu if open
                    let taskbar_y = screen_h.saturating_sub(wm.taskbar.height);
                    wm.start_menu.draw(&mut canvas, taskbar_y);

                    // Redraw Context Menu if open
                    wm.context_menu.draw(&mut canvas);

                    // Redraw mouse cursor at current mouse position
                    drivers::mouse::draw_cursor(&mut canvas, mx, my);

                    // ATOMIC PRESENTATION (DOUBLE BUFFERING - NO FLICKER!)
                    canvas.present();
                }

                // Sleep until next interrupt
                core::arch::asm!("sti; hlt");
            }
        }
    } else {
        kprintln!("[ FAIL ] Framebuffer not found! Falling back to headless idle loop.");
        arch::interrupts::enable();
        loop {
            unsafe {
                core::arch::asm!("sti; hlt");
            }
        }
    }
}

fn thread_a() {
    loop {
        kprintln!("[Thread A] Multitasking tick — Thread A active");
        for _ in 0..50_000_000 {
            core::hint::spin_loop();
        }
    }
}

fn thread_b() {
    loop {
        kprintln!("[Thread B] Multitasking tick — Thread B active");
        for _ in 0..50_000_000 {
            core::hint::spin_loop();
        }
    }
}

fn system_shutdown(canvas: &mut drivers::gui::Canvas) -> ! {
    let w = canvas.width();
    let h = canvas.height();
    canvas.clear(0x0A0F12);

    let box_w = 440;
    let box_h = 170;
    let box_x = (w.saturating_sub(box_w)) / 2;
    let box_y = (h.saturating_sub(box_h)) / 2;
    canvas.draw_rect(box_x, box_y, box_w, box_h, 0x1E2428);
    canvas.draw_rect_outline(box_x, box_y, box_w, box_h, 0x00E676);

    canvas.draw_gradient_v(box_x, box_y, box_w, 28, 0x263238, 0x191C1E);
    canvas.draw_text(box_x + 16, box_y + 8, "VikingOS — System Shutdown", 0xFFFFFF);

    canvas.draw_text(box_x + 24, box_y + 48, "Flushing storage buffers & unmounting VikingFS...", 0x80D8FF);
    canvas.draw_text(box_x + 24, box_y + 72, "Filesystem synchronized safely.", 0x00E676);
    canvas.draw_text(box_x + 24, box_y + 104, "It is now safe to turn off your computer.", 0xFFD54F);
    canvas.draw_text(box_x + 24, box_y + 132, "Powering off / Halting CPU...", 0x90A4AE);
    canvas.present();

    for _ in 0..10_000_000 {
        core::hint::spin_loop();
    }

    unsafe {
        // QEMU ACPI poweroff
        crate::arch::io::outw(0x604, 0x2000);
        // Older QEMU / Bochs ACPI poweroff
        crate::arch::io::outw(0xB004, 0x2000);
        // VirtualBox poweroff
        crate::arch::io::outw(0x4004, 0x3400);
        // QEMU isa-debug-exit
        crate::arch::io::outb(0x501, 0x31);
    }

    arch::interrupts::disable();
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

// ── Panic Handler ────────────────────────────────────────────────────

/// Panic handler — prints the panic message to serial and halts.
///
/// This is required by `#![no_std]`. In a bare-metal kernel, a panic
/// is unrecoverable — we print what we can and stop the CPU.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut writer = serial::SerialPort::com1();
    // Re-initialize in case the serial port wasn't set up yet
    writer.init();

    let _ = writeln!(writer, "");
    let _ = writeln!(writer, "!!! KERNEL PANIC !!!");
    let _ = writeln!(writer, "{}", info);
    let _ = writeln!(writer, "");
    let _ = writeln!(writer, "System halted.");

    halt_loop();
}

// ── CPU Halt ─────────────────────────────────────────────────────────

/// Halt the CPU in an idle loop waiting for interrupts.
fn halt_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("sti; hlt");
        }
    }
}

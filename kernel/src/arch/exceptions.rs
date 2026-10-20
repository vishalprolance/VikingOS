// VikingOS Kernel — x86-64 CPU Exception Handlers & Crash Dump Architecture
//
// Intercepts all 32 standard x86-64 CPU exceptions. Each exception has an
// assembly trampoline stub that pushes an aligned, uniform stack frame
// before jumping into the Rust exception dispatcher.
//
// For non-fatal traps (e.g., Breakpoint #BP), the handler logs the event and
// returns to let execution proceed. For fatal faults (e.g., Page Fault,
// Double Fault, General Protection Fault), a formatted crash dump with all
// register states is printed to the serial console before halting the CPU.

use core::fmt::Write;
use crate::serial::SerialPort;
use super::idt::{InterruptDescriptorTable, IDT_FLAG_INTERRUPT_GATE, IDT_FLAG_USER_TRAP_GATE};
use super::gdt::DOUBLE_FAULT_IST_INDEX;

/// Full CPU register snapshot captured on exception entry.
///
/// Perfectly mirrors the stack layout constructed by the assembly stubs.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExceptionContext {
    // General-purpose registers pushed by common_isr_stub:
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    // Vector number and error code pushed by specific stub:
    pub vector: u64,
    pub error_code: u64,

    // Hardware frame pushed automatically by CPU on interrupt:
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

// ── Assembly Trampoline Stubs ─────────────────────────────────────────

core::arch::global_asm!(
r#"
.macro isr_no_err nr
.global isr_stub_\nr
.type isr_stub_\nr, @function
isr_stub_\nr:
    push 0
    push \nr
    jmp common_isr_stub
.endm

.macro isr_err nr
.global isr_stub_\nr
.type isr_stub_\nr, @function
isr_stub_\nr:
    push \nr
    jmp common_isr_stub
.endm

// 0..31 Standard x86-64 CPU Exceptions:
isr_no_err 0    // #DE Divide Error
isr_no_err 1    // #DB Debug
isr_no_err 2    // #NMI Non-Maskable Interrupt
isr_no_err 3    // #BP Breakpoint
isr_no_err 4    // #OF Overflow
isr_no_err 5    // #BR Bound Range Exceeded
isr_no_err 6    // #UD Invalid Opcode
isr_no_err 7    // #NM Device Not Available
isr_err    8    // #DF Double Fault (has error code)
isr_no_err 9    // Coprocessor Segment Overrun
isr_err    10   // #TS Invalid TSS (has error code)
isr_err    11   // #NP Segment Not Present (has error code)
isr_err    12   // #SS Stack-Segment Fault (has error code)
isr_err    13   // #GP General Protection Fault (has error code)
isr_err    14   // #PF Page Fault (has error code)
isr_no_err 15   // Reserved
isr_no_err 16   // #MF x87 FPU Floating-Point Error
isr_err    17   // #AC Alignment Check (has error code)
isr_no_err 18   // #MC Machine Check
isr_no_err 19   // #XM SIMD Floating-Point Exception
isr_no_err 20   // #VE Virtualization Exception
isr_err    21   // #CP Control Protection (has error code)
isr_no_err 22   // Reserved
isr_no_err 23   // Reserved
isr_no_err 24   // Reserved
isr_no_err 25   // Reserved
isr_no_err 26   // Reserved
isr_no_err 27   // Reserved
isr_no_err 28   // #HV Hypervisor Injection
isr_err    29   // #VC VMM Communication (has error code)
isr_err    30   // #SX Security Exception (has error code)
isr_no_err 31   // Reserved

// 32..255 Hardware Interrupts:
.macro hw_isr nr
.global hw_isr_stub_\nr
.type hw_isr_stub_\nr, @function
hw_isr_stub_\nr:
    push 0            // Dummy error code
    push \nr          // Vector number
    jmp common_isr_stub
.endm

// Register timer vector 32
hw_isr 32

// Register keyboard vector 33
hw_isr 33

// Register mouse vector 44
hw_isr 44

.global common_isr_stub
.type common_isr_stub, @function
common_isr_stub:
    // Push general-purpose registers (15 registers = 120 bytes)
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push rbp
    push rdi
    push rsi
    push rdx
    push rcx
    push rbx
    push rax

    // First parameter in System V AMD64 ABI is in RDI:
    // Pass pointer to ExceptionContext (which starts at current RSP)
    mov rdi, rsp

    // Clear direction flag for standard C calling convention
    cld

    // Call Rust exception dispatcher
    call exception_dispatcher

    // Restore general-purpose registers
    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    pop rbp
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15

    // Pop vector and error code
    add rsp, 16

    // Return from interrupt
    iretq
"#
);

// Declare external assembly stub symbols
unsafe extern "C" {
    fn isr_stub_0();
    fn isr_stub_1();
    fn isr_stub_2();
    fn isr_stub_3();
    fn isr_stub_4();
    fn isr_stub_5();
    fn isr_stub_6();
    fn isr_stub_7();
    fn isr_stub_8();
    fn isr_stub_9();
    fn isr_stub_10();
    fn isr_stub_11();
    fn isr_stub_12();
    fn isr_stub_13();
    fn isr_stub_14();
    fn isr_stub_15();
    fn isr_stub_16();
    fn isr_stub_17();
    fn isr_stub_18();
    fn isr_stub_19();
    fn isr_stub_20();
    fn isr_stub_21();
    fn isr_stub_22();
    fn isr_stub_23();
    fn isr_stub_24();
    fn isr_stub_25();
    fn isr_stub_26();
    fn isr_stub_27();
    fn isr_stub_28();
    fn isr_stub_29();
    fn isr_stub_30();
    fn isr_stub_31();
    pub fn hw_isr_stub_32();
    pub fn hw_isr_stub_33();
    pub fn hw_isr_stub_44();
}

/// Table of all 32 exception stub entry points
pub const ISR_STUBS: [unsafe extern "C" fn(); 32] = [
    isr_stub_0, isr_stub_1, isr_stub_2, isr_stub_3,
    isr_stub_4, isr_stub_5, isr_stub_6, isr_stub_7,
    isr_stub_8, isr_stub_9, isr_stub_10, isr_stub_11,
    isr_stub_12, isr_stub_13, isr_stub_14, isr_stub_15,
    isr_stub_16, isr_stub_17, isr_stub_18, isr_stub_19,
    isr_stub_20, isr_stub_21, isr_stub_22, isr_stub_23,
    isr_stub_24, isr_stub_25, isr_stub_26, isr_stub_27,
    isr_stub_28, isr_stub_29, isr_stub_30, isr_stub_31,
];

/// Register all 32 standard CPU exception handlers in the IDT.
pub fn register_all(idt: &mut InterruptDescriptorTable) {
    for (i, stub) in ISR_STUBS.iter().enumerate() {
        let handler_addr = *stub as usize;
        let ist = if i == 8 {
            // Vector 8 (Double Fault) must use dedicated IST1 stack
            DOUBLE_FAULT_IST_INDEX
        } else {
            0
        };

        let flags = if i == 3 {
            // { la: "Breakpoint" }
            IDT_FLAG_USER_TRAP_GATE
        } else {
            IDT_FLAG_INTERRUPT_GATE
        };

        idt.set_handler(i, handler_addr, ist, flags);
    }
}

// ── Exception Dispatcher & Diagnostic Handlers ────────────────────────

/// Central exception dispatcher called from `common_isr_stub`.
#[unsafe(no_mangle)]
pub extern "C" fn exception_dispatcher(ctx: &mut ExceptionContext) {
    match ctx.vector {
        0 => handle_divide_error(ctx),
        1 => handle_debug(ctx),
        2 => handle_nmi(ctx),
        3 => handle_breakpoint(ctx),
        4 => handle_overflow(ctx),
        5 => handle_bound_range(ctx),
        6 => handle_invalid_opcode(ctx),
        7 => handle_device_not_available(ctx),
        8 => handle_double_fault(ctx),
        10 => handle_invalid_tss(ctx),
        11 => handle_segment_not_present(ctx),
        12 => handle_stack_segment_fault(ctx),
        13 => handle_general_protection_fault(ctx),
        14 => handle_page_fault(ctx),
        16 => handle_x87_floating_point(ctx),
        17 => handle_alignment_check(ctx),
        18 => handle_machine_check(ctx),
        19 => handle_simd_floating_point(ctx),
        32..=255 => {
            crate::arch::interrupt_dispatcher::rust_hardware_interrupt_dispatcher(ctx.vector as u8);
        }
        _ => handle_unhandled_exception(ctx),
    }
}

fn handle_divide_error(ctx: &ExceptionContext) {
    print_exception_banner("DIVIDE ERROR (#DE)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_debug(ctx: &ExceptionContext) {
    let mut writer = SerialPort::com1();
    let _ = writeln!(writer, "[ DEBUG ] Trap hit at RIP={:#018x}", ctx.rip);
}

fn handle_nmi(ctx: &ExceptionContext) {
    print_exception_banner("NON-MASKABLE INTERRUPT (#NMI)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_breakpoint(ctx: &ExceptionContext) {
    let mut writer = SerialPort::com1();
    let _ = writeln!(
        writer,
        "[ TRAP ] Breakpoint (#BP) intercepted at RIP={:#018x}, CS={:#06x}, RFLAGS={:#018x}",
        ctx.rip, ctx.cs, ctx.rflags
    );
    // Breakpoint is a trap — execution resumes at RIP automatically!
}

fn handle_overflow(ctx: &ExceptionContext) {
    print_exception_banner("OVERFLOW (#OF)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_bound_range(ctx: &ExceptionContext) {
    print_exception_banner("BOUND RANGE EXCEEDED (#BR)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_invalid_opcode(ctx: &ExceptionContext) {
    print_exception_banner("INVALID OPCODE (#UD)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_device_not_available(ctx: &ExceptionContext) {
    print_exception_banner("DEVICE NOT AVAILABLE (#NM)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_double_fault(ctx: &ExceptionContext) {
    print_exception_banner("DOUBLE FAULT (#DF) [ON DEDICATED IST1 STACK]");
    print_register_dump(ctx);
    halt_system();
}

fn handle_invalid_tss(ctx: &ExceptionContext) {
    print_exception_banner("INVALID TSS (#TS)");
    let mut writer = SerialPort::com1();
    let _ = writeln!(writer, "  Selector Index: {:#x}", ctx.error_code >> 3);
    let _ = writeln!(writer, "  External: {}", (ctx.error_code & 1) != 0);
    print_register_dump(ctx);
    halt_system();
}

fn handle_segment_not_present(ctx: &ExceptionContext) {
    print_exception_banner("SEGMENT NOT PRESENT (#NP)");
    let mut writer = SerialPort::com1();
    let _ = writeln!(writer, "  Selector Index: {:#x}", ctx.error_code >> 3);
    print_register_dump(ctx);
    halt_system();
}

fn handle_stack_segment_fault(ctx: &ExceptionContext) {
    print_exception_banner("STACK-SEGMENT FAULT (#SS)");
    let mut writer = SerialPort::com1();
    let _ = writeln!(writer, "  Selector Index: {:#x}", ctx.error_code >> 3);
    print_register_dump(ctx);
    halt_system();
}

fn handle_general_protection_fault(ctx: &ExceptionContext) {
    print_exception_banner("GENERAL PROTECTION FAULT (#GP)");
    let mut writer = SerialPort::com1();
    let _ = writeln!(writer, "  Error Code: {:#010x}", ctx.error_code);
    if ctx.error_code != 0 {
        let external = (ctx.error_code & 1) != 0;
        let idt = ((ctx.error_code >> 1) & 1) != 0;
        let table = match (ctx.error_code >> 2) & 1 {
            0 => "GDT",
            _ => "LDT",
        };
        let selector = (ctx.error_code >> 3) & 0x1FFF;
        let _ = writeln!(
            writer,
            "  Decoded: External={}, Table={} (IDT={}), Selector Index={:#x}",
            external, table, idt, selector
        );
    }
    print_register_dump(ctx);
    halt_system();
}

fn handle_page_fault(ctx: &ExceptionContext) {
    print_exception_banner("PAGE FAULT (#PF)");

    // Read faulting virtual address from CR2 register
    let cr2: u64;
    unsafe {
        core::arch::asm!("mov {0}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
    }

    let mut writer = SerialPort::com1();
    let _ = writeln!(writer, "  Faulting Virtual Address (CR2): {:#018x}", cr2);
    let _ = writeln!(writer, "  Error Code:                     {:#010x}", ctx.error_code);

    // Decode Page Fault Error Code:
    let present = (ctx.error_code & (1 << 0)) != 0;
    let write = (ctx.error_code & (1 << 1)) != 0;
    let user = (ctx.error_code & (1 << 2)) != 0;
    let reserved_write = (ctx.error_code & (1 << 3)) != 0;
    let instruction_fetch = (ctx.error_code & (1 << 4)) != 0;
    let protection_key = (ctx.error_code & (1 << 5)) != 0;
    let shadow_stack = (ctx.error_code & (1 << 6)) != 0;

    let _ = writeln!(
        writer,
        "  Violation Cause: [ {} | {} | {} | {} | {} ]",
        if present { "Protection Violation" } else { "Page Not Present" },
        if write { "Write Access" } else { "Read Access" },
        if user { "User Mode" } else { "Kernel/Supervisor Mode" },
        if instruction_fetch { "Instruction Fetch (NX)" } else { "Data Access" },
        if reserved_write { "Reserved Bit Set" } else { "No Reserved Bit" }
    );

    if protection_key {
        let _ = writeln!(writer, "  Protection Key Violation: Yes");
    }
    if shadow_stack {
        let _ = writeln!(writer, "  Shadow Stack Violation:   Yes");
    }

    print_register_dump(ctx);
    halt_system();
}

fn handle_x87_floating_point(ctx: &ExceptionContext) {
    print_exception_banner("x87 FLOATING-POINT EXCEPTION (#MF)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_alignment_check(ctx: &ExceptionContext) {
    print_exception_banner("ALIGNMENT CHECK (#AC)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_machine_check(ctx: &ExceptionContext) {
    print_exception_banner("MACHINE CHECK (#MC)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_simd_floating_point(ctx: &ExceptionContext) {
    print_exception_banner("SIMD FLOATING-POINT EXCEPTION (#XM)");
    print_register_dump(ctx);
    halt_system();
}

fn handle_unhandled_exception(ctx: &ExceptionContext) {
    print_exception_banner("UNHANDLED CPU EXCEPTION");
    print_register_dump(ctx);
    halt_system();
}

// ── Diagnostic Formatting Utilities ──────────────────────────────────

fn print_exception_banner(name: &str) {
    let mut writer = SerialPort::com1();
    let _ = writeln!(writer, "");
    let _ = writeln!(writer, "╔═══════════════════════════════════════════════════════════════════════╗");
    let _ = writeln!(writer, "║  FATAL CPU EXCEPTION: {:<47} ║", name);
    let _ = writeln!(writer, "╚═══════════════════════════════════════════════════════════════════════╝");
    crate::kprintln!();
    crate::kprintln!("!!! FATAL CPU EXCEPTION: {} !!!", name);
}

fn print_register_dump(ctx: &ExceptionContext) {
    let mut writer = SerialPort::com1();
    let _ = writeln!(writer, "  Vector:     {:#04x} ({})", ctx.vector, ctx.vector);
    let _ = writeln!(writer, "  Error Code: {:#018x}", ctx.error_code);
    let _ = writeln!(writer, "  RIP:        {:#018x}  CS:  {:#06x}", ctx.rip, ctx.cs);
    let _ = writeln!(writer, "  RSP:        {:#018x}  SS:  {:#06x}", ctx.rsp, ctx.ss);
    let _ = writeln!(writer, "  RFLAGS:     {:#018x}", ctx.rflags);
    let _ = writeln!(writer, "");
    let _ = writeln!(writer, "  Registers:");
    let _ = writeln!(writer, "    RAX: {:#018x}  RBX: {:#018x}", ctx.rax, ctx.rbx);
    let _ = writeln!(writer, "    RCX: {:#018x}  RDX: {:#018x}", ctx.rcx, ctx.rdx);
    let _ = writeln!(writer, "    RSI: {:#018x}  RDI: {:#018x}", ctx.rsi, ctx.rdi);
    let _ = writeln!(writer, "    RBP: {:#018x}  R8:  {:#018x}", ctx.rbp, ctx.r8);
    let _ = writeln!(writer, "    R9:  {:#018x}  R10: {:#018x}", ctx.r9, ctx.r10);
    let _ = writeln!(writer, "    R11: {:#018x}  R12: {:#018x}", ctx.r11, ctx.r12);
    let _ = writeln!(writer, "    R13: {:#018x}  R14: {:#018x}", ctx.r13, ctx.r14);
    let _ = writeln!(writer, "    R15: {:#018x}", ctx.r15);
    let _ = writeln!(writer, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    crate::kprintln!("  Vector: {:#04x} ({}) | Error: {:#018x}", ctx.vector, ctx.vector, ctx.error_code);
    crate::kprintln!("  RIP: {:#018x} | RSP: {:#018x} | CS: {:#06x}", ctx.rip, ctx.rsp, ctx.cs);
}

fn halt_system() -> ! {
    let mut writer = SerialPort::com1();
    let _ = writeln!(writer, "  System halted due to fatal exception.");
    let _ = writeln!(writer, "");
    crate::kprintln!("  System halted due to fatal exception.");
    loop {
        unsafe {
            core::arch::asm!("sti; hlt");
        }
    }
}

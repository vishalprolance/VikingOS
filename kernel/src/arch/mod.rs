// VikingOS Kernel — Architecture Layer (x86-64)
//
// Initializes low-level CPU structures:
//   - Global Descriptor Table (GDT)
//   - Task State Segment (TSS) with Double Fault Interrupt Stack Table (IST1)
//   - Interrupt Descriptor Table (IDT)
//   - CPU Exception Handlers (0..31)

pub mod gdt;
pub mod idt;
pub mod exceptions;
pub mod io;
pub mod interrupts;
pub mod apic;
pub mod interrupt_dispatcher;
pub mod context;
pub mod ioapic;

/// Initialize all x86-64 CPU core architecture structures:
///   1. Loads the GDT and reloads CS/DS/ES/SS segment registers
///   2. Loads the Task Register (LTR) with the TSS
///   3. Registers all 32 exception handlers in the IDT
///   4. Loads the IDT using `lidt`
pub fn init() {
    // 0. Mask legacy 8259 PIC so BIOS legacy IRQs do not collide with CPU exceptions
    interrupts::disable_pic();

    // 1. Setup GDT and TSS
    gdt::init();

    // 2. Setup IDT and Exception Handlers
    let idt = idt::get_idt();
    exceptions::register_all(idt);
    idt.load();
}

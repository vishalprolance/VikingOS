// VikingOS Kernel — Global Descriptor Table (GDT) & Task State Segment (TSS)
//
// In x86-64 long mode, memory segmentation is largely disabled, but the GDT
// is still strictly required to define:
//   1. Privilege levels (Ring 0 for kernel, Ring 3 for userspace)
//   2. 64-bit code segments (Long mode flag L=1, Default size D=0)
//   3. Task State Segment (TSS) containing the Interrupt Stack Table (IST)
//
// The IST allows critical exceptions (such as Double Fault) to switch to a
// known-good, dedicated stack, preventing hardware triple faults and resets.

use core::mem::size_of;

/// Null Segment Selector (offset 0x00)
#[allow(dead_code)]
pub const NULL_SELECTOR: u16 = 0x00;
/// Kernel Code Segment Selector (Index 1, RPL 0, offset 0x08)
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
/// Kernel Data Segment Selector (Index 2, RPL 0, offset 0x10)
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
/// User Data Segment Selector (Index 3, RPL 3, offset 0x18 | 3 = 0x1B)
#[allow(dead_code)]
pub const USER_DATA_SELECTOR: u16 = 0x18 | 3;
/// User Code Segment Selector (Index 4, RPL 3, offset 0x20 | 3 = 0x23)
#[allow(dead_code)]
pub const USER_CODE_SELECTOR: u16 = 0x20 | 3;
/// Task State Segment Selector (Index 5, RPL 0, offset 0x28)
pub const TSS_SELECTOR: u16 = 0x28;

/// Dedicated stack size for Double Fault handler (16 KiB)
pub const DOUBLE_FAULT_STACK_SIZE: usize = 16384;

/// Index in TSS IST for Double Fault handler
pub const DOUBLE_FAULT_IST_INDEX: u8 = 1;

/// Task State Segment (64-bit x86 architecture)
///
/// Holds Privilege Level 0..2 stack pointers (RSP0-RSP2) and seven
/// Interrupt Stack Table pointers (IST1-IST7).
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved_0: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    reserved_1: u64,
    pub ist1: u64,
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    reserved_2: u64,
    reserved_3: u16,
    pub iopb_offset: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            reserved_0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved_1: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved_2: 0,
            reserved_3: 0,
            // Setting iopb_offset >= sizeof(TSS) disables I/O permission bitmap
            iopb_offset: 104,
        }
    }
}

/// 16-byte aligned stack for Double Fault exception handling
#[repr(align(16))]
#[allow(dead_code)]
struct DoubleFaultStack([u8; DOUBLE_FAULT_STACK_SIZE]);

/// Dedicated stack memory for Double Faults
static mut DOUBLE_FAULT_STACK: DoubleFaultStack = DoubleFaultStack([0; DOUBLE_FAULT_STACK_SIZE]);

/// Kernel Task State Segment singleton
static mut TSS: TaskStateSegment = TaskStateSegment::new();

/// Global Descriptor Table entries:
/// 0: Null descriptor
/// 1: Kernel Code 64-bit
/// 2: Kernel Data 64-bit
/// 3: User Data 64-bit
/// 4: User Code 64-bit
/// 5: TSS Descriptor (low 64 bits)
/// 6: TSS Descriptor (high 64 bits)
static mut GDT: [u64; 7] = [0; 7];

/// Pseudo-descriptor loaded into the CPU GDTR register
#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

/// Initialize and activate the VikingOS GDT and TSS.
///
/// Sets up kernel and user segments, initializes the double-fault IST stack,
/// loads the GDT using `lgdt`, reloads segment registers with new selectors,
/// and activates the TSS using `ltr`.
pub fn init() {
    unsafe {
        // 1. Configure TSS with Double Fault IST1 stack
        let stack_start = core::ptr::addr_of!(DOUBLE_FAULT_STACK) as u64;
        let stack_top = stack_start + (DOUBLE_FAULT_STACK_SIZE as u64);
        TSS.ist1 = stack_top;

        // 2. Build GDT entries
        // Entry 0: Null Descriptor
        GDT[0] = 0x0000_0000_0000_0000;

        // Entry 1: Kernel Code 64-bit (offset 0x08)
        // Access: Present (1), Ring 0 (00), Code/Data (1), Executable (1), Readable (1), Accessed (0) -> 0x9A
        // Flags: Long mode (1), Default size (0), Granularity (0) -> 0x20
        GDT[1] = 0x0020_9A00_0000_0000;

        // Entry 2: Kernel Data 64-bit (offset 0x10)
        // Access: Present (1), Ring 0 (00), Code/Data (1), Executable (0), Writable (1), Accessed (0) -> 0x92
        // Flags: 0x00
        GDT[2] = 0x0000_9200_0000_0000;

        // Entry 3: User Data 64-bit (offset 0x18)
        // Access: Present (1), Ring 3 (11), Code/Data (1), Executable (0), Writable (1), Accessed (0) -> 0xF2
        // Flags: 0x00
        GDT[3] = 0x0000_F200_0000_0000;

        // Entry 4: User Code 64-bit (offset 0x20)
        // Access: Present (1), Ring 3 (11), Code/Data (1), Executable (1), Readable (1), Accessed (0) -> 0xFA
        // Flags: Long mode (1) -> 0x20
        GDT[4] = 0x0020_FA00_0000_0000;

        // Entries 5 & 6: 16-byte TSS Descriptor (offset 0x28)
        let tss_addr = core::ptr::addr_of!(TSS) as u64;
        let tss_limit = (size_of::<TaskStateSegment>() - 1) as u64;

        let limit_low = tss_limit & 0xFFFF;
        let base_low = (tss_addr & 0x00FFFFFF) << 16;
        // Type 0x89: Present (bit 47), DPL 0 (bits 45-46), 64-bit TSS Available (bits 40-43: 0b1001)
        let access = 0x89u64 << 40;
        let limit_high = ((tss_limit >> 16) & 0x0F) << 48;
        let base_mid = ((tss_addr >> 24) & 0xFF) << 56;

        let tss_low = limit_low | base_low | access | limit_high | base_mid;
        let tss_high = tss_addr >> 32;

        GDT[5] = tss_low;
        GDT[6] = tss_high;

        // 3. Load GDT into CPU
        let gdt_ptr = GdtPointer {
            limit: (size_of::<[u64; 7]>() - 1) as u16,
            base: core::ptr::addr_of!(GDT) as u64,
        };

        core::arch::asm!(
            "lgdt [{0}]",
            in(reg) &gdt_ptr,
            options(readonly, nostack, preserves_flags),
        );

        // 4. Reload CS via far return (retfq) and data segment registers
        core::arch::asm!(
            "push {code_sel}",
            "lea {tmp}, [2f + rip]",
            "push {tmp}",
            "retfq",
            "2:",
            "mov ds, {data_sel:x}",
            "mov es, {data_sel:x}",
            "mov fs, {data_sel:x}",
            "mov gs, {data_sel:x}",
            "mov ss, {data_sel:x}",
            code_sel = in(reg) (KERNEL_CODE_SELECTOR as u64),
            data_sel = in(reg) KERNEL_DATA_SELECTOR,
            tmp = out(reg) _,
        );

        // 5. Load Task Register (LTR)
        core::arch::asm!(
            "ltr {0:x}",
            in(reg) TSS_SELECTOR,
            options(nostack, preserves_flags),
        );
    }
}

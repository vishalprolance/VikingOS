/*
 * VikingOS Kernel Linker Script
 *
 * Places the kernel in the higher-half virtual address space as required
 * by the Limine boot protocol. Preserves Limine request structures that
 * the bootloader scans to discover kernel capabilities.
 */

ENTRY(_start)

PHDRS
{
    text    PT_LOAD FLAGS((1 << 0) | (1 << 2)); /* Execute + Read */
    rodata  PT_LOAD FLAGS((1 << 2));             /* Read only      */
    data    PT_LOAD FLAGS((1 << 1) | (1 << 2)); /* Write + Read   */
}

SECTIONS
{
    /*
     * Higher-half kernel base address.
     * Limine maps the kernel into the top 2 GiB of the 64-bit
     * virtual address space.
     */
    . = 0xffffffff80000000;

    .text : {
        *(.text.unlikely .text.unlikely.*)
        *(.text .text.*)
    } :text

    . = ALIGN(0x1000);

    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    . = ALIGN(0x1000);

    /*
     * Limine requests must survive linker garbage collection.
     * The bootloader scans the ELF for these marker and request
     * sections to discover what services the kernel needs.
     */
    .data : {
        KEEP(*(.requests_start_marker))
        KEEP(*(.requests))
        KEEP(*(.requests_end_marker))
        *(.data .data.*)
    } :data

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :data

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
        *(.comment)
    }
}

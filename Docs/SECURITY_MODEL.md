# Security Model: VikingOS

VikingOS prioritizes memory safety through the use of Rust, but acknowledges the inherent risks of kernel-level development.

## Security Boundaries

### 1. Kernel/User Boundary (Ring 0 vs Ring 3)
- **Enforcement**: Hardware-level enforcement via GDT/Paging.
- **Transition**: Controlled entry points via the `syscall` instruction.
- **Validation**: All user-provided pointers and values must be validated against the process's virtual memory bounds before use.

### 2. Process Isolation
- **Memory**: Each process possesses a unique PML4 table, ensuring it cannot access other processes' or the kernel's private memory.
- **Privileges**: User processes run with minimum necessary privileges; hardware access is restricted to kernel drivers.

### 3. Memory Safety Strategy
- **Safe Rust**: Preferred for all high-level logic.
- **Unsafe Rust**: Strictly confined to low-level hardware interactions (e.g., I/O ports, page table manipulation). Every `unsafe` block must be documented.
- **Panic Handling**: Kernel panics result in a safe system halt to prevent undefined behavior or state corruption.

## Threat Model
- **Malformed Executables**: PE loader must be hardened against buffer overflows and invalid headers.
- **System Call Abuse**: Syscall handlers must validate all arguments to prevent privilege escalation.
- **Driver Failures**: Future drivers should be isolated or wrapped in safety layers to prevent a single device failure from crashing the entire kernel.

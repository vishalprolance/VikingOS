# Compatibility Plan: Windows Applications

VikingOS aims to provide a compatibility layer for x86-64 Windows applications.

## Target Compatibility Levels

### Level 1: Console Applications
- **Requirement**: PE loader, basic C runtime (CRT), and Win32 Console API.
- **Goal**: Run simple CLI tools.

### Level 2: Basic GUI Applications
- **Requirement**: Windowing API, GDI (Graphics Device Interface), and Message Loop.
- **Goal**: Run simple Windows utilities and legacy software.

### Level 3: Advanced Applications
- **Requirement**: Complex DLL dependencies, advanced synchronization, and filesystem behavior.
- **Goal**: Run a broad set of third-party software.

## Implementation Strategy

### 1. Executable Loading
- **PE32+ Parsing**: Implementation of a loader that understands the Portable Executable format.
- **Relocation**: Handling base relocations to load binaries at arbitrary virtual addresses.
- **Import Resolution**: Mapping Windows DLL imports to VikingOS compatibility functions.

### 2. API Mapping
- **Syscall Translation**: Mapping `ntdll.dll` / `kernel32.dll` calls to the VikingOS kernel.
- **ABI Adherence**: Strict adherence to the Microsoft x64 calling convention.

### 3. Compatibility Testing
- **Test Suite**: A curated set of open-source Windows applications (e.g., Notepad++, simple games).
- **Verification**: Comparison of output and behavior against a real Windows environment.

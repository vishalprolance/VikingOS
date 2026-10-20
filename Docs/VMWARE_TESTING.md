# VikingOS VMware Workstation Testing Guide

This document describes how to test and install VikingOS in VMware Workstation or VMware Workstation Player.

> **Status**: Verified compatible. The ISO boots in both UEFI and BIOS mode via Limine, provides a 32-bpp graphical desktop, and includes a guided OS installer with MBR partitioning and native VikingFS disk deployment.

---

## Prerequisites

- **VMware Workstation 16+** or **VMware Workstation Player**
- **VikingOS ISO image** (`build/VikingOS.iso`), built via:
  ```bash
  make iso
  # Or on Windows without make:
  cd kernel && cargo build --release && cd .. && python scripts/build_iso.py
  ```

---

## ⚡ Quick Setup (Typical — Recommended)

You can create and configure the virtual machine in just 4 simple steps:

### Step 1: Launch the New Virtual Machine Wizard
1. Open VMware Workstation.
2. Click **File → New Virtual Machine** (or `Ctrl+N`).
3. Select **Typical (recommended)** and click **Next**.

### Step 2: Select the ISO & Operating System
1. Choose **Installer disc image file (iso)** and click **Browse...** to select `build/VikingOS.iso`.
2. *(VMware may note: "Could not detect which operating system is in this disc image")*. Click **Next**.
3. Set **Guest operating system**: **Other**.
4. Set **Version**: **Other 64-bit**. Click **Next**.

### Step 3: Name & Virtual Disk (Enables OS Installation)
1. **Virtual machine name**: `VikingOS` (choose any preferred save folder). Click **Next**.
2. **Maximum disk size**: Set to **2 GB** (or standard **8–20 GB**).
3. Select **"Store virtual disk as a single file"**.
   > [!TIP]
   > Creating a virtual disk allows you to test the **VikingOS Guided OS Installer & Disk Persistence** (Milestone 12). If you only want to test the read-only Live ISO, you can skip formatting the disk.
4. Click **Next**.

### Step 4: Memory & Hardware Adjustment
1. On the "Ready to Create Virtual Machine" screen, click **Customize Hardware...**:
   - **Memory**: Set to **512 MB** or **1024 MB** (recommended for Heap, Framebuffer, and Ramdisk).
   - **Processors**: 1 processor, 2 cores (recommended for multitasking tests).
   - *(Optional)* **Add Serial Port**: Click **Add...** → **Serial Port** → **Output to file** → Choose `vikingos_serial.log` (captures COM1 kernel diagnostics).
2. Click **Close** and then **Finish**.

---

## 🚀 Alternative: Instant `.vmx` Quick Start (No Wizard Needed)

If you prefer skipping the wizard entirely, create a folder named `VikingOS`, create a file named `VikingOS.vmx`, and paste the following configuration:

```ini
.encoding = "windows-1252"
config.version = "8"
virtualHW.version = "19"
vmci0.present = "TRUE"
displayName = "VikingOS"
guestOS = "other-64"
memsize = "1024"
numvcpus = "2"

# UEFI Firmware (Limine supports both UEFI and BIOS)
firmware = "efi"

# Framebuffer & Display
svga.autodetect = "TRUE"

# ISO Boot Drive
ide1:0.present = "TRUE"
ide1:0.fileName = "build/VikingOS.iso"
ide1:0.deviceType = "cdrom-image"
ide1:0.startConnected = "TRUE"

# Virtual Hard Disk (For OS Installation testing)
scsi0.present = "TRUE"
scsi0.virtualDev = "lsilogic"
scsi0:0.present = "TRUE"
scsi0:0.fileName = "VikingOS.vmdk"

# COM1 Serial Diagnostics Port
serial0.present = "TRUE"
serial0.fileType = "file"
serial0.fileName = "vikingos_serial.log"
serial0.autodetect = "FALSE"
```

Open VMware, click **File → Open...**, browse to `VikingOS.vmx`, and click **Power on this virtual machine**.

---

## 💾 VikingOS Installation & Disk Persistence (Milestone 12)

### Is OS Installation Covered?
**Yes.** VikingOS includes a complete **Guided System Installer Engine & Wizard** (`kernel/src/installer/mod.rs`) along with an MBR partition manager (`kernel/src/storage/partition.rs`) and native VikingFS filesystem (`kernel/src/fs/vikingfs.rs`).

### How It Works:
1. **Live ISO Mode**: 
   - When booted from the CD/DVD ISO, VikingOS starts in a Live environment.
   - The kernel mounts an in-memory root filesystem (`/`) and starts the 2D window server and desktop applications.
2. **Launching the Installer Wizard**:
   - **GUI Window**: An automated installation wizard window ("VikingOS Installer") opens on the desktop upon boot, or can be opened from the **Start Menu** (`VIKING` → `Installer`).
   - **Command Line**: Open the **Viking Terminal** window and execute the `install` command:
     ```text
     viking> install
     ```
3. **5-Stage Automated Installation Workflow**:
   - **Stage 1 (Partitioning)**: Generates a standard MBR partition table with an active bootable partition (Type `0x83`, LBA `2048`).
   - **Stage 2 (Formatting)**: Formats the partition with native VikingFS (superblock magic `VIKI`).
   - **Stage 3 (File Deployment)**: Deploys `/boot/viking_kernel`, `/boot/limine.conf`, and `system.ini` directly to the partition.
   - **Stage 4 (Bootloader Embed)**: Embeds Limine BIOS/GPT boot stages into the MBR gap.
   - **Stage 5 (Complete)**: Marks the target drive bootable.

### Booting from the Installed Hard Disk:
Once installed to the virtual disk:
1. Shut down the VM (**VM → Power → Shut Down Guest** or close the window).
2. Go to **VM → Settings → CD/DVD (IDE)**.
3. Uncheck **"Connected"** and uncheck **"Connect at power on"** (or change VM Options → Advanced → Firmware Boot Order to prioritize the hard disk).
4. Power on the VM. Limine will load the installed VikingOS kernel directly from the virtual hard drive.

> [!NOTE]
> VikingOS includes a native **ATA PIO Hard Disk Controller Driver** (`kernel/src/storage/ata.rs`) that directly detects and operates VMware Workstation's virtual IDE hard disks (`ide0:0` / `.vmdk`), QEMU IDE disks, and physical bare-metal ATA drives. The guided installer partitions the real virtual disk with MBR, writes the partition table, formats it with native VikingFS, and deploys system files.

---

## 🔍 Serial Diagnostics & Debugging

VikingOS outputs comprehensive initialization diagnostics to the COM1 serial port at 115200 baud (8 data bits, 1 stop bit, no parity).

### Viewing the Output:
- **File Output**: If you configured the serial port to write to `vikingos_serial.log`, open it with VS Code, Notepad, or `Get-Content -Wait vikingos_serial.log`.
- **Named Pipe**: If configured as a named pipe (`\\.\pipe\vikingos_serial`), connect PuTTY or Tera Term set to **Serial / Named Pipe** mode.

---

## 📋 Expected Boot Sequence & Output

Upon powering on the VM, you will see the Limine bootloader menu, followed by the graphical desktop and the following COM1 diagnostics:

```text
 __      ___ _    _              ___  ____
 \ \    / (_) | _(_)_ __   __ _ / _ \/ ___|
  \ \  / /| | |/ / | '_ \ / _` | | | \___ \
   \ \/ / | |   <| | | | | (_| | |_| |___) |
    \__/  |_|_|\_\_|_| |_|\__, |\___/|____/
                          |___/

  VikingOS Kernel v0.1.0
  Architecture: x86_64
  Author:       Vishal A Ker
  Bootloader:   Limine
  ...
[ INIT ] Initializing x86-64 CPU architecture...
[  OK  ] GDT loaded (CS: 0x08, DS: 0x10, User CS: 0x23, User DS: 0x1B)
[  OK  ] TSS active (Selector: 0x28, IST1 Double Fault stack: 16 KiB)
[  OK  ] IDT loaded (256 vectors, 32 exception handlers registered)
[ INIT ] Initializing Memory Management Subsystem (Milestone 3)...
[  OK  ] Allocated physical frame at 0x...
[  OK  ] Frame read/write verification passed via HHDM
[  OK  ] Allocated 4 contiguous frames
[  OK  ] Virtual page unmapped and TLB invalidated successfully
[  OK  ] Box<u64> allocated
[  OK  ] Vec<u64> populated
[  OK  ] String & format! operational
[  OK  ] Arc<u32> reference counting verified
[ INIT ] Initializing Local APIC (Milestone 4)...
[  OK  ] Local APIC initialized (Base: 0xfee00000)
[  OK  ] IDT: Registered APIC Timer handler (Vector 32)
[  OK  ] IDT: Registered Keyboard handler (Vector 33)
[  OK  ] APIC Timer configured to 100Hz
[  OK  ] I/O APIC configured: IRQ 1 -> Vector 33
[  OK  ] Keyboard driver initialized
[ INIT ] Initializing Graphical Console & Window Compositor...
[  OK  ] GUI Desktop, Window Manager & Applications rendered
[  OK  ] Console initialized (1024x768, 32bpp)
[ INIT ] Initializing Storage Subsystem & Disk Partitioning...
[  OK  ] Ramdisk filesystem mounted at /
[  OK  ] Disk partitioned (MBR) and formatted with VikingFS
[ INIT ] Configuring System Call Interface...
[  OK  ] Syscall MSRs configured (LSTAR -> dispatcher)
[ INIT ] Loading user-mode applications...
[  OK  ] Loaded Notepad (PID 1)
[  OK  ] Loaded System Monitor (PID 2)
[ TEST ] Triggering CPU breakpoint trap (int3)...
[  OK  ] Execution resumed successfully after breakpoint trap!
[ INIT ] Initializing Kernel Scheduler...
[  OK  ] Scheduler initialized
[  OK  ] Test threads spawned (A and B)
[ FINAL ] Enabling hardware interrupts... (The system will now start ticking)
[  OK  ] Interrupts enabled. Heartbeat active.
[ START ] Transitioning to multitasking environment...
[Thread A] Multitasking tick — Thread A active
[Thread B] Multitasking tick — Thread B active
```

---

## 🛠️ Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| Black screen / No Limine menu | ISO not attached or disconnected | Check CD/DVD settings, verify "Connected" and "Connect at power on" are checked. |
| "Operating system not found" | Firmware mismatch | In VM Settings → Options → Advanced, toggle between **UEFI** and **BIOS**. Both are supported. |
| Secure Boot violation | Secure Boot enabled | In VM Settings → Options → Advanced, ensure **Secure Boot is disabled** (experimental kernel is unsigned). |
| No serial output in log file | Missing Serial Port device | Add a Serial Port pointing to a file per the Quick Setup or `.vmx` configuration. |
| Mouse cursor does not move | Mouse uncaptured | Click inside the VM window to focus the mouse into VMware. Press `Ctrl+Alt` to ungrab. |
| Low resolution / distorted UI | Default VESA buffer size | Set VM RAM to at least 512 MB to ensure sufficient linear framebuffer allocation. |

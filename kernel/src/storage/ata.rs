// VikingOS Kernel — ATA PIO Hard Disk Driver (Milestone 8 & 12)
//
// Provides direct block read/write access to the Primary Master IDE / ATA hard drive (I/O 0x1F0).
// Directly compatible with VMware Workstation (ide0:0 .vmdk), VirtualBox, and QEMU.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use crate::arch::io::{inb, inw, outb, outw};
use crate::storage::{BlockDevice, StorageError};
use crate::sync::SpinLock;

const ATA_DATA: u16 = 0x1F0;
const ATA_ERROR: u16 = 0x1F1;
const ATA_SECTOR_CNT: u16 = 0x1F2;
const ATA_LBA_LOW: u16 = 0x1F3;
const ATA_LBA_MID: u16 = 0x1F4;
const ATA_LBA_HIGH: u16 = 0x1F5;
const ATA_DRIVE_SEL: u16 = 0x1F6;
const ATA_STATUS: u16 = 0x1F7;
const ATA_COMMAND: u16 = 0x1F7;
const ATA_CONTROL: u16 = 0x3F6;

// Status register bits
const STATUS_BSY: u8 = 0x80;
const STATUS_DRDY: u8 = 0x40;
const STATUS_DF: u8 = 0x20;
const STATUS_DRQ: u8 = 0x08;
const STATUS_ERR: u8 = 0x01;

// Commands
const CMD_READ_SECTORS: u8 = 0x20;
const CMD_WRITE_SECTORS: u8 = 0x30;
const CMD_IDENTIFY: u8 = 0xEC;
const CMD_CACHE_FLUSH: u8 = 0xE7;

/// ATA PIO Block Device implementation for the primary master IDE drive.
pub struct AtaPioBlockDevice {
    total_sectors: u64,
    model: String,
    lock: SpinLock<()>,
}

impl AtaPioBlockDevice {
    /// Detect if an ATA Primary Master drive is present.
    pub fn detect() -> Option<Self> {
        unsafe {
            // Select Master drive (0xA0)
            outb(ATA_DRIVE_SEL, 0xA0);
            for _ in 0..4 {
                inb(ATA_CONTROL);
            } // 400ns delay

            outb(ATA_SECTOR_CNT, 0);
            outb(ATA_LBA_LOW, 0);
            outb(ATA_LBA_MID, 0);
            outb(ATA_LBA_HIGH, 0);

            // Send IDENTIFY command
            outb(ATA_COMMAND, CMD_IDENTIFY);
            for _ in 0..4 {
                inb(ATA_CONTROL);
            }

            let status = inb(ATA_STATUS);
            if status == 0 || status == 0xFF {
                return None; // No device attached
            }

            // Wait until BSY clears
            let mut timeout = 100_000;
            while (inb(ATA_STATUS) & STATUS_BSY) != 0 && timeout > 0 {
                timeout -= 1;
            }
            if timeout == 0 {
                return None;
            }

            // Check if LBA_MID or LBA_HIGH are non-zero (indicates ATAPI CD-ROM device, not ATA disk)
            let mid = inb(ATA_LBA_MID);
            let high = inb(ATA_LBA_HIGH);
            if mid != 0 || high != 0 {
                return None; // ATAPI device (CD-ROM)
            }

            // Wait until DRQ or ERR is set
            timeout = 100_000;
            let mut poll_status = inb(ATA_STATUS);
            while (poll_status & (STATUS_DRQ | STATUS_ERR)) == 0 && timeout > 0 {
                poll_status = inb(ATA_STATUS);
                timeout -= 1;
            }
            if (poll_status & STATUS_ERR) != 0 || (poll_status & STATUS_DRQ) == 0 {
                return None;
            }

            // Read 256 words (512 bytes) of IDENTIFY data
            let mut id_buf = [0u16; 256];
            for word in id_buf.iter_mut() {
                *word = inw(ATA_DATA);
            }

            // Word 60-61: Total 28-bit LBA sectors
            let lba28_sectors = (id_buf[60] as u64) | ((id_buf[61] as u64) << 16);

            // Word 83 bit 10: Supports 48-bit LBA
            let lba48_supported = (id_buf[83] & (1 << 10)) != 0;
            let total_sectors = if lba48_supported {
                let s0 = id_buf[100] as u64;
                let s1 = id_buf[101] as u64;
                let s2 = id_buf[102] as u64;
                let s3 = id_buf[103] as u64;
                let lba48 = s0 | (s1 << 16) | (s2 << 32) | (s3 << 48);
                if lba48 > 0 {
                    lba48
                } else {
                    lba28_sectors
                }
            } else {
                lba28_sectors
            };

            if total_sectors == 0 {
                return None;
            }

            // Words 27..46: Model string (40 bytes ASCII, byte swapped in each word)
            let mut model_bytes = Vec::with_capacity(40);
            for w in &id_buf[27..=46] {
                model_bytes.push((w >> 8) as u8);
                model_bytes.push((w & 0xFF) as u8);
            }
            let model_str = core::str::from_utf8(&model_bytes).unwrap_or("VMware Virtual IDE Hard Drive");
            let model = String::from(model_str.trim());

            Some(Self {
                total_sectors,
                model,
                lock: SpinLock::new(()),
            })
        }
    }

    /// Size of the hard drive in megabytes.
    pub fn size_mb(&self) -> u64 {
        (self.total_sectors * 512) / (1024 * 1024)
    }

    /// Detected drive model string.
    pub fn model_name(&self) -> &str {
        &self.model
    }

    fn wait_ready(&self) -> Result<(), StorageError> {
        unsafe {
            let mut timeout = 100_000;
            while timeout > 0 {
                let status = inb(ATA_STATUS);
                if (status & STATUS_BSY) == 0 && (status & STATUS_DRDY) != 0 {
                    return Ok(());
                }
                if (status & STATUS_ERR) != 0 || (status & STATUS_DF) != 0 {
                    return Err(StorageError::DeviceNotReady);
                }
                timeout -= 1;
            }
            Err(StorageError::DeviceNotReady)
        }
    }
}

impl BlockDevice for AtaPioBlockDevice {
    fn read_block(&self, block_id: u64, buffer: &mut [u8]) -> Result<(), StorageError> {
        if block_id >= self.total_sectors || buffer.len() < 512 {
            return Err(StorageError::InvalidBlock);
        }

        let _guard = self.lock.lock();
        self.wait_ready()?;

        unsafe {
            outb(ATA_DRIVE_SEL, 0xE0 | (((block_id >> 24) & 0x0F) as u8));
            for _ in 0..4 {
                inb(ATA_CONTROL);
            }

            outb(ATA_SECTOR_CNT, 1);
            outb(ATA_LBA_LOW, block_id as u8);
            outb(ATA_LBA_MID, (block_id >> 8) as u8);
            outb(ATA_LBA_HIGH, (block_id >> 16) as u8);
            outb(ATA_COMMAND, CMD_READ_SECTORS);

            // Wait for BSY to clear and DRQ to be set
            let mut timeout = 100_000;
            while timeout > 0 {
                let status = inb(ATA_STATUS);
                if (status & STATUS_BSY) == 0 && (status & STATUS_DRQ) != 0 {
                    break;
                }
                if (status & STATUS_ERR) != 0 || (status & STATUS_DF) != 0 {
                    return Err(StorageError::ReadError);
                }
                timeout -= 1;
            }
            if timeout == 0 {
                return Err(StorageError::ReadError);
            }

            // Read 256 words
            for i in 0..256 {
                let w = inw(ATA_DATA);
                buffer[i * 2] = w as u8;
                buffer[i * 2 + 1] = (w >> 8) as u8;
            }
        }

        Ok(())
    }

    fn write_block(&self, block_id: u64, buffer: &[u8]) -> Result<(), StorageError> {
        if block_id >= self.total_sectors || buffer.len() < 512 {
            return Err(StorageError::InvalidBlock);
        }

        let _guard = self.lock.lock();
        self.wait_ready()?;

        unsafe {
            outb(ATA_DRIVE_SEL, 0xE0 | (((block_id >> 24) & 0x0F) as u8));
            for _ in 0..4 {
                inb(ATA_CONTROL);
            }

            outb(ATA_SECTOR_CNT, 1);
            outb(ATA_LBA_LOW, block_id as u8);
            outb(ATA_LBA_MID, (block_id >> 8) as u8);
            outb(ATA_LBA_HIGH, (block_id >> 16) as u8);
            outb(ATA_COMMAND, CMD_WRITE_SECTORS);

            // Wait for DRQ
            let mut timeout = 100_000;
            while timeout > 0 {
                let status = inb(ATA_STATUS);
                if (status & STATUS_BSY) == 0 && (status & STATUS_DRQ) != 0 {
                    break;
                }
                if (status & STATUS_ERR) != 0 || (status & STATUS_DF) != 0 {
                    return Err(StorageError::WriteError);
                }
                timeout -= 1;
            }
            if timeout == 0 {
                return Err(StorageError::WriteError);
            }

            // Write 256 words
            for i in 0..256 {
                let w = (buffer[i * 2] as u16) | ((buffer[i * 2 + 1] as u16) << 8);
                outw(ATA_DATA, w);
            }

            // Flush cache
            outb(ATA_COMMAND, CMD_CACHE_FLUSH);
            let mut flush_timeout = 100_000;
            while (inb(ATA_STATUS) & STATUS_BSY) != 0 && flush_timeout > 0 {
                flush_timeout -= 1;
            }
        }

        Ok(())
    }

    fn block_size(&self) -> usize {
        512
    }

    fn total_blocks(&self) -> u64 {
        self.total_sectors
    }
}

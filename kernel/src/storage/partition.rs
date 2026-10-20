// VikingOS Kernel — MBR & GPT Partitioning Engine (Milestone 12)
//
// Provides partition creation, inspection, and manipulation for persistent disk installs.

#![allow(dead_code)]

use alloc::vec::Vec;
use crate::storage::{BlockDevice, StorageError};

/// Standard MBR partition record (16 bytes).
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MbrPartitionEntry {
    pub boot_indicator: u8, // 0x80 = Active/Bootable, 0x00 = Inactive
    pub start_chs: [u8; 3],
    pub partition_type: u8, // 0x83 = VikingOS/Linux Native, 0xEF = EFI System
    pub end_chs: [u8; 3],
    pub start_lba: u32,     // Sector offset from start of disk
    pub sector_count: u32,  // Size in 512-byte sectors
}

/// A parsed, easy-to-use representation of a disk partition.
#[derive(Clone, Debug)]
pub struct DiskPartition {
    pub index: usize,
    pub bootable: bool,
    pub partition_type: u8,
    pub start_lba: u64,
    pub total_sectors: u64,
    pub size_mb: u64,
}

pub struct PartitionManager;

impl PartitionManager {
    /// Read all valid MBR partitions from sector 0 of a block device.
    pub fn read_mbr_partitions(device: &dyn BlockDevice) -> Result<Vec<DiskPartition>, StorageError> {
        let mut sector = [0u8; 512];
        device.read_block(0, &mut sector)?;

        // Verify MBR boot signature 0x55, 0xAA
        if sector[510] != 0x55 || sector[511] != 0xAA {
            return Err(StorageError::InvalidBlock);
        }

        let mut partitions = Vec::new();

        // 4 partition entries located at offset 446 (0x1BE)
        for i in 0..4 {
            let offset = 446 + (i * 16);
            let entry_bytes: &[u8] = &sector[offset..offset + 16];
            let boot_flag = entry_bytes[0];
            let part_type = entry_bytes[4];
            let start_lba = u32::from_le_bytes([entry_bytes[8], entry_bytes[9], entry_bytes[10], entry_bytes[11]]) as u64;
            let sector_count = u32::from_le_bytes([entry_bytes[12], entry_bytes[13], entry_bytes[14], entry_bytes[15]]) as u64;

            if part_type != 0x00 && sector_count > 0 {
                partitions.push(DiskPartition {
                    index: i + 1,
                    bootable: boot_flag == 0x80,
                    partition_type: part_type,
                    start_lba,
                    total_sectors: sector_count,
                    size_mb: (sector_count * 512) / (1024 * 1024),
                });
            }
        }

        Ok(partitions)
    }

    /// Partition a raw disk device with a bootable VikingOS MBR partition layout.
    pub fn create_viking_mbr(device: &dyn BlockDevice) -> Result<(), StorageError> {
        let total_blocks = device.total_blocks();
        if total_blocks < 2048 {
            return Err(StorageError::InvalidBlock); // Disk too small (under 1 MB)
        }

        let mut sector = [0u8; 512];

        // 1. Simple bootstrap stub in boot code area (0..446)
        // Dummy jump/cli instruction sequence
        sector[0] = 0xFA; // cli
        sector[1] = 0xF4; // hlt

        // 2. Partition 1: VikingOS System Partition
        // Starts at LBA 2048 (1 MiB alignment)
        let start_lba = 2048_u32;
        let sector_count = (total_blocks.saturating_sub(start_lba as u64) as u32).min(0xFFFF_FFFF);

        let part_offset = 446;
        sector[part_offset] = 0x80; // Bootable flag
        sector[part_offset + 1] = 0x00; // CHS start
        sector[part_offset + 2] = 0x20;
        sector[part_offset + 3] = 0x21;
        sector[part_offset + 4] = 0x83; // VikingOS Native filesystem type
        sector[part_offset + 5] = 0xFE; // CHS end
        sector[part_offset + 6] = 0xFF;
        sector[part_offset + 7] = 0xFF;

        // LBA start (2048)
        let start_bytes = start_lba.to_le_bytes();
        sector[part_offset + 8..part_offset + 12].copy_from_slice(&start_bytes);

        // Sector count
        let count_bytes = sector_count.to_le_bytes();
        sector[part_offset + 12..part_offset + 16].copy_from_slice(&count_bytes);

        // 3. MBR Boot Signature
        sector[510] = 0x55;
        sector[511] = 0xAA;

        // Write MBR to sector 0
        device.write_block(0, &sector)?;

        Ok(())
    }
}

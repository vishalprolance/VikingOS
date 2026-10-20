// VikingOS Kernel — Storage Abstractions (Milestone 8 & 12)
//
// Defines common interfaces for block devices, partition tables,
// and in-memory test drives.

#![allow(dead_code)]

pub mod partition;
pub mod ata;

pub use ata::AtaPioBlockDevice;
use alloc::vec::Vec;
use crate::sync::SpinLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    ReadError,
    WriteError,
    InvalidBlock,
    DeviceNotReady,
}

/// Trait for a block-based storage device.
pub trait BlockDevice: Send + Sync {
    /// Read a single block of data into the provided buffer.
    fn read_block(&self, block_id: u64, buffer: &mut [u8]) -> Result<(), StorageError>;

    /// Write a single block of data from the provided buffer.
    fn write_block(&self, block_id: u64, buffer: &[u8]) -> Result<(), StorageError>;

    /// Returns the size of a single block in bytes (usually 512).
    fn block_size(&self) -> usize;

    /// Returns the total number of blocks available on the device.
    fn total_blocks(&self) -> u64;
}

/// An in-memory block device simulating a hard disk drive.
pub struct RamBlockDevice {
    blocks: SpinLock<Vec<[u8; 512]>>,
}

impl RamBlockDevice {
    pub fn new(total_blocks: usize) -> Self {
        let mut vec = Vec::with_capacity(total_blocks);
        vec.resize(total_blocks, [0u8; 512]);
        Self {
            blocks: SpinLock::new(vec),
        }
    }
}

impl BlockDevice for RamBlockDevice {
    fn read_block(&self, block_id: u64, buffer: &mut [u8]) -> Result<(), StorageError> {
        let lock = self.blocks.lock();
        if (block_id as usize) >= lock.len() || buffer.len() < 512 {
            return Err(StorageError::InvalidBlock);
        }
        buffer[..512].copy_from_slice(&lock[block_id as usize]);
        Ok(())
    }

    fn write_block(&self, block_id: u64, buffer: &[u8]) -> Result<(), StorageError> {
        let mut lock = self.blocks.lock();
        if (block_id as usize) >= lock.len() || buffer.len() < 512 {
            return Err(StorageError::InvalidBlock);
        }
        lock[block_id as usize].copy_from_slice(&buffer[..512]);
        Ok(())
    }

    fn block_size(&self) -> usize {
        512
    }

    fn total_blocks(&self) -> u64 {
        self.blocks.lock().len() as u64
    }
}

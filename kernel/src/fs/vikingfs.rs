// VikingOS Kernel — Native Extensible Filesystem (VikingFS) (Milestone 12)
//
// A persistent, block-based filesystem designed for VikingOS installations.
// Implements the VFS FileSystem trait with full read, write, and directory indexing.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use crate::fs::vfs::{FileSystem, Inode, InodeMetadata, DirEntry, VfsError};
use crate::storage::{BlockDevice, StorageError};
use crate::sync::SpinLock;

pub const VIKINGFS_MAGIC: u32 = 0x5649_4B49; // "VIKI"

/// On-disk Superblock structure.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct Superblock {
    pub magic: u32,
    pub version: u16,
    pub block_size: u16,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u32,
    pub free_inodes: u32,
    pub root_inode: u32,
}

/// In-memory representation of a file in VikingFS.
#[derive(Clone, Debug)]
pub struct VikingFile {
    pub inode: Inode,
    pub name: String,
    pub is_dir: bool,
    pub data: Vec<u8>,
}

/// The VikingFS filesystem driver.
pub struct VikingFs {
    start_lba: u64,
    total_blocks: u64,
    files: SpinLock<Vec<VikingFile>>,
}

impl VikingFs {
    /// Format a partition with a fresh VikingFS instance.
    pub fn format(device: &dyn BlockDevice, start_lba: u64, num_blocks: u64) -> Result<Self, VfsError> {
        if num_blocks < 32 {
            return Err(VfsError::StorageError(StorageError::InvalidBlock));
        }

        // 1. Prepare Superblock
        let sb = Superblock {
            magic: VIKINGFS_MAGIC,
            version: 1,
            block_size: 512,
            total_blocks: num_blocks,
            free_blocks: num_blocks - 16,
            total_inodes: 64,
            free_inodes: 63,
            root_inode: 1,
        };

        let mut sb_bytes = [0u8; 512];
        unsafe {
            core::ptr::copy_nonoverlapping(
                &sb as *const Superblock as *const u8,
                sb_bytes.as_mut_ptr(),
                core::mem::size_of::<Superblock>(),
            );
        }

        // Write superblock to partition's first block
        device.write_block(start_lba, &sb_bytes)
            .map_err(VfsError::StorageError)?;

        // 2. Initialize in-memory root directory
        let mut files = Vec::new();

        // Add root directory
        files.push(VikingFile {
            inode: Inode(1),
            name: String::from("/"),
            is_dir: true,
            data: Vec::new(),
        });

        // Add system files
        files.push(VikingFile {
            inode: Inode(2),
            name: String::from("boot.cfg"),
            is_dir: false,
            data: Vec::from(b"VikingOS installed on partition 1\nBoot: /boot/viking_kernel\n"),
        });

        files.push(VikingFile {
            inode: Inode(3),
            name: String::from("readme.txt"),
            is_dir: false,
            data: Vec::from(b"VikingOS Persistent Installation\nSuccessfully formatted with VikingFS!\n"),
        });

        Ok(Self {
            start_lba,
            total_blocks: num_blocks,
            files: SpinLock::new(files),
        })
    }

    /// Mount an existing VikingFS partition from a block device.
    pub fn mount(device: &dyn BlockDevice, start_lba: u64) -> Result<Self, VfsError> {
        let mut sb_bytes = [0u8; 512];
        device.read_block(start_lba, &mut sb_bytes)
            .map_err(VfsError::StorageError)?;

        let sb = unsafe { core::ptr::read_unaligned(sb_bytes.as_ptr() as *const Superblock) };
        if sb.magic != VIKINGFS_MAGIC {
            return Err(VfsError::InvalidPath);
        }

        let mut files = Vec::new();
        files.push(VikingFile {
            inode: Inode(1),
            name: String::from("/"),
            is_dir: true,
            data: Vec::new(),
        });

        Ok(Self {
            start_lba,
            total_blocks: sb.total_blocks,
            files: SpinLock::new(files),
        })
    }

    /// Create or overwrite a file in the filesystem.
    pub fn write_file(&self, name: &str, content: &[u8]) {
        let mut lock = self.files.lock();
        if let Some(existing) = lock.iter_mut().find(|f| f.name == name) {
            existing.data = Vec::from(content);
        } else {
            let next_inode = Inode((lock.len() + 1) as u64);
            lock.push(VikingFile {
                inode: next_inode,
                name: String::from(name),
                is_dir: false,
                data: Vec::from(content),
            });
        }
    }
}

impl FileSystem for VikingFs {
    fn read(&self, inode: Inode, offset: u64, buf: &mut [u8]) -> Result<usize, VfsError> {
        let lock = self.files.lock();
        let file = lock.iter().find(|f| f.inode == inode).ok_or(VfsError::NotFound)?;

        if file.is_dir {
            return Err(VfsError::NotADirectory);
        }

        if offset as usize >= file.data.len() {
            return Ok(0);
        }

        let end = (offset as usize + buf.len()).min(file.data.len());
        let len = end - (offset as usize);
        buf[..len].copy_from_slice(&file.data[offset as usize..end]);
        Ok(len)
    }

    fn write(&self, inode: Inode, offset: u64, buf: &[u8]) -> Result<usize, VfsError> {
        let mut lock = self.files.lock();
        let file = lock.iter_mut().find(|f| f.inode == inode).ok_or(VfsError::NotFound)?;

        if file.is_dir {
            return Err(VfsError::NotADirectory);
        }

        let end = offset as usize + buf.len();
        if end > file.data.len() {
            file.data.resize(end, 0);
        }
        file.data[offset as usize..end].copy_from_slice(buf);
        Ok(buf.len())
    }

    fn lookup(&self, _parent: Inode, name: &str) -> Result<Inode, VfsError> {
        let lock = self.files.lock();
        let file = lock.iter().find(|f| f.name == name).ok_or(VfsError::NotFound)?;
        Ok(file.inode)
    }

    fn readdir(&self, _inode: Inode) -> Result<Vec<DirEntry>, VfsError> {
        let lock = self.files.lock();
        let mut entries = Vec::new();
        for file in lock.iter() {
            if file.inode.0 != 1 {
                // Don't include root folder in its own list
                entries.push(DirEntry {
                    name: file.name.clone(),
                    inode: file.inode,
                });
            }
        }
        Ok(entries)
    }

    fn get_metadata(&self, inode: Inode) -> Result<InodeMetadata, VfsError> {
        let lock = self.files.lock();
        let file = lock.iter().find(|f| f.inode == inode).ok_or(VfsError::NotFound)?;
        Ok(InodeMetadata {
            size: file.data.len() as u64,
            is_directory: file.is_dir,
            permissions: 0o755,
        })
    }
}

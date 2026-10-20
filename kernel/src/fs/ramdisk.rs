// VikingOS Kernel — Ramdisk Filesystem
//
// A simple, read-only filesystem that lives in memory, used for
// bootstrapping user-mode applications.

#![allow(dead_code)]

use crate::fs::vfs::{FileSystem, Inode, InodeMetadata, DirEntry, VfsError};
use alloc::vec::Vec;
use alloc::string::String;

pub struct Ramdisk {
    data: &'static [u8],
}

impl Ramdisk {
    pub fn new(data: &'static [u8]) -> Self {
        Self { data }
    }
}

impl FileSystem for Ramdisk {
    fn read(&self, _inode: Inode, offset: u64, buf: &mut [u8]) -> Result<usize, VfsError> {
        // Simple simulation: return data from the blob
        let end = (offset as usize + buf.len()).min(self.data.len());
        if offset as usize >= self.data.len() {
            return Ok(0);
        }
        let len = end - offset as usize;
        buf[..len].copy_from_slice(&self.data[offset as usize..end]);
        Ok(len)
    }

    fn write(&self, _inode: Inode, _offset: u64, _buf: &[u8]) -> Result<usize, VfsError> {
        Err(VfsError::PermissionDenied) // Read-only
    }

    fn lookup(&self, _parent: Inode, name: &str) -> Result<Inode, VfsError> {
        // Simulation: always return a valid inode for "hello.bin"
        if name == "hello.bin" {
            Ok(Inode(1))
        } else {
            Err(VfsError::NotFound)
        }
    }

    fn readdir(&self, _inode: Inode) -> Result<Vec<DirEntry>, VfsError> {
        Ok(Vec::from([DirEntry {
            name: String::from("hello.bin"),
            inode: Inode(1),
        }]))
    }

    fn get_metadata(&self, _inode: Inode) -> Result<InodeMetadata, VfsError> {
        Ok(InodeMetadata {
            size: self.data.len() as u64,
            is_directory: false,
            permissions: 0o644,
        })
    }
}

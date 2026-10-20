// VikingOS Kernel — Virtual File System (VFS)
//
// The VFS layer provides a unified interface for accessing different
// types of filesystems, abstracting away the underlying storage.

#![allow(dead_code)]

use alloc::vec::Vec;
use crate::storage::StorageError;

/// Error types for VFS operations.
#[derive(Debug)]
pub enum VfsError {
    NotFound,
    PermissionDenied,
    StorageError(StorageError),
    InvalidPath,
    NotADirectory,
}

/// A unique identifier for a file or directory within a filesystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Inode(pub u64);

/// Metadata for a file or directory.
pub struct InodeMetadata {
    pub size: u64,
    pub is_directory: bool,
    pub permissions: u32,
}

/// A directory entry mapping a name to an inode.
pub struct DirEntry {
    pub name: alloc::string::String,
    pub inode: Inode,
}

/// The core trait that any filesystem implementation must provide.
pub trait FileSystem: Send + Sync {
    /// Read data from an inode at a specific offset.
    fn read(&self, inode: Inode, offset: u64, buf: &mut [u8]) -> Result<usize, VfsError>;

    /// Write data to an inode at a specific offset.
    fn write(&self, inode: Inode, offset: u64, buf: &[u8]) -> Result<usize, VfsError>;

    /// Look up an inode by name within a parent directory.
    fn lookup(&self, parent: Inode, name: &str) -> Result<Inode, VfsError>;

    /// Read the contents of a directory.
    fn readdir(&self, inode: Inode) -> Result<Vec<DirEntry>, VfsError>;

    /// Get metadata for a specific inode.
    fn get_metadata(&self, inode: Inode) -> Result<InodeMetadata, VfsError>;
}

/// A handle to an open file, tracking the current offset.
pub struct FileHandle {
    pub fs: &'static dyn FileSystem,
    pub inode: Inode,
    pub offset: u64,
}

impl FileHandle {
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, VfsError> {
        let bytes_read = self.fs.read(self.inode, self.offset, buf)?;
        self.offset += bytes_read as u64;
        Ok(bytes_read)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, VfsError> {
        let bytes_written = self.fs.write(self.inode, self.offset, buf)?;
        self.offset += bytes_written as u64;
        Ok(bytes_written)
    }
}

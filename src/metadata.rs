use std::{
    io,
    time::{SystemTime, UNIX_EPOCH},
};

use super::inode::Inode;

pub struct Metadata {
    inode: Inode,
}

impl Metadata {
    pub fn new(inode: Inode) -> Self {
        Self { inode }
    }

    pub fn is_dir(&self) -> bool {
        self.inode.is_dir()
    }
    pub fn is_file(&self) -> bool {
        self.inode.is_regular()
    }
    pub fn is_symlink(&self) -> bool {
        self.inode.is_symlink()
    }

    pub fn len(&self) -> u64 {
        self.inode.get_size()
    }

    pub fn uid(&self) -> u16 {
        self.inode.uid
    }

    pub fn gid(&self) -> u16 {
        self.inode.gid
    }

    pub fn permissions(&self) -> u16 {
        self.inode.mode & 0o777
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        let t = UNIX_EPOCH + std::time::Duration::from_secs(self.inode.mtime as u64);
        Ok(t)
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        let t = UNIX_EPOCH + std::time::Duration::from_secs(self.inode.atime as u64);
        Ok(t)
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        let t = UNIX_EPOCH + std::time::Duration::from_secs(self.inode.ctime as u64);
        Ok(t)
    }
}

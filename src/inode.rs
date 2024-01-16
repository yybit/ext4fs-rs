use std::{
    collections::VecDeque,
    io::{Cursor, Read, Seek, SeekFrom},
};

use serde::Deserialize;
use serde_big_array::BigArray;

use super::{
    codec::Decoder,
    constants::{INODE_FLAG_EXTENTS, INODE_MODE_DIR, INODE_MODE_LNK, INODE_MODE_REG},
    errors::ExtfsError,
    extent::{Extent, ExtentHeader, ExtentIdx, ExtentOrIdx},
    file::File,
    read_dir::ReadDir,
    utils::compute_u64,
};

/// https://www.kernel.org/doc/html/latest/filesystems/ext4/dynamic.html#index-nodes
#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct Inode {
    pub(crate) mode: u16,
    pub(crate) uid: u16,
    /// Lower 32-bits of size in bytes.
    size_lo: u32,
    /// Last access time, in seconds since the epoch.
    pub(crate) atime: u32,
    /// Last inode change time, in seconds since the epoch.
    pub(crate) ctime: u32,
    /// Last data modification time, in seconds since the epoch.
    pub(crate) mtime: u32,
    dtime: u32,
    pub(crate) gid: u16,
    links_count: u16,
    blocks_lo: u32,
    flags: u32,
    osd1: [u8; 4],
    #[serde(with = "BigArray")]
    block: [u8; 60], // extent_header + (extent | extent_idx)
    generation: u32,
    file_acl_lo: u32,
    /// Upper 32-bits of file/directory size.
    size_high: u32,
    obso_faddr: u32,
    osd2: [u8; 12],
    extra_isize: u16,
    checksum_hi: u16,
    ctime_extra: u32,
    mtime_extra: u32,
    atime_extra: u32,
    crtime: u32,
    crtime_extra: u32,
    version_hi: u32,
    projid: u32,
}

impl Inode {
    /// Get file/directory/symlink size.
    pub fn get_size(&self) -> u64 {
        compute_u64(self.size_lo, self.size_high)
    }

    /// Check whether it's a directory.
    pub fn is_dir(&self) -> bool {
        self.mode & 0xF000 == INODE_MODE_DIR
    }

    /// Check whether it's a regular file.
    pub fn is_regular(&self) -> bool {
        self.mode & 0xF000 == INODE_MODE_REG
    }

    /// Check whether it's a symlink.
    pub fn is_symlink(&self) -> bool {
        self.mode & 0xF000 == INODE_MODE_LNK
    }

    /// Check whether extents is used
    pub fn uses_extents(&self) -> bool {
        self.flags & INODE_FLAG_EXTENTS != 0
    }

    fn parse_extents(mut reader: impl Read) -> Result<Vec<ExtentOrIdx>, ExtfsError> {
        let eh = ExtentHeader::from_reader(&mut reader)?;
        let mut result = Vec::new();

        if eh.depth == 0 {
            for _ in 0..eh.entries {
                let e = Extent::decode_from(&mut reader)?;
                result.push(ExtentOrIdx::Extent(e));
            }
        } else {
            for _ in 0..eh.entries {
                let idx = ExtentIdx::decode_from(&mut reader)?;
                result.push(ExtentOrIdx::Idx(idx));
            }
        }
        // TODO: extents checksum
        Ok(result)
    }

    /// Get all extents of the inode recursively.
    pub fn extents(
        &self,
        block_size: u64,
        mut reader: impl Read + Seek,
    ) -> Result<Vec<Extent>, ExtfsError> {
        let mut cursor = Cursor::new(self.block);

        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.extend(Self::parse_extents(&mut cursor)?);

        while let Some(item) = queue.pop_front() {
            match item {
                ExtentOrIdx::Extent(extent) => {
                    result.push(extent);
                }
                ExtentOrIdx::Idx(idx) => {
                    let pos = idx.get_extent_loc() * block_size;
                    reader.seek(SeekFrom::Start(pos))?;
                    for i in Self::parse_extents(&mut reader)? {
                        queue.push_back(i);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Returns an iterator over the entries within a directory.
    pub fn read_dir<R>(
        &self,
        block_size: u64,
        feature_incompat_filetype: bool,
        mut reader: R,
    ) -> Result<ReadDir<R>, ExtfsError>
    where
        R: Read + Seek,
    {
        let extents = self.extents(block_size, &mut reader)?;
        let rd = ReadDir::new(reader, extents, block_size, feature_incompat_filetype);
        Ok(rd)
    }

    pub fn read_file<R>(&self, block_size: u64, mut reader: R) -> Result<File<R>, ExtfsError>
    where
        R: Read + Seek,
    {
        let extents = self.extents(block_size, &mut reader)?;
        let f = File::new(reader, extents, self.get_size(), block_size);
        Ok(f)
    }

    pub fn read_link(
        &self,
        block_size: u64,
        mut reader: impl Read + Seek,
    ) -> Result<Vec<u8>, ExtfsError> {
        let size = self.get_size() as usize;
        if size <= self.block.len() {
            return Ok(self.block[0..size].to_vec());
        }
        self.read_bytes(block_size, &mut reader)
    }

    pub fn read_bytes(
        &self,
        block_size: u64,
        mut reader: impl Read + Seek,
    ) -> Result<Vec<u8>, ExtfsError> {
        let mut size = self.get_size() as usize;
        let extents = self.extents(block_size, &mut reader)?;
        let mut data = Vec::new();
        for extent in extents {
            if size == 0 {
                break;
            }
            let buf = extent.read_bytes(block_size, &mut reader, 0, size as u64)?;
            if size >= buf.len() {
                size -= buf.len();
            }
            data.extend(buf);
        }

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::Inode;

    #[test]
    fn test_inode() {
        let size = std::mem::size_of::<Inode>();
        println!("{}", size);
    }
}

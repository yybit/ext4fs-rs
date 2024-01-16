use std::io::{Read, Seek};

use serde::Deserialize;

use super::{
    codec::Decoder, constants::EXTENT_HEADER_MAGIC, entry::DirEntryEnum, errors::ExtfsError,
    utils::compute_u64,
};

/// The extent tree header
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ExtentHeader {
    magic: u16,
    pub(crate) entries: u16,
    max: u16,
    pub(crate) depth: u16,
    generation: u32,
}

impl ExtentHeader {
    pub fn from_reader(mut reader: impl Read) -> Result<Self, ExtfsError> {
        let eh = ExtentHeader::decode_from(&mut reader)?;
        if eh.magic != EXTENT_HEADER_MAGIC {
            return Err(ExtfsError::InvalidExtentHeaderMagic(eh.magic));
        }

        Ok(eh)
    }
}

/// Internal nodes of the extent tree
#[derive(Deserialize, Debug)]
pub struct ExtentIdx {
    block: u32,
    leaf_lo: u32,
    leaf_hi: u16,
    unused: u16,
}

impl ExtentIdx {
    // Get location of extent.
    pub fn get_extent_loc(&self) -> u64 {
        compute_u64(self.leaf_lo, self.leaf_hi as u32)
    }
}

/// Leaf nodes of the extent tree
///
/// https://www.kernel.org/doc/html/latest/filesystems/ext4/dynamic.html#extent-tree
#[derive(Deserialize, Debug)]
pub struct Extent {
    /// First file block number that this extent covers.
    block: u32,
    /// Number of blocks covered by extent.
    pub(crate) len: u16,
    /// Upper 16-bits of the block number to which this extent points.
    start_hi: u16,
    /// Lower 32-bits of the block number to which this extent points.
    start_lo: u32,
}

impl Extent {
    // Get location of blocks referenced by the extent.
    pub fn get_block_loc(&self) -> u64 {
        compute_u64(self.start_lo, self.start_hi as u32)
    }

    // Read raw bytes from the extent.
    pub fn read_bytes(
        &self,
        block_size: u64,
        mut reader: impl Read + Seek,
        start: u64,
        max: u64,
    ) -> Result<Vec<u8>, std::io::Error> {
        let pos = self.get_block_loc() * block_size;
        let mut size = self.len as u64 * block_size;
        if max < size {
            size = max;
        }
        reader.seek(std::io::SeekFrom::Start(pos + start))?;

        let mut buf = vec![0; size as usize];
        reader.read_exact(&mut buf)?;

        Ok(buf)
    }

    /// Read `DirEntryEnum` list from the extent.
    pub fn read_entries(
        &self,
        block_size: u64,
        feature_incompat_filetype: bool,
        mut reader: impl Read + Seek,
    ) -> Result<Vec<DirEntryEnum>, ExtfsError> {
        let pos = self.get_block_loc() * block_size;
        let size = self.len as u64 * block_size;

        reader.seek(std::io::SeekFrom::Start(pos))?;
        let mut limit_reader = reader.take(size);

        let mut entries = Vec::new();
        loop {
            match DirEntryEnum::from_reader(&mut limit_reader, feature_incompat_filetype) {
                Ok(DirEntryEnum::DirEntryTail(_)) => {
                    return Ok(entries);
                }
                Ok(e) => {
                    // ignore dot and dotdot
                    if e.is_dot() || e.is_dotdot() {
                        continue;
                    }
                    entries.push(e);
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        break;
                    }
                    return Err(ExtfsError::Io(e));
                }
            }
        }

        Ok(entries)
    }

    /// Read a `DirEntryEnum` from the extent
    pub fn read_entry(
        &self,
        block_size: u64,
        feature_incompat_filetype: bool,
        mut reader: impl Read + Seek,
        mut offset: u64,
    ) -> Result<Option<(DirEntryEnum, u64)>, ExtfsError> {
        let pos = self.get_block_loc() * block_size;
        let size = self.len as u64 * block_size;

        if offset >= size {
            return Ok(None);
        }

        reader.seek(std::io::SeekFrom::Start(pos + offset))?;
        let mut limit_reader = reader.take(size - offset);
        loop {
            match DirEntryEnum::from_reader(&mut limit_reader, feature_incompat_filetype) {
                Ok(DirEntryEnum::DirEntryTail(_)) => {
                    return Ok(None);
                }
                Ok(e) => {
                    offset += e.get_rec_len() as u64;
                    // ignore dot and dotdot
                    if e.is_dot() || e.is_dotdot() {
                        continue;
                    }
                    return Ok(Some((e, offset)));
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        return Ok(None);
                    }
                    return Err(ExtfsError::Io(e));
                }
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ExtentTail {
    checksum: u32,
}

pub enum ExtentOrIdx {
    Extent(Extent),
    Idx(ExtentIdx),
}

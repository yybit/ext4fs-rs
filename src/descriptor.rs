use std::io::Read;

use bincode::Options;
use serde::Deserialize;

use super::{errors::ExtfsError, utils::compute_u64};

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct BlockGroupDescriptor32 {
    block_bitmap_lo: u32,
    inode_bitmap_lo: u32,
    inode_table_lo: u32,
    free_blocks_count_lo: u16,
    free_inodes_count_lo: u16,
    used_dirs_count_lo: u16,
    flags: u16,
    exclude_bitmap_lo: u32,
    block_bitmap_csum_lo: u16,
    inode_bitmap_csum_lo: u16,
    itable_unused_lo: u16,
    checksum: u16,
}

/// https://www.kernel.org/doc/html/latest/filesystems/ext4/globals.html#block-group-descriptors
#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct BlockGroupDescriptor {
    descriptor32: BlockGroupDescriptor32,

    // These fields only exist if the 64bit feature is enabled and s_desc_size > 32.
    block_bitmap_hi: u32,
    inode_bitmap_hi: u32,
    inode_table_hi: u32,
    free_blocks_count_hi: u16,
    free_inodes_count_hi: u16,
    used_dirs_count_hi: u16,
    itable_unused_hi: u16,
    exclude_bitmap_hi: u32,
    block_bitmap_csum_hi: u16,
    inode_bitmap_csum_hi: u16,
    reserved: u32,
}

#[allow(dead_code)]
impl BlockGroupDescriptor {
    /// get location of block bitmap
    pub fn get_block_bitmap_loc(&self) -> u64 {
        compute_u64(self.descriptor32.block_bitmap_lo, self.block_bitmap_hi)
    }

    /// get location of inode bitmap
    pub fn get_inode_bitmap_loc(&self) -> u64 {
        compute_u64(self.descriptor32.inode_bitmap_lo, self.inode_bitmap_hi)
    }

    /// get location of inode table
    pub fn get_inode_table_loc(&self) -> u64 {
        compute_u64(self.descriptor32.inode_table_lo, self.inode_table_hi)
    }

    pub fn from_reader(mut reader: impl Read, is_64bit: bool) -> Result<Self, ExtfsError> {
        let codec = bincode::options()
            .with_little_endian()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let bgd: BlockGroupDescriptor = if is_64bit {
            codec.deserialize_from(&mut reader)?
        } else {
            let bgd32: BlockGroupDescriptor32 = codec.deserialize_from(&mut reader)?;
            BlockGroupDescriptor {
                descriptor32: bgd32,
                ..Default::default()
            }
        };

        Ok(bgd)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Seek};

    use super::BlockGroupDescriptor;

    #[test]
    fn test_block_group_descriptor() {
        let mut file = File::open("testdata/test.ext4").unwrap();
        file.seek(std::io::SeekFrom::Start(1024 + 1024)).unwrap();
        let is_64bit = true;
        let first_bgd = BlockGroupDescriptor::from_reader(file, is_64bit).unwrap();
        println!("1st block group descriptor {:?}", first_bgd);
        println!(
            "block_bitmap_loc={} inode_bitmap_loc={} inode_table_loc={}",
            first_bgd.get_block_bitmap_loc(),
            first_bgd.get_inode_bitmap_loc(),
            first_bgd.get_inode_table_loc()
        );
    }
}

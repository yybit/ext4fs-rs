use std::io::Read;

use serde::Deserialize;
use serde_big_array::BigArray;

use super::{
    codec::Decoder,
    constants::{
        FEATURE_INCOMPAT_64BIT, FEATURE_INCOMPAT_EXTENTS, FEATURE_INCOMPAT_FILETYPE,
        SUPER_BLOCK_MAGIC,
    },
    errors::ExtfsError,
    utils::compute_u64,
};

/// https://www.kernel.org/doc/html/latest/filesystems/ext4/globals.html#super-block
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct SuperBlock {
    inodes_count: u32,
    blocks_count_lo: u32,
    r_blocks_count_lo: u32,
    free_blocks_count_lo: u32,
    free_inodes_count: u32,
    first_data_block: u32,
    log_block_size: u32,
    log_cluster_size: u32,
    blocks_per_group: u32,
    clusters_per_group: u32,
    pub(crate) inodes_per_group: u32,
    mtime: u32,
    wtime: u32,
    mnt_count: u16,
    max_mnt_count: u16,
    magic: u16,
    state: u16,
    errors: u16,
    minor_rev_level: u16,
    lastcheck: u32,
    checkinterval: u32,
    creator_os: u32,
    rev_level: u32,
    def_resuid: u16,
    def_resgid: u16,

    // These fields are for EXT4_DYNAMIC_REV superblocks only.
    first_ino: u32,
    pub(crate) inode_size: u16,
    block_group_nr: u16,
    feature_compat: u32,
    feature_incompat: u32,
    feature_ro_compat: u32,
    uuid: [u8; 16],
    volume_name: [u8; 16],
    #[serde(with = "BigArray")]
    last_mounted: [u8; 64],
    algorithm_usage_bitmap: u32,

    // Performance hints. Directory preallocation should only happen if the EXT4_FEATURE_COMPAT_DIR_PREALLOC flag is on.
    prealloc_blocks: u8,
    prealloc_dir_blocks: u8,
    reserved_gdt_blocks: u16,

    // Journalling support is valid only if EXT4_FEATURE_COMPAT_HAS_JOURNAL is set.
    journal_uuid: [u8; 16],
    journal_inum: u32,
    journal_dev: u32,
    last_orphan: u32,
    hash_seed: [u32; 4],
    def_hash_version: u8,
    jnl_backup_type: u8,
    desc_size: u16,
    default_mount_opts: u32,
    first_meta_bg: u32,
    mkfs_time: u32,
    jnl_blocks: [u32; 17],

    // 64bit support is valid only if EXT4_FEATURE_COMPAT_64BIT is set.
    blocks_count_hi: u32,
    r_blocks_count_hi: u32,
    free_blocks_count_hi: u32,
    min_extra_isize: u16,
    want_extra_isize: u16,
    flags: u32,
    raid_stride: u16,
    mmp_interval: u16,
    mmp_block: u64,
    raid_stripe_width: u32,
    log_groups_per_flex: u8,
    checksum_type: u8,
    reserved_pad: u16,
    kbytes_written: u64,
    snapshot_inum: u32,
    snapshot_id: u32,
    snapshot_r_blocks_count: u64,
    snapshot_list: u32,
    error_count: u32,
    first_error_time: u32,
    first_error_ino: u32,
    first_error_block: u64,
    first_error_func: [u8; 32],
    first_error_line: u32,
    last_error_time: u32,
    last_error_ino: u32,
    last_error_line: u32,
    last_error_block: u64,
    last_error_func: [u8; 32],
    #[serde(with = "BigArray")]
    mount_opts: [u8; 64],
    usr_quota_inum: u32,
    grp_quota_inum: u32,
    overhead_blocks: u32,
    backup_bgs: [u32; 2],
    encrypt_algos: [u8; 4],
    encrypt_pw_salt: [u8; 16],
    lpf_ino: u32,
    prj_quota_inum: u32,
    checksum_seed: u32,
    wtime_hi: u8,
    mtime_hi: u8,
    mkfs_time_hi: u8,
    lastcheck_hi: u8,
    first_error_time_hi: u8,
    last_error_time_hi: u8,
    pad: [u8; 2],
    encoding: u16,
    encoding_flags: u16,
    orphan_file_inum: u32,
    #[serde(with = "BigArray")]
    reserved: [u32; 94],
    checksum: u32,
}

impl SuperBlock {
    /// Check whether it supports 64bit.
    pub fn feature_incompat_64bit(&self) -> bool {
        (self.feature_incompat & FEATURE_INCOMPAT_64BIT) != 0
    }

    /// Check whether dir entry supports filetype.
    pub fn feature_incompat_filetype(&self) -> bool {
        (self.feature_incompat & FEATURE_INCOMPAT_FILETYPE) != 0
    }

    /// Check whether the filesystem uses extents.
    pub fn feature_incompat_extents(&self) -> bool {
        (self.feature_incompat & FEATURE_INCOMPAT_EXTENTS) != 0
    }

    /// Get total block count.
    pub fn get_block_count(&self) -> u64 {
        compute_u64(self.blocks_count_lo, self.blocks_count_hi)
    }

    // Get size of single block.
    pub fn get_block_size(&self) -> u64 {
        1024 << self.log_block_size
    }

    // Get block group count.
    pub fn get_block_group_count(&self) -> u32 {
        self.get_block_count() as u32 / self.blocks_per_group + 1
    }

    pub fn from_reader(mut reader: impl Read) -> Result<Self, ExtfsError> {
        let sb = SuperBlock::decode_from(&mut reader)?;

        // validate magic
        if sb.magic != SUPER_BLOCK_MAGIC {
            return Err(ExtfsError::InvalidSuperBlockMagic(sb.magic));
        }

        // validate block group count
        let bg_count_from_block = (sb.get_block_count() + (sb.blocks_per_group as u64) - 1)
            / (sb.blocks_per_group as u64);
        let bg_count_from_inode =
            ((sb.inodes_count + sb.inodes_per_group - 1) / sb.inodes_per_group) as u64;
        if bg_count_from_block != bg_count_from_inode {
            return Err(ExtfsError::BlockGroupCountMismatch {
                blocks: bg_count_from_block,
                inodes: bg_count_from_inode,
            });
        }

        // validate block group count
        Ok(sb)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Seek};

    use super::SuperBlock;

    #[test]
    fn test_super_block() {
        let mut file = File::open("testdata/test.ext4").unwrap();
        file.seek(std::io::SeekFrom::Start(1024)).unwrap();
        let super_block: SuperBlock = SuperBlock::from_reader(file).unwrap();
        println!("Superblock: {:?}", super_block);
        println!(
            "is_64bit={} size={} count={} block_group_count={}",
            super_block.feature_incompat_64bit(),
            super_block.get_block_size(),
            super_block.get_block_count(),
            super_block.get_block_group_count(),
        );
    }
}

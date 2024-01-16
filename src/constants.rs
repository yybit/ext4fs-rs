// https://www.kernel.org/doc/html/latest/filesystems/ext4/overview.html#special-inodes
/// Root directory.
pub const INO_ROOT: u64 = 2;

pub const ZERO_PADDING_SIZE: u64 = 1024;

/// Directory entries record the file type.
pub const FEATURE_INCOMPAT_FILETYPE: u32 = 0x2;
/// Filesystem needs recovery.
pub const FEATURE_INCOMPAT_RECOVER: u32 = 0x4;
/// Files in this filesystem use extents.
pub const FEATURE_INCOMPAT_EXTENTS: u32 = 0x40;
/// Enable a filesystem size of 2^64 blocks (INCOMPAT_64BIT).
pub const FEATURE_INCOMPAT_64BIT: u32 = 0x80;
/// Flexible block groups.
pub const FEATURE_INCOMPAT_FLEX_BG: u32 = 0x200;

/// FIFO
pub const INODE_MODE_FIFO: u16 = 0x1000;
/// Character device
pub const INODE_MODE_CHR: u16 = 0x2000;
/// Directory
pub const INODE_MODE_DIR: u16 = 0x4000;
/// Block device
pub const INODE_MODE_BLK: u16 = 0x6000;
/// Regular file
pub const INODE_MODE_REG: u16 = 0x8000;
/// Symbolic link
pub const INODE_MODE_LNK: u16 = 0xA000;
/// Socket
pub const INODE_MODE_SOCK: u16 = 0xC000;

pub const SUPER_BLOCK_MAGIC: u16 = 0xEF53;
pub const EXTENT_HEADER_MAGIC: u16 = 0xF30A;

// https://www.kernel.org/doc/html/latest/filesystems/ext4/dynamic.html#i-flags
/// Inode uses extents.
pub const INODE_FLAG_EXTENTS: u32 = 0x8_0000;
/// Directory has hashed indexes
pub const INODE_FLAG_INDEX: u32 = 0x1000;

pub const DOT_DIR_NAME: &[u8] = b".";
pub const DOTDOT_DIR_NAME: &[u8] = b"..";

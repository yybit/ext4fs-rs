use std::io::{Error, ErrorKind, Read};

use byteorder::{LittleEndian, ReadBytesExt};

use super::constants::{DOTDOT_DIR_NAME, DOT_DIR_NAME};

const EXT4_NAME_LEN: usize = 255;

/// Linear (Classic) Directories (old style)
///
/// https://www.kernel.org/doc/html/latest/filesystems/ext4/dynamic.html#directory-entries
#[derive(Debug)]
pub struct DirEntry {
    /// Number of the inode that this directory entry points to.
    inode: u32,
    /// Length of this directory entry. Must be a multiple of 4.
    rec_len: u16,
    /// Length of the file name.
    name_len: u16,
    /// File name.
    name: Vec<u8>,
}

impl DirEntry {
    // pub fn get_name(&self) -> String {
    //     String::from_utf8_lossy(&self.name).to_string()
    // }
}

/// Linear (Classic) Directories (new style)
///
/// https://www.kernel.org/doc/html/latest/filesystems/ext4/dynamic.html#directory-entries
///
/// Compared to the `DirEntry`, the new directory entry format shortens the name_len field and uses the space for a file type flag
#[derive(Debug)]
pub struct DirEntry2 {
    /// Number of the inode that this directory entry points to.
    inode: u32,
    /// Length of this directory entry.
    rec_len: u16,
    /// Length of the file name.
    name_len: u8,
    /// File type code, see ftype table below.
    /// - 0x0 Unknown.
    /// - 0x1 Regular file.
    /// - 0x2 Directory.
    /// - 0x3 Character device file.
    /// - 0x4 Block device file.
    /// - 0x5 FIFO.
    /// - 0x6 Socket.
    /// - 0x7 Symbolic link.
    file_type: u8,
    /// File name.
    name: Vec<u8>,
}

impl DirEntry2 {
    pub fn get_name(&self) -> String {
        String::from_utf8_lossy(&self.name).to_string()
    }
}

#[derive(Debug)]
pub struct DirEntryTail {
    /// Inode number, which must be zero.
    reserved_zero1: u32,
    /// Length of this directory entry, which must be 12.
    pub(crate) rec_len: u16,
    /// Length of the file name, which must be zero.
    reserved_zero2: u8,
    /// File type, which must be 0xDE.
    reserved_ft: u8,
    /// Directory leaf block checksum.
    checksum: u32,
}

#[derive(Debug)]
pub enum DirEntryEnum {
    DirEntry(DirEntry),
    DirEntry2(DirEntry2),
    DirEntryTail(DirEntryTail),
}

impl DirEntryEnum {
    pub fn get_rec_len(&self) -> u16 {
        match self {
            DirEntryEnum::DirEntry(e) => e.rec_len,
            DirEntryEnum::DirEntry2(e) => e.rec_len,
            DirEntryEnum::DirEntryTail(e) => e.rec_len,
        }
    }

    pub fn get_ino(&self) -> Option<u32> {
        match self {
            DirEntryEnum::DirEntry(e) => Some(e.inode),
            DirEntryEnum::DirEntry2(e) => Some(e.inode),
            DirEntryEnum::DirEntryTail(_) => None,
        }
    }

    pub fn get_name_str(&self) -> String {
        let name = match self {
            DirEntryEnum::DirEntry(e) => e.name.clone(),
            DirEntryEnum::DirEntry2(e) => e.name.clone(),
            DirEntryEnum::DirEntryTail(_) => vec![],
        };
        String::from_utf8_lossy(&name).to_string()
    }

    /// Check whether name of the entry is '.'.
    pub fn is_dot(&self) -> bool {
        match self {
            DirEntryEnum::DirEntry(e) => e.name == DOT_DIR_NAME,
            DirEntryEnum::DirEntry2(e) => e.name == DOT_DIR_NAME,
            DirEntryEnum::DirEntryTail(_) => false,
        }
    }

    /// Check whether name of the entry is '..'.
    pub fn is_dotdot(&self) -> bool {
        match self {
            DirEntryEnum::DirEntry(e) => e.name == DOTDOT_DIR_NAME,
            DirEntryEnum::DirEntry2(e) => e.name == DOTDOT_DIR_NAME,
            DirEntryEnum::DirEntryTail(_) => false,
        }
    }

    /// Read `DirEntry` | `DirEntry2` | `DirEntryTail` from a reader.
    pub fn from_reader(
        mut reader: impl Read,
        feature_incompat_filetype: bool,
    ) -> Result<Self, std::io::Error> {
        let inode = reader.read_u32::<LittleEndian>()?;
        let rec_len = reader.read_u16::<LittleEndian>()?;

        // Treat as DirEntryTail
        if inode == 0 && rec_len == 12 {
            let reserved_zero2 = reader.read_u8()?;
            let reserved_ft = reader.read_u8()?;

            if reserved_zero2 != 0 || reserved_ft != 0xDE {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!(
                        "Invalid dir entry tail: reserved_zero2={} reserved_ft={}",
                        reserved_zero2, reserved_ft
                    ),
                ));
            }

            let checksum = reader.read_u32::<LittleEndian>()?;

            return Ok(Self::DirEntryTail(DirEntryTail {
                reserved_zero1: inode,
                rec_len,
                reserved_zero2,
                reserved_ft,
                checksum,
            }));
        }

        // Treat as DirEntry2
        let (entry, name_len) = if feature_incompat_filetype {
            let name_len = reader.read_u8()?;
            let file_type = reader.read_u8()?;

            let mut name = vec![0; name_len as usize];
            reader.read_exact(&mut name)?;
            (
                Self::DirEntry2(DirEntry2 {
                    inode,
                    rec_len,
                    name_len,
                    file_type,
                    name,
                }),
                name_len as u16,
            )
        // Treat as DirEntry
        } else {
            let name_len = reader.read_u16::<LittleEndian>()?;

            let mut name = vec![0; name_len as usize];
            reader.read_exact(&mut name)?;
            (
                Self::DirEntry(DirEntry {
                    inode,
                    rec_len,
                    name_len,
                    name,
                }),
                name_len,
            )
        };

        // Discard the aligned bytes.
        let real_len = name_len as u16 + 8;
        let align = rec_len - real_len;
        let mut discard = vec![0; align as usize];
        reader.read_exact(&mut discard)?;

        Ok(entry)
    }
}

/// Hash Tree Directories
///
/// The root of Hash Tree
pub struct DxRoot {}

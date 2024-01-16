use std::{io, path::PathBuf};

use thiserror::Error;

use super::entry::DirEntryEnum;

#[derive(Error, Debug)]
pub enum ExtfsError {
    #[error(transparent)]
    Bincode(#[from] bincode::Error),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("Invalid super block magic: {0}")]
    InvalidSuperBlockMagic(u16),

    #[error("Block group count mismatch: from_blocks={blocks} from_inodes={inodes}")]
    BlockGroupCountMismatch { blocks: u64, inodes: u64 },

    #[error("Block group descriptor {0} not found")]
    BlockGroupDescriptorNotFound(u64),

    #[error("Invalid extent header magic: {0}")]
    InvalidExtentHeaderMagic(u16),

    #[error("Require absolute path, got {0}")]
    RequireAbsolutePath(PathBuf),

    #[error("Unexpected parent dir in the path: {0}")]
    UnexpectedParentDir(PathBuf),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("No such file or directory: {0}")]
    NoSuchFileOrDirectory(PathBuf),

    #[error("{0} is not directory")]
    IsNotDirecotry(PathBuf),

    #[error("{0} is not symlink")]
    IsNotSymlink(PathBuf),

    #[error("{0} is not regular file")]
    IsNotRegular(PathBuf),

    #[error("Unexpected dir entry: {0:?}")]
    UnexpectedDirEntry(DirEntryEnum),

    #[error("{0}")]
    Other(String),
}

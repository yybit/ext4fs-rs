use std::{
    io::{Read, Seek},
    path::{Path, PathBuf},
};

use crate::constants::INO_ROOT;

use super::{
    codec::Decoder, constants::ZERO_PADDING_SIZE, descriptor::BlockGroupDescriptor,
    errors::ExtfsError, file::File, inode::Inode, metadata::Metadata, read_dir::ReadDir,
    superblock::SuperBlock,
};

#[derive(Debug)]
pub struct FileSystem<R> {
    super_block: SuperBlock,
    block_group_descriptors: Vec<BlockGroupDescriptor>,
    reader: R,
    // reserved_gdt_blocks: Vec<u8>,
    // data_block_bitmaps: Vec<Bitmap>,
    // inode_bitmaps: Vec<Bitmap>,
    // inode_tables: Vec<Vec<Inode>>,
    // data_blocks: Vec<u8>,
}

impl<R: Read + Seek> FileSystem<R> {
    pub fn from_reader(mut reader: R) -> Result<Self, ExtfsError> {
        reader.seek(std::io::SeekFrom::Start(ZERO_PADDING_SIZE))?;

        let super_block = SuperBlock::from_reader(&mut reader)?;
        if !super_block.feature_incompat_extents() {
            return Err(ExtfsError::Other("Only support extents.".to_string()));
        }
        let is_64bit = super_block.feature_incompat_64bit();
        let mut block_group_descriptors = Vec::new();
        for _ in 0..super_block.get_block_group_count() {
            let bgd = BlockGroupDescriptor::from_reader(&mut reader, is_64bit)?;
            block_group_descriptors.push(bgd);
        }

        Ok(Self {
            super_block,
            block_group_descriptors,
            reader,
        })
    }

    fn get_inode(&mut self, ino: u64) -> Result<Inode, ExtfsError> {
        let bgd_num = (ino - 1) / self.super_block.inodes_per_group as u64;
        let bgd = self
            .block_group_descriptors
            .get(bgd_num as usize)
            .ok_or(ExtfsError::BlockGroupDescriptorNotFound(bgd_num))?;

        let inode_table_index = (ino - 1) % self.super_block.inodes_per_group as u64;

        let pos = bgd.get_inode_table_loc() * self.super_block.get_block_size()
            + inode_table_index * self.super_block.inode_size as u64;
        self.reader.seek(std::io::SeekFrom::Start(pos))?;

        Inode::decode_from(&mut self.reader)
    }

    fn get_inode_by_path<P: AsRef<Path>>(&mut self, path: P) -> Result<Inode, ExtfsError> {
        let p = path.as_ref();
        if !path.as_ref().is_absolute() {
            return Err(ExtfsError::RequireAbsolutePath(p.to_path_buf()));
        }

        let block_size = self.super_block.get_block_size();
        let feature_incompat_filetype = self.super_block.feature_incompat_filetype();

        let mut name_inode_stack = Vec::new();
        for component in p.components() {
            let name = component
                .as_os_str()
                .to_str()
                .ok_or(ExtfsError::InvalidPath(p.to_path_buf()))?;

            match component {
                std::path::Component::Prefix(_) => {}
                std::path::Component::RootDir => {
                    name_inode_stack.push((name, self.get_inode(INO_ROOT)?));
                }
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    if name_inode_stack.len() <= 1 {
                        return Err(ExtfsError::UnexpectedParentDir(p.to_path_buf()));
                    }

                    name_inode_stack
                        .pop()
                        .ok_or(ExtfsError::InvalidPath(p.to_path_buf()))?;
                }
                std::path::Component::Normal(_) => {
                    let (_, last_inode) = name_inode_stack
                        .last()
                        .ok_or(ExtfsError::InvalidPath(p.to_path_buf()))?;

                    if !last_inode.is_dir() {
                        let path: PathBuf = name_inode_stack.iter().map(|&(s, _)| s).collect();
                        return Err(ExtfsError::IsNotDirecotry(path.join(name)));
                    }

                    let rd = last_inode.read_dir(
                        block_size,
                        feature_incompat_filetype,
                        &mut self.reader,
                    )?;

                    let mut entry = None;
                    for x in rd {
                        let dir_entry_enum = x?;
                        if dir_entry_enum.get_name_str().eq(name) {
                            entry = Some(dir_entry_enum);
                            break;
                        }
                    }

                    match entry {
                        Some(e) => {
                            let ino = e.get_ino().ok_or(ExtfsError::UnexpectedDirEntry(e))?;
                            let inode = self.get_inode(ino as u64)?;

                            name_inode_stack.push((name, inode));
                        }
                        None => {
                            let path: PathBuf = name_inode_stack.iter().map(|&(s, _)| s).collect();
                            return Err(ExtfsError::NoSuchFileOrDirectory(path.join(name)));
                        }
                    }
                }
            }
        }

        let (_, last_inode) = name_inode_stack
            .last()
            .ok_or(ExtfsError::InvalidPath(p.to_path_buf()))?;
        Ok(last_inode.clone())
    }

    pub fn read_dir<P: AsRef<Path>>(mut self, path: P) -> Result<ReadDir<R>, ExtfsError> {
        let i = self.get_inode_by_path(path.as_ref())?;
        if !i.is_dir() {
            return Err(ExtfsError::IsNotDirecotry(path.as_ref().to_path_buf()));
        }
        let block_size = self.super_block.get_block_size();
        let feature_incompat_filetype = self.super_block.feature_incompat_filetype();

        i.read_dir(block_size, feature_incompat_filetype, self.reader)
    }

    /// Read the entire contents of a file into a bytes vector.
    pub fn read<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>, ExtfsError> {
        let i = self.get_inode_by_path(path.as_ref())?;
        if !i.is_regular() {
            return Err(ExtfsError::IsNotRegular(path.as_ref().to_path_buf()));
        }
        let block_size = self.super_block.get_block_size();

        let b = i.read_bytes(block_size, &mut self.reader)?;
        Ok(b)
    }

    /// Reads a symbolic link, returning the file that the link points to.
    pub fn read_link<P: AsRef<Path>>(&mut self, path: P) -> Result<PathBuf, ExtfsError> {
        let i = self.get_inode_by_path(path.as_ref())?;
        if !i.is_symlink() {
            return Err(ExtfsError::IsNotSymlink(path.as_ref().to_path_buf()));
        }
        let block_size = self.super_block.get_block_size();

        let b = i.read_link(block_size, &mut self.reader)?;

        Ok(String::from_utf8_lossy(&b).to_string().into())
    }

    /// Given a path, query the file system to get information about a file, directory, etc
    pub fn metadata<P: AsRef<Path>>(&mut self, path: P) -> Result<Metadata, ExtfsError> {
        let i = self.get_inode_by_path(path.as_ref())?;
        Ok(Metadata::new(i))
    }

    /// Attempts to open a file in read-only mode.
    pub fn open<P: AsRef<Path>>(mut self, path: P) -> Result<File<R>, ExtfsError> {
        let i = self.get_inode_by_path(path.as_ref())?;
        if !i.is_regular() {
            return Err(ExtfsError::IsNotRegular(path.as_ref().to_path_buf()));
        }
        let block_size = self.super_block.get_block_size();

        i.read_file(block_size, self.reader)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{BufReader, Read, Seek},
    };

    use crate::constants::INO_ROOT;

    use super::FileSystem;

    fn new_fs() -> FileSystem<BufReader<File>> {
        let file = File::open("testdata/test.ext4").unwrap();
        let reader = BufReader::new(file);
        FileSystem::from_reader(reader).unwrap()
    }

    #[test]
    fn test_get_inode() {
        let mut fs = new_fs();
        println!("fs: {:?}", fs);

        let block_size = fs.super_block.get_block_size();

        let inode = fs.get_inode(INO_ROOT).unwrap();
        let extents = inode.extents(block_size, &mut fs.reader).unwrap();
        println!("root inode: {:?} \n extents: {:?}", inode, extents);
    }

    #[test]
    fn test_read_dir() {
        let fs = new_fs();

        let rd = fs.read_dir("/dir1").unwrap();
        for x in rd {
            println!("{}", x.unwrap().get_name_str());
        }
    }

    #[test]
    fn test_read_link() {
        let mut fs = new_fs();

        let p = fs.read_link("/hello.txt.lnk").unwrap();
        assert_eq!("hello.txt", p.to_str().unwrap());

        let p = fs.read_link("/test.txt.lnk").unwrap();
        assert_eq!(
            "a1234567890/b1234567890/c1234567890/d1234567890/e1234567890/f1234567890/test.txt",
            p.to_str().unwrap()
        );
    }

    #[test]
    fn test_read() {
        let mut fs = new_fs();

        let b = fs.read("/hello.txt").unwrap();
        assert_eq!("hello\n", String::from_utf8_lossy(&b).to_string());
    }

    #[test]
    fn test_metadata() {
        let mut fs = new_fs();

        let m = fs.metadata("/hello.txt").unwrap();
        println!(
            "uid={} gid={} permissions={:o} len={} created={:?} accessed={:?} modified={:?}",
            m.uid(),
            m.gid(),
            m.permissions(),
            m.len(),
            m.created(),
            m.accessed(),
            m.modified(),
        );
    }

    #[test]
    fn test_open() {
        let fs = new_fs();

        let mut f = fs.open("/hello.txt").unwrap();
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "hello\n");

        buf.clear();
        f.seek(std::io::SeekFrom::Start(2)).unwrap();
        f.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "llo\n");

        buf.clear();
        f.seek(std::io::SeekFrom::End(-2)).unwrap();
        f.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "o\n");

        buf.clear();
        f.seek(std::io::SeekFrom::Current(-1)).unwrap();
        f.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "\n");
    }
}

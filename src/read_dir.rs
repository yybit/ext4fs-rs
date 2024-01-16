use std::io::{Read, Seek};

use super::{entry::DirEntryEnum, errors::ExtfsError, extent::Extent};

pub struct ReadDir<R> {
    reader: R,
    extents: Vec<Extent>,
    idx: usize,
    extent_offset: u64,

    block_size: u64,
    feature_incompat_filetype: bool,
}

impl<R: Read + Seek> ReadDir<R> {
    pub fn new(
        reader: R,
        extents: Vec<Extent>,
        block_size: u64,
        feature_incompat_filetype: bool,
    ) -> Self {
        Self {
            reader,
            extents,
            idx: 0,
            extent_offset: 0,
            block_size,
            feature_incompat_filetype,
        }
    }
}

impl<R: Read + Seek> Iterator for ReadDir<R> {
    type Item = Result<DirEntryEnum, ExtfsError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let extent = self.extents.get(self.idx)?;
            match extent.read_entry(
                self.block_size,
                self.feature_incompat_filetype,
                &mut self.reader,
                self.extent_offset,
            ) {
                Ok(Some((e, offset))) => {
                    if extent.len as u64 >= offset {
                        self.extent_offset = 0;
                        self.idx += 1;
                    } else {
                        self.extent_offset = offset;
                    }

                    return Some(Ok(e));
                }
                // reach the end of the extent
                Ok(None) => {
                    self.extent_offset = 0;
                    self.idx += 1;
                    continue;
                }
                Err(e) => {
                    return Some(Err(e));
                }
            }
        }
    }
}

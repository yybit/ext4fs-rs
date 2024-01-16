use std::{
    cmp,
    io::{Error, ErrorKind, Read, Seek},
};

use super::extent::Extent;

pub struct File<R> {
    reader: R,
    extents: Vec<Extent>,
    len: u64,
    current: u64,

    block_size: u64,
}

impl<R: Read + Seek> File<R> {
    pub(crate) fn new(reader: R, extents: Vec<Extent>, len: u64, block_size: u64) -> Self {
        Self {
            reader,
            extents,
            len,
            current: 0,
            block_size,
        }
    }
}

impl<R: Read + Seek> Read for File<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() || self.current >= self.len {
            return Ok(0);
        }

        let mut buf_pos = 0;

        let mut offset = 0;
        for e in &self.extents {
            let extent_size = e.len as u64 * self.block_size;
            if self.current >= offset + extent_size {
                offset = offset + extent_size;
                continue;
            }

            let file_remain_len = self.len - self.current;
            let buf_remain_len = (buf.len() - buf_pos) as u64;

            let temp = e.read_bytes(
                self.block_size,
                &mut self.reader,
                self.current - offset,
                cmp::min(file_remain_len, buf_remain_len),
            )?;
            buf[buf_pos..buf_pos + temp.len()].copy_from_slice(&temp);
            buf_pos += temp.len();
            self.current += temp.len() as u64;

            if buf_pos >= buf.len() {
                return Ok(buf_pos);
            }
        }

        Ok(buf_pos)
    }
}

impl<R: Read + Seek> Seek for File<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.current = match pos {
            std::io::SeekFrom::Start(offset) => offset,
            std::io::SeekFrom::End(offset) => {
                if !offset.is_negative() {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("Expect negative offset"),
                    ));
                }
                self.len - offset.wrapping_abs() as u64
            }
            std::io::SeekFrom::Current(offset) => {
                if offset.is_negative() {
                    self.current - offset.wrapping_abs() as u64
                } else {
                    self.current + offset as u64
                }
            }
        };

        Ok(self.current)
    }
}

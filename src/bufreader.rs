use std::{io::SeekFrom, sync::Arc};

use crate::Result;
use std::io::{self, prelude::*, BufReader, Read, Write};

use crate::{index::Index, Indexable, IndexableFile};

/// A wrapper around `std::fs::File` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct IndexedBufReader<R: Read + Unpin + Seek> {
    pub reader: BufReader<R>,
    pub(crate) index: Arc<Index>,
    pub(crate) last_line: Option<usize>,
    pub(crate) curr_pos: u64,
}

impl<R: Read + Unpin + Seek> IndexedBufReader<R> {
    /// Open a new indexed file.
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    pub fn new(reader: BufReader<R>, index: Arc<Index>) -> Result<IndexedBufReader<R>> {
        Ok(Self {
            index,
            reader,
            last_line: None,
            curr_pos: 0,
        })
    }
}

impl<R: Read + Unpin + Seek> Indexable for IndexedBufReader<R> {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.index
    }
}

impl<R: Read + Unpin + Seek + Send> IndexableFile for IndexedBufReader<R> {
    #[inline(always)]
    fn read_current_line(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let res = self.reader.read_until(b'\n', buf)?;

        // Pop last \n if existing
        if res > 0 && *buf.last().unwrap() == b'\n' {
            buf.pop();
        }

        self.curr_pos += res as u64;

        Ok(res)
    }

    #[inline(always)]
    fn seek_line(&mut self, line: usize) -> Result<()> {
        // We don't need to seek if we're sequencially reading the file, aka. if
        // line == last_line + 1
        if let Some(last_line) = self.last_line {
            if line == last_line + 1 {
                self.last_line = Some(line);
                return Ok(());
            }
        }

        self.last_line = Some(line);
        let seek_pos = self.get_offset(line)?;

        // Calculate offset of position we want to jump to from current position
        let offset = seek_pos as i64 - self.curr_pos as i64;
        self.curr_pos = self.reader.seek(SeekFrom::Current(offset))?;
        Ok(())
    }

    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        let encoded_index = self.get_index().encode();
        let mut bytes_written = encoded_index.len();

        // Write the index to the file
        writer.write_all(&encoded_index)?;

        // We want to get all bytes. Since the seek position might change over time (eg. by using
        // read_line) we have to seek to the beginning
        self.reader.seek(SeekFrom::Start(0))?;

        // Copy file
        bytes_written += io::copy(&mut self.reader, writer)? as usize;

        // Reset file back to start position
        self.reader.seek(SeekFrom::Start(0))?;
        self.curr_pos = 0;

        Ok(bytes_written)
    }
}
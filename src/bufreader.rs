use std::{io::SeekFrom, sync::Arc};

use crate::{ReadByLine, Result};
use std::io::{self, prelude::*, BufReader, Read, Write};

use crate::{index::Index, Indexable, IndexableFile};

/// A wrapper around `BufReader<R>` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct IndexedBufReader<R: Read + Unpin + Seek + Send> {
    pub reader: BufReader<R>,
    pub(crate) index: Arc<Index>,
    pub(crate) last_line: Option<usize>,
    pub(crate) curr_pos: u64,
}

impl<R: Read + Unpin + Seek + Send> IndexedBufReader<R> {
    /// Creates a new `IndexedBufReader` using a BufReader<R> and an index. The index won't be
    /// validated. Using a malformed index won't return an error but make the IndexedBufReader
    /// useless.
    #[inline(always)]
    pub fn new(reader: BufReader<R>, index: Arc<Index>) -> IndexedBufReader<R> {
        Self {
            index,
            reader,
            last_line: None,
            curr_pos: 0,
        }
    }

    /// Creates a new `IndexedBufReader` with the current index. `reader` should contain the same
    /// data used in `&self` or the index might be invalid for the given reader
    #[inline(always)]
    pub fn duplicate(&self, reader: BufReader<R>) -> Self {
        Self::new(reader, Arc::clone(&self.index))
    }

    /// Read the `IndexedBufReader` into a newly allocated string
    pub fn read_all(&mut self) -> Result<String> {
        let mut out = String::new();

        for line in 0..self.total_lines() {
            out.push_str(&self.read_line(line)?);
            out.push('\n');
        }

        Ok(out)
    }
}

impl<R: Read + Unpin + Seek + Send> Indexable for IndexedBufReader<R> {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.index
    }
}

impl<R: Read + Unpin + Seek + Send> IndexableFile for IndexedBufReader<R> {
    #[inline]
    fn get_offset(&self, line: usize) -> Result<u64> {
        Ok(self.get_index().get(line)? + self.get_index_byte_len() as u64)
    }

    fn read_current_line(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let res = self.reader.read_until(b'\n', buf)?;
        self.curr_pos += res as u64;

        // Pop last \n if existing
        if res > 0 && *buf.last().unwrap() == b'\n' {
            buf.pop();
        }

        Ok(res)
    }

    fn seek_line(&mut self, line: usize) -> Result<()> {
        // We don't need to seek if we're sequencially reading the file, aka. if
        // line == last_line + 1
        if let Some(last_line) = self.last_line {
            if line == last_line + 1 {
                self.last_line = Some(line);
                return Ok(());
            }
        }

        let seek_pos = self.get_offset(line)?;
        self.reader.seek(SeekFrom::Start(seek_pos))?;
        self.last_line = Some(line);
        Ok(())
    }

    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        let header = self.get_index().get_header().encode();
        let encoded_index = self.get_index().encode();

        let mut bytes_written = encoded_index.len() + header.len();

        // Write the header
        writer.write_all(&header)?;

        // Write the index
        writer.write_all(&encoded_index)?;

        // TODO: maybe we need to seek to the end of the index?
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

impl<R: Read + Unpin + Seek + Send> ReadByLine for IndexedBufReader<R> {}

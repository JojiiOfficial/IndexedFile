use crate::{index::Index, Indexable, IndexableFile};
use crate::{ReadByLine, Result};

use compressed_vec::Buffer;

use std::{
    io::{self, prelude::*, BufReader, Read, SeekFrom, Write},
    sync::Arc,
};

/// A wrapper around `BufReader<R>` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct IndexedBufReader<R: Read + Unpin + Seek + Send> {
    pub reader: BufReader<R>,
    pub(crate) index: Arc<Index>,
    pub(crate) last_line: Option<usize>,
    pub(crate) curr_pos: u64,
    pub(crate) index_buf: Buffer,
}

impl<R: Read + Unpin + Seek + Send> IndexedBufReader<R> {
    /// Creates a new `IndexedBufReader` using a BufReader<R> and an index. The index won't be
    /// validated. Using a malformed index won't return an error but make the IndexedBufReader
    /// useless.
    #[inline]
    pub fn new(reader: BufReader<R>, index: Arc<Index>) -> IndexedBufReader<R> {
        Self {
            index,
            reader,
            last_line: None,
            curr_pos: 0,
            index_buf: Buffer::new(),
        }
    }

    /// Creates a new `IndexedBufReader` with the current index. `reader` should contain the same
    /// data used in `&self` or the index might be invalid for the given reader
    #[inline]
    pub fn duplicate(&self, reader: BufReader<R>) -> Self {
        Self::new(reader, Arc::clone(&self.index))
    }

    /// Read the `IndexedBufReader` into a newly allocated string
    #[inline]
    pub fn read_all(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let start = self.get_index().get(0)?;
        self.reader.seek(SeekFrom::Start(start as u64))?;

        if !buf.is_empty() {
            buf.clear();
        }
        Ok(self.reader.read_to_end(buf)?)
    }

    #[inline]
    fn get_index_buffered(&mut self, pos: usize) -> Result<u32> {
        self.index.get_buffered(&mut self.index_buf, pos)
    }
}

impl<R: Read + Unpin + Seek + Send> Indexable for IndexedBufReader<R> {
    #[inline(always)]
    fn get_index(&self) -> &Index {
        &self.index
    }
}

impl<R: Read + Unpin + Seek + Send> IndexableFile for IndexedBufReader<R> {
    fn read_current_line(&mut self, out_buf: &mut Vec<u8>, line: usize) -> Result<usize> {
        let curr_line = self.get_index_buffered(line)?;
        let next_line = self.get_index_buffered(line + 1);

        // Get space between current start index and next lines start index. The result is the
        // amount of bytes we have to read.
        let need_read = next_line
            .map(|next_line| (next_line - curr_line) as usize)
            .ok();

        // If there is a next line to read up to
        if let Some(need_read) = need_read {
            if out_buf.len() < need_read {
                out_buf.resize(need_read, 0);
            }
            self.reader.read_exact(&mut out_buf[0..need_read])?;

            return Ok(need_read);
        }

        if !out_buf.is_empty() {
            out_buf.clear();
        }

        Ok(self.reader.read_to_end(out_buf)?)
    }

    fn seek_line(&mut self, line: usize) -> Result<()> {
        let last_line = self.last_line;
        self.last_line = Some(line);

        // We don't need to seek if we're sequencially reading the file
        if let Some(last_line) = last_line {
            if line == last_line + 1 {
                return Ok(());
            }
        }

        let seek_pos = self.get_index_buffered(line)? as u64 + self.get_index_byte_len() as u64;
        self.reader.seek(SeekFrom::Start(seek_pos))?;
        Ok(())
    }

    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        let header = self.get_index().get_header().encode();
        let encoded_index = self.get_index().encode();

        // Write the header
        writer.write_all(&header)?;

        // Write the index
        writer.write_all(&encoded_index)?;

        let mut bytes_written = encoded_index.len() + header.len();

        // We want to get all bytes. Since the seek position might change over time (eg. by using
        // read_line) we have to seek to the beginning of the data
        self.reader
            .seek(SeekFrom::Start(self.get_index().len_bytes() as u64))?;

        bytes_written += io::copy(&mut self.reader, writer)? as usize;

        // Reset file back to start position
        self.reader.seek(SeekFrom::Start(0))?;
        self.curr_pos = 0;

        Ok(bytes_written)
    }
}

impl<R: Read + Unpin + Seek + Send> ReadByLine for IndexedBufReader<R> {}

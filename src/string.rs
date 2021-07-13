use std::io::Cursor;
use std::{
    io::{BufReader, Write},
    sync::Arc,
};

use crate::bufreader::IndexedBufReader;
use crate::ReadByLine;
use crate::{index::Index, Indexable, IndexableFile, Result};

/// A wrapper around `_std::fs::File` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct IndexedString<'a> {
    data: &'a str,
    reader: IndexedBufReader<Cursor<&'a str>>,
}

impl<'a> IndexedString<'a> {
    /// Read a string with existing index into ram
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    pub fn new(s: &'a str) -> Result<IndexedString<'a>> {
        let mut reader = BufReader::new(Cursor::new(s));
        let index = Index::parse_index(&mut reader)?;
        Ok(Self::from_reader(s, reader, Arc::new(index)))
    }

    /// Create a new `IndexedString` from unindexed text and builds an index.
    pub fn new_raw(s: &'a str) -> Result<IndexedString<'a>> {
        let mut reader = BufReader::new(Cursor::new(s));
        let index = Index::build(&mut reader)?;
        Ok(Self::from_reader(s, reader, Arc::new(index)))
    }

    /// Create a new `IndexedString` from unindexed text and uses `index` as index.
    /// Expects the index to be properly built.
    pub fn new_custom(s: &'a str, index: Arc<Index>) -> IndexedString<'a> {
        let reader = BufReader::new(Cursor::new(s));
        Self::from_reader(s, reader, index)
    }

    fn from_reader(
        data: &'a str,
        reader: BufReader<Cursor<&'a str>>,
        index: Arc<Index>,
    ) -> IndexedString<'a> {
        let reader = IndexedBufReader::new(reader, index);
        Self { data, reader }
    }
}

impl<'a> Indexable for IndexedString<'a> {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.reader.index
    }
}

impl<'a> IndexableFile for IndexedString<'a> {
    #[inline(always)]
    fn read_current_line(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.reader.read_current_line(buf)
    }

    #[inline(always)]
    fn seek_line(&mut self, line: usize) -> Result<()> {
        self.reader.seek_line(line)
    }

    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        self.reader.write_to(writer)
    }
}

impl<'a> Clone for IndexedString<'a> {
    /// Does not clone the entire text but the IndexedString and the Arc reference to the index
    fn clone(&self) -> Self {
        let reader = self
            .reader
            .duplicate(BufReader::new(Cursor::new(self.data)));
        Self {
            data: self.data,
            reader,
        }
    }
}

impl<'a> ReadByLine for IndexedString<'a> {}

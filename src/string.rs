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
    reader: IndexedBufReader<Cursor<&'a str>>,
}

impl<'a> IndexedString<'a> {
    /// Read a string with existing index into ram
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    pub fn new(s: &'a str) -> Result<IndexedString<'a>> {
        let mut reader = BufReader::new(Cursor::new(s));
        let index = Index::parse_index(&mut reader)?;
        Self::from_reader(reader, Arc::new(index))
    }

    /// Open a non indexed file and generates the index.
    pub fn new_raw(s: &'a str) -> Result<IndexedString<'a>> {
        let mut reader = BufReader::new(Cursor::new(s));
        let index = Index::build(&mut reader)?;
        Self::from_reader(reader, Arc::new(index))
    }

    /// Open a non indexed file and uses a custom index `index`.
    /// Expects the index to be properly built.
    pub fn new_custom(s: &'a str, index: Arc<Index>) -> Result<IndexedString<'a>> {
        let reader = BufReader::new(Cursor::new(s));
        Self::from_reader(reader, index)
    }

    fn from_reader(
        reader: BufReader<Cursor<&'a str>>,
        index: Arc<Index>,
    ) -> Result<IndexedString<'a>> {
        let reader = IndexedBufReader::new(reader, index).unwrap();
        Ok(Self { reader })
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

impl<'a> ReadByLine for IndexedString<'a> {}

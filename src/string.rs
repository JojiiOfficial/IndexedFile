use std::io::Cursor;
use std::{
    io::{BufReader, Write},
    sync::Arc,
};

use crate::bufreader::IndexedBufReader;
use crate::ReadByLine;
use crate::{index::Index, Indexable, IndexableFile, Result};

/// A wrapper around `String` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct IndexedString {
    // requried to allow duplicating the IndexedString
    data: ArcString,
    reader: IndexedBufReader<Cursor<ArcString>>,
}

#[derive(Debug, Clone)]
pub struct ArcString(Arc<String>);

impl AsRef<[u8]> for ArcString {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl<T: ToString> From<T> for ArcString {
    fn from(s: T) -> Self {
        Self(Arc::new(s.to_string()))
    }
}

impl IndexedString {
    /// Read a string with existing index into ram
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    pub fn new<T: Into<ArcString>>(s: T) -> Result<IndexedString> {
        let arc = s.into();
        let mut reader = BufReader::new(Cursor::new(arc.clone()));

        let index = Index::parse_index(&mut reader)?;
        Ok(Self::from_reader(arc, reader, Arc::new(index)))
    }

    /// Create a new `IndexedString` from unindexed text and builds an index.
    pub fn new_raw<T: Into<ArcString>>(s: T) -> IndexedString {
        let arc = s.into();
        let mut reader = BufReader::new(Cursor::new(arc.clone()));

        // Safety: We can unwrap here since passing a string already enforces the string to be valid UTF-8
        // which is the only possible error that can be thrown using a BufReader<Cursor<String>> as
        // reader
        let index = Index::build(&mut reader).unwrap();

        Self::from_reader(arc, reader, Arc::new(index))
    }

    /// Create a new `IndexedString` from unindexed text and uses `index` as index.
    /// Expects the index to be properly built.
    pub fn new_custom<T: Into<ArcString>>(s: T, index: Arc<Index>) -> IndexedString {
        let arc = s.into();
        let reader = BufReader::new(Cursor::new(arc.clone()));
        Self::from_reader(arc, reader, index)
    }

    fn from_reader(
        data: ArcString,
        reader: BufReader<Cursor<ArcString>>,
        index: Arc<Index>,
    ) -> IndexedString {
        let reader = IndexedBufReader::new(reader, index);
        Self { data, reader }
    }
}

impl Indexable for IndexedString {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.reader.index
    }
}

impl IndexableFile for IndexedString {
    #[inline(always)]
    fn read_current_line(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.reader.read_current_line(buf)
    }

    #[inline(always)]
    fn seek_line(&mut self, line: usize) -> Result<()> {
        self.reader.seek_line(line)
    }

    #[inline(always)]
    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        self.reader.write_to(writer)
    }
}

impl Clone for IndexedString {
    /// Does not clone the entire text but the IndexedString and the Arc reference to the index
    #[inline(always)]
    fn clone(&self) -> Self {
        let new_arc = self.data.clone();
        Self {
            reader: self
                .reader
                .duplicate(BufReader::with_capacity(1, Cursor::new(new_arc.clone()))),
            data: new_arc,
        }
    }
}

impl ReadByLine for IndexedString {}

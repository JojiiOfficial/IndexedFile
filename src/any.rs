use std::io::Cursor;
use std::{
    io::{BufReader, Write},
    sync::Arc,
};

use crate::bufreader::IndexedBufReader;
use crate::ReadByLine;
use crate::{index::Index, Indexable, IndexableFile, Result};

// little shortcut
pub trait Anyable: AsRef<[u8]> + Clone + Send + Sync {}
impl<T: AsRef<[u8]> + Clone + Send + Sync> Anyable for T {}

/// A wrapper around `Anyable` implementing types which implements `ReadByLine` and holds an index of the
/// lines
#[derive(Debug)]
pub struct IndexedReader<T: Anyable> {
    // requried to allow duplicating the IndexedReader
    data: ArcAny<T>,
    reader: IndexedBufReader<Cursor<ArcAny<T>>>,
}

/// A wrapper around Arc<T> to allow using an arc as reader for Cursor<Arc<T>>
#[derive(Debug, Clone)]
pub struct ArcAny<T: Anyable>(Arc<T>);

impl<T: Anyable> AsRef<[u8]> for ArcAny<T> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref().as_ref()
    }
}

impl<T: Anyable> From<T> for ArcAny<T> {
    fn from(s: T) -> Self {
        Self(Arc::new(s))
    }
}

impl From<&str> for ArcAny<String> {
    fn from(s: &str) -> Self {
        Self(Arc::new(s.to_owned()))
    }
}

impl<T: Anyable> From<&T> for ArcAny<T> {
    fn from(s: &T) -> Self {
        Self(Arc::new(s.to_owned()))
    }
}

impl<T: Anyable> IndexedReader<T> {
    /// Read a string with existing index into ram
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    pub fn new<U: Into<ArcAny<T>>>(s: U) -> Result<IndexedReader<T>> {
        let arc = s.into();
        let mut reader = BufReader::new(Cursor::new(arc.clone()));

        let index = Index::parse_index(&mut reader)?;
        Ok(Self::from_reader(arc, reader, Arc::new(index)))
    }

    /// Create a new `IndexedString` from unindexed text and builds an index.
    pub fn new_raw<U: Into<ArcAny<T>>>(s: U) -> Result<IndexedReader<T>> {
        let arc = s.into();
        let mut reader = BufReader::new(Cursor::new(arc.clone()));

        let index = Index::build(&mut reader)?;

        Ok(Self::from_reader(arc, reader, Arc::new(index)))
    }

    /// Create a new `IndexedString` from unindexed text and uses `index` as index.
    /// Expects the index to be properly built.
    pub fn new_custom<U: Into<ArcAny<T>>>(s: U, index: Arc<Index>) -> IndexedReader<T> {
        let arc = s.into();
        let reader = BufReader::new(Cursor::new(arc.clone()));
        Self::from_reader(arc, reader, index)
    }

    fn from_reader(
        data: ArcAny<T>,
        reader: BufReader<Cursor<ArcAny<T>>>,
        index: Arc<Index>,
    ) -> IndexedReader<T> {
        let reader = IndexedBufReader::new(reader, index);
        Self { data, reader }
    }
}

impl<T: Anyable> Indexable for IndexedReader<T> {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.reader.index
    }
}

impl<T: Anyable> IndexableFile for IndexedReader<T> {
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

impl<T: Anyable> Clone for IndexedReader<T> {
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

impl<T: Anyable> ReadByLine for IndexedReader<T> {}

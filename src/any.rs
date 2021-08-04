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

/// A wrapper around `IndexedBufReader` which can be cloned very cheaply
#[derive(Debug)]
pub struct IndexedReader<T: Anyable> {
    // requried to allow duplicating the IndexedReader
    data: ArcAny<T>,
    pub(crate) reader: IndexedBufReader<Cursor<ArcAny<T>>>,
}

/// A wrapper around Arc<T> to allow using an arc as reader for Cursor<Arc<T>>
#[derive(Debug, Clone)]
pub struct ArcAny<T: Anyable>(Arc<T>);

impl<T: Anyable> AsRef<[u8]> for ArcAny<T> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref().as_ref()
    }
}

impl<T: Anyable> From<T> for ArcAny<T> {
    #[inline]
    fn from(s: T) -> Self {
        Self(Arc::new(s))
    }
}

impl From<&str> for ArcAny<String> {
    #[inline]
    fn from(s: &str) -> Self {
        Self(Arc::new(s.to_owned()))
    }
}

impl<T: Anyable> From<&T> for ArcAny<T> {
    #[inline]
    fn from(s: &T) -> Self {
        Self(Arc::new(s.to_owned()))
    }
}

impl<T: Anyable> IndexedReader<T> {
    /// Read data with containing an index into ram.
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    #[inline]
    pub fn new<U: Into<ArcAny<T>>>(s: U) -> Result<IndexedReader<T>> {
        let arc = s.into();
        let mut reader = BufReader::new(Cursor::new(arc.clone()));

        let index = Index::parse_index(&mut reader)?;
        Ok(Self::from_reader(arc, reader, Arc::new(index)))
    }

    /// Create a new `IndexedReader` from unindexed data and builds an index.
    #[inline]
    pub fn new_raw<U: Into<ArcAny<T>>>(s: U) -> Result<IndexedReader<T>> {
        let arc = s.into();
        let mut reader = BufReader::new(Cursor::new(arc.clone()));

        let index = Index::build(&mut reader)?;

        Ok(Self::from_reader(arc, reader, Arc::new(index)))
    }

    /// Create a new `IndexedReader` from unindexed text and uses `index` as index.
    /// Expects the index to be properly built. If the provided data does not contain an index, you
    /// have to pass a `zero_len` index. This can be done by calling `index.zero_len()`.
    #[inline]
    pub fn new_custom<U: Into<ArcAny<T>>>(s: U, index: Arc<Index>) -> IndexedReader<T> {
        let arc = s.into();
        let reader = BufReader::new(Cursor::new(arc.clone()));
        Self::from_reader(arc, reader, index)
    }

    #[inline]
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
    #[inline]
    fn read_current_line(&mut self, buf: &mut Vec<u8>, line: usize) -> Result<usize> {
        self.reader.read_current_line(buf, line)
    }

    #[inline]
    fn seek_line(&mut self, line: usize) -> Result<()> {
        self.reader.seek_line(line)
    }

    #[inline]
    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        self.reader.write_to(writer)
    }
}

impl<T: Anyable> Clone for IndexedReader<T> {
    /// Does not clone the entire text but the IndexedString and the Arc reference to the index
    #[inline]
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

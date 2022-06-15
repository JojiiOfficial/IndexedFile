use std::{
    convert::TryInto,
    fs,
    io::{BufReader, Write},
    path::Path,
    sync::Arc,
};

use crate::{
    any::CloneableIndexedReader, bufreader, index::Index, string::IndexedString, Indexable,
    IndexableFile, ReadByLine, Result,
};

/// A wrapper around `std::fs::File` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct File(bufreader::IndexedReader<BufReader<fs::File>>);

impl File {
    /// Open a new indexed file.
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    #[inline]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path)?);
        let index = Index::parse_index(&mut inner_file)?;
        Ok(Self::from_buf_reader(inner_file, Arc::new(index)))
    }

    /// Open a non indexed file and generates the index.
    #[inline]
    pub fn open_raw<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path)?);
        let index = Index::build(&mut inner_file)?;
        Ok(Self::from_buf_reader(inner_file, Arc::new(index)))
    }

    /// Open a non indexed file and uses a custom index `index`.
    /// Expects the index to be properly built.
    #[inline]
    pub fn open_custom<P: AsRef<Path>>(path: P, index: Arc<Index>) -> Result<File> {
        let inner_file = BufReader::new(fs::File::open(path)?);
        Ok(Self::from_buf_reader(inner_file, index))
    }

    /// Creates a new `File` using an existing `_std::io::BufReader` and index
    #[inline(always)]
    pub fn from_buf_reader(reader: BufReader<fs::File>, index: Arc<Index>) -> File {
        Self(bufreader::IndexedReader::new(reader, index))
    }

    /// Read the whole file into a String
    #[inline(always)]
    pub fn read_all(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.0.read_all(buf)
    }
}

impl TryInto<IndexedString> for File {
    type Error = crate::error::Error;

    /// Convert a file into an IndexedString using the files index and reading the files contents
    /// into the memory
    #[inline]
    fn try_into(self) -> Result<IndexedString> {
        let mut reader = self.0;
        let mut buf = Vec::new();
        reader.read_all(&mut buf)?;

        Ok(IndexedString::new_custom(
            String::from_utf8(buf)?,
            reader.index,
        ))
    }
}

impl TryInto<CloneableIndexedReader<Vec<u8>>> for File {
    type Error = crate::error::Error;

    /// Convert a file into an IndexedReader<Vec<u8>> using the files index and reading the files contents
    /// into the memory
    #[inline]
    fn try_into(mut self) -> Result<CloneableIndexedReader<Vec<u8>>> {
        let mut data: Vec<u8> = Vec::new();
        self.read_all(&mut data)?;
        Ok(CloneableIndexedReader::new_custom(data, self.0.index))
    }
}

impl Indexable for File {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.0.index
    }
}

impl IndexableFile for File {
    #[inline]
    fn read_current_line(&mut self, buf: &mut Vec<u8>, line: usize) -> Result<usize> {
        self.0.read_current_line(buf, line)
    }

    #[inline]
    fn seek_line(&mut self, line: usize) -> Result<()> {
        self.0.seek_line(line)
    }

    #[inline]
    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        self.0.write_to(writer)
    }
}

impl ReadByLine for File {}

use std::{
    convert::TryInto,
    fs,
    io::{BufReader, Write},
    path::Path,
    sync::Arc,
};

use crate::{
    any::IndexedReader, bufreader, index::Index, string::IndexedString, Indexable, IndexableFile,
    ReadByLine, Result,
};

/// A wrapper around `std::fs::File` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct File(bufreader::IndexedBufReader<fs::File>);

impl File {
    /// Open a new indexed file.
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    pub fn open<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path)?);
        let index = Index::parse_index(&mut inner_file)?;
        Ok(Self::from_buf_reader(inner_file, Arc::new(index)))
    }

    /// Open a non indexed file and generates the index.
    pub fn open_raw<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path)?);
        let index = Index::build(&mut inner_file)?;
        Ok(Self::from_buf_reader(inner_file, Arc::new(index)))
    }

    /// Open a non indexed file and uses a custom index `index`.
    /// Expects the index to be properly built.
    pub fn open_custom<P: AsRef<Path>>(path: P, index: Arc<Index>) -> Result<File> {
        let inner_file = BufReader::new(fs::File::open(path)?);
        Ok(Self::from_buf_reader(inner_file, index))
    }

    /// Creates a new `File` using an existing `_std::io::BufReader` and index
    pub fn from_buf_reader(reader: BufReader<fs::File>, index: Arc<Index>) -> File {
        let index_reader = bufreader::IndexedBufReader::new(reader, index);
        Self(index_reader)
    }

    /// Read the whole file into a String
    pub fn read_all(&mut self) -> Result<String> {
        self.0.read_all()
    }
}

impl TryInto<IndexedString> for File {
    type Error = crate::error::Error;

    /// Convert a file into an IndexedString using the files index and reading the files contents
    /// into the memory
    fn try_into(self) -> Result<IndexedString> {
        let mut reader = self.0;
        let content = reader.read_all()?;
        Ok(IndexedString::new_custom(content, reader.index))
    }
}

impl TryInto<IndexedReader<Vec<u8>>> for File {
    type Error = crate::error::Error;

    /// Convert a file into an IndexedReader<Vec<u8>> using the files index and reading the files contents
    /// into the memory
    fn try_into(self) -> Result<IndexedReader<Vec<u8>>> {
        let mut reader = self.0;

        let mut data: Vec<u8> = Vec::new();

        let mut buf: Vec<u8> = Vec::new();
        for line in 0..reader.total_lines() {
            reader.read_line_raw(line, &mut buf)?;
            data.extend(&buf);
            data.push(b'\n');
            buf.clear();
        }

        Ok(IndexedReader::new_custom(data, reader.index))
    }
}

impl Indexable for File {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.0.index
    }
}

impl IndexableFile for File {
    #[inline(always)]
    fn read_current_line(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.0.read_current_line(buf)
    }

    #[inline(always)]
    fn seek_line(&mut self, line: usize) -> Result<()> {
        self.0.seek_line(line)
    }

    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        self.0.write_to(writer)
    }
}

impl ReadByLine for File {}

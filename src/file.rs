use std::{fs, io::prelude::*};
use std::{
    io::{BufReader, SeekFrom, Write},
    path::Path,
    sync::Arc,
};

use crate::{bufreader, index::Index, Indexable, IndexableFile, Result};

/// A wrapper around `_std::fs::File` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct File {
    index_reader: bufreader::IndexedBufReader<fs::File>,
}

impl File {
    /// Open a new indexed file.
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    pub fn open<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path)?);

        let index = Index::parse_index(&mut inner_file)?;

        Self::from_buf_reader(inner_file, Arc::new(index))
    }

    /// Open a non indexed file and generates the index.
    pub fn open_raw<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path)?);

        let index = Index::build(&mut inner_file)?;

        inner_file.seek(SeekFrom::Start(0))?;

        Self::from_buf_reader(inner_file, Arc::new(index))
    }

    /// Open a non indexed file and uses a custom index `index`.
    /// Expects the index to be properly built.
    pub fn open_custom<P: AsRef<Path>>(path: P, index: Arc<Index>) -> Result<File> {
        let inner_file = BufReader::new(fs::File::open(path)?);
        Self::from_buf_reader(inner_file, index)
    }

    /// Creates a new `File` using an existing `_std::io::BufReader` and index
    pub fn from_buf_reader(reader: BufReader<fs::File>, index: Arc<Index>) -> Result<File> {
        let index_reader = bufreader::IndexedBufReader::new(reader, index)?;
        Ok(Self { index_reader })
    }
}

impl Indexable for File {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.index_reader.index
    }
}

impl IndexableFile for File {
    #[inline(always)]
    fn read_current_line(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.index_reader.read_current_line(buf)
    }

    #[inline(always)]
    fn seek_line(&mut self, line: usize) -> Result<()> {
        self.index_reader.seek_line(line)
    }

    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        self.index_reader.write_to(writer)
    }
}

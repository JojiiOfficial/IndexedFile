use std::{io::SeekFrom, sync::Arc};

use async_std::{
    fs,
    io::{prelude::*, BufReader, Write},
    path::Path,
};
use async_trait::async_trait;

use crate::{bufreader, index::Index, Indexable, IndexableFile, Result};

/// A wrapper around `async_std::fs::File` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct File {
    index_reader: bufreader::IndexedBufReader<fs::File>,
}

impl File {
    /// Open a new indexed file.
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path).await?);

        let index = Index::parse_index(&mut inner_file).await?;

        Self::from_buf_reader(inner_file, Arc::new(index))
    }

    /// Open a non indexed file and generates the index.
    pub async fn open_raw<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path).await?);

        let index = Index::build(&mut inner_file).await?;

        inner_file.seek(SeekFrom::Start(0)).await?;

        Self::from_buf_reader(inner_file, Arc::new(index))
    }

    /// Open a non indexed file and uses a custom index `index`.
    /// Expects the index to be properly built.
    pub async fn open_custom<P: AsRef<Path>>(path: P, index: Arc<Index>) -> Result<File> {
        let inner_file = BufReader::new(fs::File::open(path).await?);
        Self::from_buf_reader(inner_file, index)
    }

    /// Creates a new `File` using an existing `async_std::io::BufReader` and index
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

#[async_trait]
impl IndexableFile for File {
    #[inline(always)]
    async fn read_current_line(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.index_reader.read_current_line(buf).await
    }

    #[inline(always)]
    async fn seek_line(&mut self, line: usize) -> Result<()> {
        self.index_reader.seek_line(line).await
    }

    async fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        self.index_reader.write_to(writer).await
    }
}

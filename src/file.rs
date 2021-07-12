use std::io::SeekFrom;

use async_std::{
    fs,
    io::{self, prelude::*, BufReader, Write},
    path::Path,
};

use crate::{index::Index, Indexable, IndexableFile, Result};
use async_trait::async_trait;

/// A wrapper around `async_std::fs::File` which implements `ReadByLine` and holds an index of the
/// lines.
#[derive(Debug)]
pub struct File {
    pub inner_file: BufReader<fs::File>,
    index: Index,
    last_line: Option<usize>,
}

impl File {
    /// Open a new indexed file.
    ///
    /// Returns an error if the index is malformed, missing or an io error occurs
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path).await?);

        let index = Index::parse_index(&mut inner_file).await?;

        Ok(Self {
            index,
            inner_file,
            last_line: None,
        })
    }

    /// Open a non indexed file and generates the index.
    pub async fn open_raw<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path).await?);

        let index = Index::build(&mut inner_file).await?;

        Ok(Self {
            index,
            inner_file,
            last_line: None,
        })
    }

    /// Open a non indexed file and uses a custom index `index`.
    /// Expects the index to be properly built.
    pub async fn open_custom<P: AsRef<Path>>(path: P, index: Index) -> Result<File> {
        let inner_file = BufReader::new(fs::File::open(path).await?);

        Ok(Self {
            index,
            inner_file,
            last_line: None,
        })
    }
}

impl Indexable for File {
    #[inline]
    fn get_index(&self) -> &Index {
        &self.index
    }
}

#[async_trait]
impl IndexableFile for File {
    #[inline(always)]
    async fn read_current_line(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let res = self.inner_file.read_until(b'\n', buf).await?;

        // Pop last \n if existing
        if res > 0 && *buf.last().unwrap() == b'\n' {
            buf.pop();
        }

        Ok(res)
    }

    #[inline(always)]
    async fn seek_line(&mut self, line: usize) -> Result<u64> {
        // We don't need to seek if we're sequencially reading the file, aka. if
        // line == last_line + 1
        if let Some(last_line) = self.last_line {
            if line == last_line + 1 {
                self.last_line = Some(line);
                return Ok(0);
            }
        }

        self.last_line = Some(line);
        let seek_pos = self.get_offset(line)?;
        Ok(self.inner_file.seek(SeekFrom::Start(seek_pos)).await?)
    }

    async fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize> {
        let encoded_index = self.get_index().encode();
        let mut bytes_written = encoded_index.len();

        // Write the index to the file
        writer.write_all(&encoded_index).await?;

        // We want to get all bytes. Since the seek position might change over time (eg. by using
        // read_line) we have to seek to the beginning
        self.inner_file.seek(SeekFrom::Start(0)).await?;

        // Copy file
        bytes_written += io::copy(&mut self.inner_file, writer).await? as usize;

        Ok(bytes_written)
    }
}

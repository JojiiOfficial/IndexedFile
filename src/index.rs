use std::io::SeekFrom;

use async_std::{
    io::{prelude::*, BufReader, Read},
    stream::StreamExt,
};
use itertools::Itertools;

use crate::{error::Error, Result};

/// Contains an in-memory line-index
#[derive(Debug, Clone)]
pub struct Index {
    /// Maps line to seek position in order to seek efficiently. The index within the Vec represents
    /// the line-index in the file
    inner: Vec<u64>,
    /// The len in bytes of the index
    len_bytes: usize,
}

impl Index {
    /// Parse an encoded index usually stored in the first line of a file.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let data_str = String::from_utf8(data.to_vec())?;
        let len_bytes = data_str.len() + 1;

        let inner: Vec<u64> = data_str
            .split(',')
            .map(|i| -> Result<u64> { i.parse().map_err(|_| Error::MalformedIndex) })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(Self { inner, len_bytes })
    }

    /// Build a new index for UTF8-text within `reader`. Returns a `Vec<u8>` holding the bytes representing
    /// the index in encoded format. This is usually needed for building an indexed file.
    pub async fn build<R: Read + Unpin + Seek>(reader: &mut BufReader<R>) -> Result<Self> {
        let mut lines = reader.lines();

        let mut line_index: Vec<u64> = Vec::new();

        let mut curr_offset: u64 = 0;
        while let Some(line) = lines.next().await {
            line_index.push(curr_offset);

            // Calculate offset of next line. We have to do +1 since we're omitting the \n
            curr_offset += line?.len() as u64 + 1;
        }

        // Reset reader
        reader.seek(SeekFrom::Start(0)).await?;

        Ok(Self {
            inner: line_index,
            len_bytes: 0,
        })
    }

    /// Encodes an index into bytes, which can be used to store it into a file.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = self.inner.iter().map(|i| format!("{}", i)).join(",");
        out.push('\n');
        out.as_bytes().to_vec()
    }

    /// Get the Index value
    #[inline]
    pub fn get(&self, pos: usize) -> Result<u64> {
        self.inner.get(pos).ok_or(Error::OutOfBounds).map(|i| *i)
    }

    /// Returns the amount of items of the index. On a properly built index, this represents the
    /// amount of lines in the file without counting the index.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Get the len of the index
    pub fn len_bytes(&self) -> usize {
        self.len_bytes
    }

    /// Returns `true` if the index is empty
    pub fn is_empty(&self) -> bool {
        self.len_bytes() == 0
    }

    /// Parse an index from a reader.
    pub(super) async fn parse_index<R: Read + Unpin>(reader: &mut BufReader<R>) -> Result<Index> {
        let mut first_line = Vec::new();
        reader.read_until(b'\n', &mut first_line).await?;

        if first_line.len() <= 1 {
            return Err(Error::MissingIndex);
        }

        // Remove last '\n'
        first_line.pop();

        Ok(Index::parse(&first_line)?)
    }
}

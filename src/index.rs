use std::{
    convert::TryInto,
    io::{prelude::*, BufReader, Read, SeekFrom},
};

use crate::{error::Error, Result};

/// Length of header in bytes
const HEADER_SIZE: usize = 8;

/// An index header
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Header {
    /// Count of files lines.
    /// This value is equivalent to the amount of entries in the index
    items: usize,
}

impl Header {
    #[inline]
    pub(crate) fn new(items: usize) -> Self {
        Self { items }
    }

    /// Encode a header to bytes.
    #[inline]
    pub(crate) fn encode(&self) -> [u8; HEADER_SIZE] {
        let enc: [u8; HEADER_SIZE] = self.items.to_le_bytes().try_into().unwrap();
        enc
    }

    /// Decodes a header from a reader
    pub fn decode<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        reader.seek(SeekFrom::Start(0))?;

        let mut header: [u8; 8] = [0; 8];
        reader.read_exact(&mut header)?;

        let lines = usize::from_le_bytes(header);

        Ok(Header { items: lines })
    }
}

/// Contains an in-memory line-index
#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    /// Maps line to seek position in order to seek efficiently. The index within the Vec represents
    /// the line-index in the file
    inner: Vec<u32>,
    /// The len in bytes of the index and the header
    len_bytes: usize,
}

impl Index {
    /// Create a new Index
    #[inline]
    pub fn new<T: IntoIterator<Item = u32>>(line: T) -> Index {
        let inner = line.into_iter().collect::<Vec<_>>();
        let inner_byte_size = inner.len() * 4 + 1;
        Self {
            len_bytes: HEADER_SIZE + inner_byte_size,
            inner,
        }
    }

    /// Build a new index for text within `reader`. Returns a `Vec<u8>` holding the bytes representing
    /// the index in encoded format. This is usually needed for building an indexed file.
    pub fn build<R: Read + Unpin + Seek>(reader: &mut BufReader<R>) -> Result<Self> {
        // Seeking to 0 doesn't throw an error so we can unwrap it
        reader.seek(SeekFrom::Start(0)).unwrap();

        //let mut line_index: Vec<u64> = Vec::new();
        let mut line_index = Vec::new();
        let mut curr_offset: u64 = 0;

        let mut buff = Vec::with_capacity(1000);

        loop {
            let last_offset = curr_offset;

            buff.clear();
            let n = reader.read_until(b'\n', &mut buff)?;

            if n == 0 || buff.is_empty() {
                break;
            }

            // We don't want to push the last line-index twice which we would if this was at the
            // top of the loop
            line_index.push(last_offset as u32);

            curr_offset += n as u64;
        }

        // Seeking to 0 doesn't throw an error so we can unwrap it
        reader.seek(SeekFrom::Start(0)).unwrap();

        Ok(Self {
            // Storing it in a normal vec first is faster than pushing it repetitively
            inner: line_index.into(),
            len_bytes: 0,
        })
    }

    /// Adds a new value to the index
    #[inline]
    pub fn add(&mut self, pos: u32) {
        self.inner.push(pos);
        // Update length since we (might) have changed the index len
        self.len_bytes = self.calc_length();
    }

    /// Encodes an index into bytes, which can be used to store it into a file.
    #[inline]
    pub fn encode(&self) -> Vec<u8> {
        let mut out: Vec<_> = self
            .inner
            .iter()
            .map(|i| i.to_le_bytes())
            .flatten()
            .collect();
        out.push(b'\n');
        out
    }

    /// Returns true if the index has a given value
    #[inline]
    pub fn has(&self, pos: usize) -> bool {
        self.inner.get(pos).is_some()
    }

    /// Calculate the index size
    #[inline]
    pub fn calc_length(&self) -> usize {
        HEADER_SIZE + self.len() * 4
    }

    /// Decodes an encoded index
    pub fn decode<R: Read + Unpin + Seek>(reader: &mut R, header: &Header) -> Result<Self> {
        // Skip header bytes
        reader.seek(SeekFrom::Start(HEADER_SIZE as u64))?;

        // List of the beginning offset of each line in the file
        let mut inner: Vec<u32> = Vec::new();

        // Decode line indices
        let mut buff = [0u8; 4];
        for _ in 0..header.items {
            reader.read_exact(&mut buff)?;
            inner.push(u32::from_le_bytes(
                buff.try_into().map_err(|_| Error::MalformedIndex)?,
            ));
        }

        Ok(Index::new(inner))
    }

    /// Converts an `Index` to an index with zero length
    #[inline]
    pub fn zero_len(self) -> Self {
        Self {
            len_bytes: 0,
            inner: self.inner,
        }
    }

    /// Generate a header out of the index
    #[inline]
    pub(crate) fn get_header(&self) -> Header {
        Header::new(self.inner.len())
    }

    /// Get the Index value at `pos`
    #[inline]
    pub fn get(&self, pos: usize) -> Result<u32> {
        Ok(*self.inner.get(pos).ok_or(Error::OutOfBounds)?)
    }

    /// Returns the amount of items of the index. On a properly built index, this represents the
    /// amount of lines in the file without counting the index.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Get the len of the index in bytes
    #[inline]
    pub fn len_bytes(&self) -> usize {
        self.len_bytes
    }

    /// Returns `true` if the index is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len_bytes() == 0
    }

    /// Parse an index from a reader
    #[inline]
    pub(super) fn parse_index<R: Read + Unpin + Seek>(reader: &mut R) -> Result<Index> {
        let header = Header::decode(reader)?;
        let index = Index::decode(reader, &header)?;
        Ok(index)
    }
}

impl Extend<u32> for Index {
    /// Adds the values to the index. This should be preferred over `add` since it is faster
    fn extend<T: IntoIterator<Item = u32>>(&mut self, iter: T) {
        self.inner.extend(iter);

        // Update length since we (might) have changed the index len
        self.len_bytes = self.calc_length();
    }
}

//!A simple library to index and read large files by its lines.

pub mod error;
pub mod index;

use error::Error;
use index::Index;

use std::io::SeekFrom;

use async_std::{
    fs,
    io::{self, prelude::*, BufReader},
    path::Path,
};
use async_trait::async_trait;

pub type Result<T> = std::result::Result<T, error::Error>;

/// A trait defining behavior for reading certain lines directly from indexed files.
#[async_trait]
pub trait ReadByLine {
    /// Should return the offset to seek to given the line-index
    fn get_offset(&self, line: usize) -> Result<u64>;

    /// Should seek to the reader used in `read_to_eol` to the given `SeekFrom`
    async fn seek(&mut self, line: usize) -> Result<u64>;

    /// Should read from the current position until the end of the line, omitting the \n
    async fn read_to_eol(&mut self, buf: &mut Vec<u8>) -> Result<usize>;

    /// Reads the given line
    async fn read_line(&mut self, line: usize) -> Result<String> {
        self.seek(line).await?;

        let mut read_data = Vec::new();
        self.read_to_eol(&mut read_data).await?;
        Ok(String::from_utf8(read_data)?)
    }

    /// Reads the given line and stores into `buf`
    async fn read_line_raw(&mut self, line: usize, buf: &mut Vec<u8>) -> Result<usize> {
        self.seek(line).await?;
        Ok(self.read_to_eol(buf).await?)
    }
}

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

        let index = Self::parse_index(&mut inner_file).await?;

        Ok(Self {
            index,
            inner_file,
            last_line: None,
        })
    }

    /// Opens a non indexed file and generates the index.
    pub async fn open_raw<P: AsRef<Path>>(path: P) -> Result<File> {
        let mut inner_file = BufReader::new(fs::File::open(path).await?);

        let index = Index::build(&mut inner_file).await?;

        Ok(Self {
            index,
            inner_file,
            last_line: None,
        })
    }

    /// Opens a non indexed file and uses a custom index `index`.
    /// Expects the index to be properly built.
    pub async fn open_custom<P: AsRef<Path>>(path: P, index: Index) -> Result<File> {
        let inner_file = BufReader::new(fs::File::open(path).await?);

        Ok(Self {
            index,
            inner_file,
            last_line: None,
        })
    }

    /// Writes the index, followed by the files contents into `writer`. A file generated using this
    /// function will always be parsable by `File::open`.
    pub async fn write_to<W: Write + Unpin>(&mut self, writer: &mut W) -> Result<usize> {
        let encoded_index = self.index.encode();
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

    /// Returns the total amount of lines of the file.
    #[inline]
    pub fn total_lines(&self) -> usize {
        self.index.len()
    }

    /// Returns a reference to the files index.
    #[inline]
    pub fn get_index(&self) -> &Index {
        &self.index
    }

    /// Parse an index from a reader.
    async fn parse_index(reader: &mut BufReader<fs::File>) -> Result<Index> {
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

#[async_trait]
impl ReadByLine for File {
    #[inline(always)]
    fn get_offset(&self, line: usize) -> Result<u64> {
        self.index
            .get(line)
            // The indexed value represents the position of the line in the original file. We need
            // to add the amount of bytes of the index to the seek position.
            .map(|i| i + (self.index.len_bytes() as u64))
    }

    #[inline(always)]
    async fn seek(&mut self, line: usize) -> Result<u64> {
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

    #[inline(always)]
    async fn read_to_eol(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let res = self.inner_file.read_until(b'\n', buf).await?;

        // Pop last \n if existing
        if res > 0 && *buf.last().unwrap() == b'\n' {
            buf.pop();
        }

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::{fs, stream::StreamExt};

    #[async_std::test]
    async fn test_empty() {
        let indexed_file = File::open("./testfiles/empty").await;
        assert!(indexed_file.is_err());
    }

    #[async_std::test]
    async fn test_sequencial() {
        let input_files = &["input1", "LICENSE"];
        let output_file = "./test1_out";

        for input_file in input_files {
            let input_file = format!("./testfiles/{}", input_file);
            generate_index(&input_file, output_file).await;

            // Open indexed file
            let mut indexed_file = File::open(output_file)
                .await
                .expect("failed opening indexed file");

            // Read original file and match against indexed file
            let mut original_file =
                BufReader::new(fs::File::open(input_file).await.unwrap()).lines();
            let mut curr_line = 0;
            while let Some(line) = original_file.next().await {
                let orig_line: String = line.unwrap();
                let indexed_line = indexed_file
                    .read_line(curr_line)
                    .await
                    .expect("Failed to read line");

                assert_eq!(orig_line, indexed_line);
                curr_line += 1;
            }
        }
    }

    async fn generate_index(input_file: &str, output_file: &str) {
        // Open input file
        let mut tf = File::open_raw(input_file).await.unwrap();

        // Open output
        let mut output = fs::File::create(output_file)
            .await
            .expect("failed to create output");

        // Write indexed file to `output`
        tf.write_to(&mut output)
            .await
            .expect("failed to write indexed file");
    }
}

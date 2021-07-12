//!A simple library to index and read large files by its lines.

pub mod bufreader;
pub mod error;
pub mod file;
pub mod index;

use async_std::io::Write;
pub use file::File;

use async_trait::async_trait;
use index::Index;

pub type Result<T> = std::result::Result<T, error::Error>;

pub trait Indexable {
    /// Returns a reference to the files index.
    fn get_index(&self) -> &Index;

    /// Returns the total amount of lines in the file without the lines used by the index.
    #[inline]
    fn total_lines(&self) -> usize {
        self.get_index().len()
    }

    #[inline]
    fn get_index_byte_len(&self) -> usize {
        self.get_index().len_bytes()
    }
}

#[async_trait]
pub trait IndexableFile: Indexable {
    /// Should read from the current position until the end of the line, omitting the \n
    async fn read_current_line(&mut self, buf: &mut Vec<u8>) -> Result<usize>;

    /// Should seek the file to the given line `line`
    async fn seek_line(&mut self, line: usize) -> Result<()>;

    /// Write the index, followed by the files contents into `writer`. A file generated using this
    /// function will always be parsable by `File::open`.
    async fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize>;

    /// Should return the offset to seek to given the line-index
    #[inline(always)]
    fn get_offset(&self, line: usize) -> Result<u64> {
        self.get_index().get(line)
    }
}

/// A trait defining behavior for reading certain lines directly from indexed files.
#[async_trait]
pub trait ReadByLine: IndexableFile {
    /// Reads the given line
    async fn read_line(&mut self, line: usize) -> Result<String> {
        self.seek_line(line).await?;

        let mut read_data = Vec::new();
        self.read_current_line(&mut read_data).await?;
        Ok(String::from_utf8(read_data)?)
    }

    /// Reads the given line and stores into `buf`
    async fn read_line_raw(&mut self, line: usize, buf: &mut Vec<u8>) -> Result<usize> {
        self.seek_line(line).await?;
        Ok(self.read_current_line(buf).await?)
    }
}

impl<T: IndexableFile> ReadByLine for T {}

#[cfg(test)]
mod tests {
    use crate::IndexableFile;

    use super::*;
    use async_std::{
        fs,
        io::{prelude::*, BufReader},
        stream::StreamExt,
    };

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

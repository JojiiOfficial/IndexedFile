//!A simple library to index and read large files by its lines using a pregenerated index

/// Generic implementation to use various types as reader
pub mod any;
/// Basic implementation for std::io::BufReader
pub mod bufreader;
pub mod error;
/// A wrapper around std::fs::File which implements ReadByLine
pub mod file;
/// The index of files
pub mod index;
/// An indexed string reader
pub mod string;

pub use file::File;
pub use string::IndexedString;

use std::{cmp::Ordering, io::Write};

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

pub trait IndexableFile: Indexable {
    /// Should read from the current position until the end of the line, omitting the \n
    fn read_current_line(&mut self, buf: &mut Vec<u8>, line: usize) -> Result<usize>;

    /// Should seek the file to the given line `line`
    fn seek_line(&mut self, line: usize) -> Result<()>;

    /// Write the index, followed by the files contents into `writer`. A file generated using this
    /// function will always be parsable by `File::open`.
    fn write_to<W: Write + Unpin + Send>(&mut self, writer: &mut W) -> Result<usize>;

    /// Should return the offset to seek to given the line-index
    #[inline(always)]
    fn get_offset(&self, line: usize) -> Result<u64> {
        self.get_index().get(line)
    }
}

/// A trait defining behavior for reading certain lines directly from indexed files.
pub trait ReadByLine: IndexableFile {
    /// Reads the given line
    fn read_line(&mut self, line: usize) -> Result<String> {
        self.seek_line(line)?;
        let mut read_data = Vec::new();
        self.read_current_line(&mut read_data, line)?;
        Ok(String::from_utf8(read_data)?)
    }

    /// Reads the given line and stores into `buf`
    fn read_line_raw(&mut self, line: usize, buf: &mut Vec<u8>) -> Result<usize> {
        self.seek_line(line)?;
        Ok(self.read_current_line(buf, line)?)
    }

    /// Do a binary search on `ReadByLine` implementing Types, since it provides everything required
    /// for binary search. Only works with sorted files
    #[inline]
    fn binary_search(&mut self, x: &str) -> Result<usize> {
        self.binary_search_by(|p| p.cmp(x))
    }

    /// Do a binary search by on `ReadByLine` implementing Types, since it provides everything required
    /// for binary search. Only works with sorted files
    fn binary_search_by<F>(&mut self, mut f: F) -> Result<usize>
    where
        F: FnMut(&str) -> std::cmp::Ordering,
    {
        let mut size = self.total_lines();
        let mut left = 0;
        let mut right = size;

        while left < right {
            let mid = left + size / 2;

            let cmp = f(&self.read_line(mid)?);

            if cmp == Ordering::Less {
                left = mid + 1;
            } else if cmp == Ordering::Greater {
                right = mid;
            } else {
                return Ok(mid);
            }

            size = right - left;
        }

        Err(error::Error::NotFound)
    }

    /// Do a binary search on `ReadByLine` implementing Types, since it provides everything required
    /// for binary search. Only works with sorted files
    #[inline]
    fn binary_search_raw(&mut self, x: &[u8]) -> Result<usize> {
        self.binary_search_raw_by(|p| p.cmp(x))
    }

    /// Do a binary search by on `ReadByLine` implementing Types, since it provides everything required
    /// for binary search. Only works with sorted files
    fn binary_search_raw_by<F>(&mut self, mut f: F) -> Result<usize>
    where
        F: FnMut(&[u8]) -> std::cmp::Ordering,
    {
        let mut size = self.total_lines();
        let mut left = 0;
        let mut right = size;

        let mut buf = Vec::new();

        while left < right {
            let mid = left + size / 2;

            buf.clear();
            self.read_line_raw(mid, &mut buf)?;
            let cmp = f(&buf);

            if cmp == Ordering::Less {
                left = mid + 1;
            } else if cmp == Ordering::Greater {
                right = mid;
            } else {
                return Ok(mid);
            }

            size = right - left;
        }

        Err(error::Error::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use rand::{distributions::Uniform, Rng};

    use crate::{any::IndexedReader, string::IndexedString};

    use super::*;
    use std::{
        convert::TryInto,
        fs::read_to_string,
        io::{prelude::*, BufReader},
        sync::Arc,
    };

    #[test]
    fn test_empty() {
        let indexed_file = File::open("./testfiles/empty");
        assert!(indexed_file.is_err());
    }

    #[test]
    fn test() {
        let input_files = &["simple", "LICENSE", "input1"];

        for input_file in input_files {
            let file = format!("./testfiles/{}", input_file);

            // Test File
            let mut indexed_file = File::open_raw(&file).expect("failed opening indexed file");
            test_reader(&mut indexed_file, &file);

            let mut buf = Vec::new();
            indexed_file.read_all(&mut buf).unwrap();
            assert_eq!(buf, read_to_string(&file).unwrap().as_bytes());

            // Test IndexedString
            let file_content = read_to_string(&file).unwrap();
            let mut indexed_string = IndexedString::new_raw(file_content).unwrap();
            test_reader(&mut indexed_string, &file);

            // Test File to indexed string
            let indexed_file = File::open_raw(&file).expect("failed opening indexed file");
            let indexed_str_file: Result<IndexedString> = indexed_file.try_into();
            assert!(indexed_str_file.is_ok());
            test_reader(&mut indexed_str_file.unwrap(), &file);

            // Test File to indexed Vec<u8>
            let indexed_file = File::open_raw(&file).expect("failed opening indexed file");
            let indexed_str_file: Result<IndexedReader<Vec<u8>>> = indexed_file.try_into();
            assert!(indexed_str_file.is_ok());
            test_reader(&mut indexed_str_file.unwrap(), &file);
        }
    }

    fn test_reader<L: ReadByLine>(reader: &mut L, original_file: &str) {
        test_sequencially(reader, &original_file);
        test_random(reader, &original_file);
    }

    fn test_sequencially<L: ReadByLine>(reader: &mut L, original_file: &str) {
        let original = BufReader::new(std::fs::File::open(&original_file).unwrap());

        for (line, original) in original.lines().enumerate() {
            let original = original.unwrap();

            let read = reader.read_line(line);

            assert!(read.is_ok());
            assert_eq!(original, read.unwrap());

            let mut buf = Vec::new();
            let res = reader.read_line_raw(line, &mut buf);
            assert!(res.is_ok());
            assert_eq!(original, String::from_utf8(buf).unwrap());
        }
    }

    fn test_random<L: ReadByLine>(reader: &mut L, original_file: &str) {
        let original = BufReader::new(std::fs::File::open(&original_file).unwrap());
        let orig_content: Vec<_> = original.lines().map(|i| i.unwrap()).collect();

        let lines: Vec<_> = rand::thread_rng()
            .sample_iter(Uniform::new(0, reader.total_lines() - 1))
            .take(reader.total_lines() * 3)
            .collect();

        for line in lines {
            let original = orig_content.get(line).unwrap();
            let read = reader.read_line(line);

            assert!(read.is_ok());
            assert_eq!(*original, read.unwrap());

            let mut buf = Vec::new();
            let res = reader.read_line_raw(line, &mut buf);
            assert!(res.is_ok());
            assert_eq!(*original, String::from_utf8(buf).unwrap());
        }
    }

    #[test]
    fn test_no_new_line() {
        let index = Index::new(vec![0, 5, 10, 15, 20, 25, 30, 35, 40, 45]);
        let text = "this is a text with fifty characters. this is a te";

        let mut indexed_text = IndexedString::new_custom(text, Arc::new(index.zero_len()));

        for i in 0..indexed_text.total_lines() {
            let read = indexed_text.read_line(i).unwrap();
            assert_eq!(read.len(), 4);
        }

        let mut buf = Vec::new();
        indexed_text.reader.read_all(&mut buf).unwrap();
        assert_eq!(buf, text.as_bytes());
    }

    #[test]
    fn test_write_to() {
        let file = "./testfiles/pre_indexed";

        let mut indexed_data = Vec::new();
        File::open(&file)
            .expect("failed opening pre_indexed file")
            .write_to(&mut indexed_data)
            .unwrap();

        let mut raw_data = Vec::new();
        std::fs::File::open(file)
            .unwrap()
            .read_to_end(&mut raw_data)
            .unwrap();

        assert_eq!(raw_data, indexed_data);
    }
}

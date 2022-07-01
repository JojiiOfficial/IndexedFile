use crate::index::Index;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemFile {
    data: Vec<u8>,
    index: Index,
}

impl MemFile {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            index: Index::default(),
        }
    }

    #[inline]
    pub fn new_raw(data: Vec<u8>, index: Index) -> Self {
        Self { data, index }
    }

    /// Inserts Data into the file
    #[inline]
    pub fn insert(&mut self, data: &[u8]) -> usize {
        let pos = self.index.len();
        self.index.add(self.data.len() as u32);
        self.data.extend_from_slice(data);
        pos
    }

    /// Replaces an entry with new data. This automatically adjusts the index which means the new input can be any size.
    /// Depending on amount of data stored in MemFile this can take some time
    pub fn replace(&mut self, pos: usize, data: &[u8]) -> Option<()> {
        let (start, end) = self.index_range(pos)?;
        self.data.splice(start..end, data.iter().copied());
        let diff = data.len() as isize - (start..end).len() as isize;

        for i in self.index.inner.iter_mut().skip(pos + 1) {
            *i = (*i as isize + diff) as u32;
        }

        Some(())
    }

    #[inline]
    pub fn get(&self, pos: usize) -> Option<&[u8]> {
        let (start, end) = self.index_range(pos)?;
        Some(&self.data[start..end])
    }

    #[inline]
    pub fn get_unchecked(&self, pos: usize) -> &[u8] {
        let (start, end) = self.index_range_unchecked(pos);
        &self.data[start..end]
    }

    #[inline]
    fn index_range(&self, pos: usize) -> Option<(usize, usize)> {
        let start = self.index.get2(pos)?;
        let next = self.index.get2(pos + 1).unwrap_or(self.raw_len());
        Some((start, next))
    }

    #[inline]
    fn index_range_unchecked(&self, pos: usize) -> (usize, usize) {
        let start = self.index.get_unchecked(pos);
        let next_pos = pos + 1;
        if next_pos < self.index.inner.len() {
            (start, self.index.get_unchecked(next_pos))
        } else {
            (start, self.raw_len())
        }
    }

    #[inline]
    pub fn iter(&self) -> MemFileIter<'_> {
        MemFileIter::new(self)
    }

    /// Returns the amount of entries in the file
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Returns the amount of bytes stored in the file
    #[inline(always)]
    pub fn raw_len(&self) -> usize {
        self.data.len()
    }
}

impl<I: AsRef<[u8]>> Extend<I> for MemFile {
    #[inline]
    fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
        for line in iter {
            self.insert(line.as_ref());
        }
    }
}

impl<U: Iterator<Item = impl AsRef<[u8]>>> From<U> for MemFile {
    fn from(iter: U) -> Self {
        let mut new = MemFile::new();
        for i in iter {
            new.insert(i.as_ref());
        }
        new
    }
}

pub struct MemFileIter<'a> {
    file: &'a MemFile,
    pos: usize,
}

impl<'a> MemFileIter<'a> {
    #[inline]
    fn new(file: &'a MemFile) -> Self {
        Self { file, pos: 0 }
    }
}

impl<'a> Iterator for MemFileIter<'a> {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let content = self.file.get(self.pos)?;
        self.pos += 1;
        Some(content)
    }
}

impl Default for MemFile {
    #[inline]
    fn default() -> Self {
        Self {
            data: Default::default(),
            index: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs::read_to_string};

    use super::*;

    fn test_data() -> &'static [&'static str] {
        &[
            "俺はプログラミングできねええ",
            "音楽好き",
            "昨日のコーヒー飲んじゃった",
            "this is a text",
        ]
    }

    #[test]
    fn test_mem_file_unicode() {
        test_entries(test_data());
    }

    #[test]
    fn test_replace() {
        let mut m_file = MemFile::new();
        let data = test_data();
        for entry in data {
            m_file.insert(entry.as_bytes());
        }

        for (pos, i) in data.iter().enumerate() {
            assert_eq!(m_file.get(pos).unwrap(), i.as_bytes());
        }

        m_file.replace(0, "lol".as_bytes()).unwrap();
        assert_eq!(m_file.get(0), Some("lol".as_bytes()));
        for (pos, i) in data.iter().enumerate().skip(1) {
            assert_eq!(m_file.get(pos).unwrap(), i.as_bytes());
        }

        m_file.replace(0, "sometesttextあぶ".as_bytes()).unwrap();
        assert_eq!(m_file.get(0), Some("sometesttextあぶ".as_bytes()));
        for (pos, i) in data.iter().enumerate().skip(1) {
            assert_eq!(m_file.get(pos).unwrap(), i.as_bytes());
        }

        m_file.replace(0, data[0].as_bytes()).unwrap();
        for (pos, i) in data.iter().enumerate() {
            assert_eq!(m_file.get(pos).unwrap(), i.as_bytes());
        }

        m_file
            .replace(m_file.len() - 1, "lastlol".as_bytes())
            .unwrap();
        for (pos, i) in data.iter().enumerate().rev().skip(1) {
            assert_eq!(m_file.get(pos).unwrap(), i.as_bytes());
        }
        assert_eq!(m_file.get(m_file.len() - 1).unwrap(), "lastlol".as_bytes());
    }

    #[test]
    fn test_mem_file_2() {
        let input_files = &["simple", "LICENSE", "input1"];

        for input_file in input_files {
            let file = format!("./testfiles/{}", input_file);
            let content = read_to_string(file).unwrap();

            let split: Vec<_> = content.split('\n').collect();
            test_entries(&split);
            test_from_iter(&split);

            let split: Vec<_> = content.split(' ').collect();
            test_entries(&split);
            test_from_iter(&split);
        }
    }

    fn test_entries(entries: &[&str]) {
        let mut new_file = MemFile::new();
        let mut map = HashMap::new();

        for (pos, entry) in entries.iter().enumerate() {
            let file_pos = new_file.insert(entry.as_bytes());
            map.insert(pos, file_pos);
        }

        assert_eq!(new_file.len(), entries.len());

        for (entry_pos, file_pos) in map {
            let entry = entries[entry_pos];
            let raw = new_file.get(file_pos).unwrap();
            let file_str = std::str::from_utf8(raw).unwrap();
            assert_eq!(file_str, entry);
        }

        for (res, exp) in new_file.iter().zip(entries.iter()) {
            let res_str = std::str::from_utf8(res).unwrap();
            assert_eq!(res_str, *exp);
        }
    }

    fn test_from_iter(entries: &[&str]) {
        let new_file = MemFile::from(entries.iter());

        assert_eq!(new_file.len(), entries.len());

        for (pos, entry) in entries.iter().enumerate() {
            let raw = new_file.get(pos).unwrap();
            let file_str = std::str::from_utf8(raw).unwrap();
            assert_eq!(file_str, *entry);
        }
    }
}

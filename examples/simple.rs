//! Index a normal text file and read a given line directly

use indexed_file::{Indexable, ReadByLine};

fn main() {
    // Open and index file
    let mut file = indexed_file::File::open_raw("./testfiles/LICENSE").unwrap();

    // Get line count efficiently without reading the entire file
    let line_count = file.total_lines();
    println!("File contains {} lines in total", line_count);

    // Read line 32
    let line_32 = file.read_line(32).unwrap();

    println!("Line 32 content: {}", line_32);
}

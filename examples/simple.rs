//! Index a normal text file and read a given line directly

use indexed_file::{IndexableFile, ReadByLine};

#[async_std::main]
async fn main() {
    // Open and index file
    let mut file = indexed_file::File::open_raw("<some unindexed file>")
        .await
        .unwrap();

    // Get line count efficiently without reading the entire file
    let _line_count = file.total_lines();

    // Read line 30
    let _line_30 = file.read_line(30).await.unwrap();
}

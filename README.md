# IndexedFile
A library to read lines of a file directly without having to read more than the requested line.

# Example

### Non indexed files
```rust
use indexed_file::{Indexable, ReadByLine};

#[async_std::main]
async fn main() {
    // Open and index a file
    let mut file = indexed_file::File::open_raw("<some unindexed file>")
        .await
        .unwrap();

    // Get line count efficiently without reading the entire file
    let line_count = file.total_lines();

    // Read line 30 directly
    let line_30 = file.read_line(30).await.unwrap();
}
```

### Indexed files
```rust
use indexed_file::{Indexable, ReadByLine};

#[async_std::main]
async fn main() {
    // Open an indexed file
    let mut file = indexed_file::File::open("<some indexed file>")
        .await
        .unwrap();

    // Read line 30 directly
    let line_30 = file.read_line(30).await.unwrap();
}
```

For more examples visit the [examples directory](https://github.com/JojiiOfficial/IndexedFile/tree/master/examples).

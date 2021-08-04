# IndexedFile
A library to index files/strings and read them efficiently. 
This allows reading every line of a file/string directly without having to read everything from the beginning to the given line.
Both, binary and text files are supported.

# Index size
The index is stored in a [CVec](https://github.com/JojiiOfficial/CompressedVec) which means it is stored compressed in the file and also in the applications memory.

# Example

### Non indexed files
```rust
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
```

### Indexed files
```rust
use indexed_file::{Indexable, ReadByLine};

fn main() {
    // Open an indexed file
    let mut file = indexed_file::File::open("<some indexed file>").unwrap();

    // Read line 32 directly
    let line_32 = file.read_line(32).unwrap();
}
```

For more examples visit the [examples directory](https://github.com/JojiiOfficial/IndexedFile/tree/master/examples).

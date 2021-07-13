//! Convert a normal to an indexed file

use indexed_file::IndexableFile;

fn main() {
    // Open and index file
    let mut file = indexed_file::File::open_raw("<some unindexed file>").unwrap();

    // Create a new file which will contain the index data and the original content
    let mut output = std::fs::File::create("output").unwrap();

    // Store index together with the files content into `output`
    file.write_to(&mut output).unwrap();

    // Now we don't need to build the index each time we open the file but can use `File::open`
    // which will load the stored index
    let _file = indexed_file::File::open("output").unwrap();
}

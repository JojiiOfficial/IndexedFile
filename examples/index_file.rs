//! Convert a normal to an indexed file

#[async_std::main]
async fn main() {
    // Open and index file
    let mut file = indexed_file::File::open_raw("<some unindexed file>")
        .await
        .unwrap();

    // Create a new file which will contain the index data and the original content
    let mut output = async_std::fs::File::create("output").await.unwrap();

    // Store index together with the files content into `output`
    file.write_to(&mut output).await.unwrap();

    // Now we don't need to build the index each time we open the file but can use `File::open`
    // which will load the stored index
    let file = indexed_file::File::open("output").await.unwrap();
}

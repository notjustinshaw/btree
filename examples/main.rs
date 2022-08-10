use std::path::Path;

use btree::{btree::BTreeBuilder, error::Error, node_type::KeyValuePair};

fn main() -> Result<(), Error> {
    // Initialize a new BTree;
    // The BTree nodes are stored in file '/tmp/db' (created if does not exist)
    // with parameter b=2.
    let mut btree = BTreeBuilder::new()
        .path(Path::new("/tmp/db"))
        .b_parameter(2)
        .build()?;

    // Write some data.
    println!("Inserting data...");
    for i in 0..100_000 {
        println!("Inserting {}", i);
        btree.insert(KeyValuePair::new(i.to_string(), i.to_string()))?;
    }

    // Read it back.
    println!("Reading data...");
    for i in 0..100_000 {
        let kv = btree.search(i.to_string())?;
        assert_eq!(kv.key, i.to_string());
        assert_eq!(kv.value, i.to_string());
        println!("Found: {} => {}", kv.key, kv.value);
    }

    Ok(())
}

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
    btree.insert(KeyValuePair::new("justin".to_string(), "shaw".to_string()))?;
    btree.insert(KeyValuePair::new("hallie".to_string(), "jones".to_string()))?;
    btree.insert(KeyValuePair::new("soren".to_string(), "rood".to_string()))?;

    // Read it back.
    let mut kv = btree.search("justin".to_string())?;
    assert_eq!(kv.key, "justin");
    assert_eq!(kv.value, "shaw");

    kv = btree.search("soren".to_string())?;
    assert_eq!(kv.key, "soren");
    assert_eq!(kv.value, "rood");

    Ok(())
}

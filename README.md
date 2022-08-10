
<p align="center">
  <img src="docs/logo.png" width="300" height="230">
</p>

# btree

[![Build status](https://github.com/nimrodshn/btree/actions/workflows/build.yml/badge.svg)](https://github.com/nimrodshn/btree/actions)
[![GitHub commit activity](https://img.shields.io/github/commit-activity/m/nimrodshn/btree)](https://github.com/nimrodshn/btree/graphs/commit-activity)

A *persistent copy-on-write* B+Tree implementation, designed as an index for a key-value store, inspired by [SQLite](https://www.sqlite.org/index.html).

## Design
Each `BTree` struct is associated with a file that contains its nodes in a predefined structure.
The `BTree` API is implemented in a copy-on-write manner, that is, a copy of the newly written nodes is created on each write or delete without mutating the previous version of the tree. To keep track of the latest version of the tree we maintain a write-ahead-log to log the current root.

Unit tests serve as helpful examples of API usage.

## On disk node structure
There are two `NodeType` variants - `Internal` and `Leaf`; Each variant has its own predefined structure on disk.

#### Internal Node
An internal node has the following structure (the number of keys is always one
less than the number of children):
```
0         1           2                           10                          18
+---------+-----------+---------------------------+---------------------------+
| is_root | node_type | parent_offset (8 bytes)   | num_children (8 bytes)    |
+---------+-----------+---------------------------+---------------------------+
+--------------------------------------+--------------------------------------+
| child_offset #0 (8 bytes)            | child_offset #1 (8 bytes)            |
+--------------------------------------+--------------------------------------+
| child_offset #2 (8 bytes)            | ...                                  |
+--------------------------------------+--------------------------------------+
+---------------------------+-------------------------------------------------+
| key_size_0 (8 bytes)      | key #0 (key_size_1 bytes)                       |
+---------------------------+-------------------------------------------------+
| key_size_1 (8 bytes)      | key #1 (key_size_2 bytes)                       |
+---------------------------+-------------------------------------------------+
| key_size_2 (8 bytes)      | key #2 (key_size_0 bytes)                       |
+---------------------------+-------------------------------------------------+
```

#### Leaf Node
While the structure of a leaf node on disk is the following:
```
0         1           2                           10                          18
+---------+-----------+---------------------------+---------------------------+
| is_root | node_type | parent_offset (8 bytes)   | num_pairs (8 bytes)       |
+---------+-----------+---------------------------+---------------------------+
+--------------------------------------+--------------------------------------+
| key_size #0 (8 bytes)                | value_size #0 (8 bytes)              |
|--------------------------------------+--------------------------------------|
| key #0 (key_size bytes)                                                     |
|-----------------------------------------------------------------------------|
| value #0 (key_size bytes)                                                   |
+-----------------------------------------------------------------------------+
+--------------------------------------+--------------------------------------+
| key_size #1 (8 bytes)                | value_size #1 (8 bytes)              |
|--------------------------------------+--------------------------------------|
| key #1 (key_size bytes)                                                     |
|-----------------------------------------------------------------------------|
| value #1 (key_size bytes)                                                   |
+-----------------------------------------------------------------------------+
 ...
+--------------------------------------+--------------------------------------+
| key_size #N (8 bytes)                | value_size #N (8 bytes)              |
|--------------------------------------+--------------------------------------|
| key #N (key_size bytes)                                                     |
|-----------------------------------------------------------------------------|
| value #N (key_size bytes)                                                   |
+-----------------------------------------------------------------------------+
```

## Features
- [X] Support all CRUD operations (read, write, delete).
- [X] Support for crash recovery from disk.
- [ ] Support for varied length key-value pairs.
- [ ] Key compression.
- [ ] Garbage collection.

## API

### From disk to memory and back
Nodes are mapped to pages on disk with `TryFrom` methods implemented for easier de/serialization of nodes to pages and back.

```rust
let some_leaf = Node::new(
   NodeType::Leaf(vec![
         KeyValuePair::new("foo".to_string(), "bar".to_string()),
         KeyValuePair::new("lebron".to_string(), "james".to_string()),
         KeyValuePair::new("ariana".to_string(), "grande".to_string()),
   ]),
   true,
   None,
);

// Serialize data.
let page = Page::try_from(&some_leaf)?;
// Deserialize back the page.
let res = Node::try_from(page)?;
```

See tests at `src/page.rs` and `src/node.rs` for more information.

### Writing and Reading key-value pairs.
```rust
// Initialize a new BTree;
// The BTree nodes are stored in file '/tmp/db' (created if does not exist)
// with parameter b=2.
 let mut btree = BTreeBuilder::new()
            .path(Path::new("/tmp/db"))
            .b_parameter(2)
            .build()?;

// Write some data.
btree.insert(KeyValuePair::new("a".to_string(), "shalom".to_string()))?;
btree.insert(KeyValuePair::new("b".to_string(), "hello".to_string()))?;
btree.insert(KeyValuePair::new("c".to_string(), "marhaba".to_string()))?;

// Read it back.
let mut kv = btree.search("b".to_string())?;
assert_eq!(kv.key, "b");
assert_eq!(kv.value, "hello");

kv = btree.search("c".to_string())?;
assert_eq!(kv.key, "c");
assert_eq!(kv.value, "marhaba");
```

### Deleting key-value pairs.
```rust
// Initialize a new BTree.
let mut btree = BTreeBuilder::new()
      .path(Path::new("/tmp/db"))
      .b_parameter(2)
      .build()?;

// Write some data.
btree.insert(KeyValuePair::new("d".to_string(), "olah".to_string()))?;
btree.insert(KeyValuePair::new("e".to_string(), "salam".to_string()))?;
btree.insert(KeyValuePair::new("f".to_string(), "hallo".to_string()))?;
btree.insert(KeyValuePair::new("a".to_string(), "shalom".to_string()))?;
btree.insert(KeyValuePair::new("b".to_string(), "hello".to_string()))?;
btree.insert(KeyValuePair::new("c".to_string(), "marhaba".to_string()))?;

// Find the key.
let kv = btree.search("c".to_string())?;
assert_eq!(kv.key, "c");
assert_eq!(kv.value, "marhaba");

// Delete the key.
btree.delete(Key("c".to_string()))?;

// Sanity check.
let res = btree.search("c".to_string());
assert!(matches!(
      res,
      Err(Error::KeyNotFound)
));
```

## License
MIT.

use crate::error::Error;
use crate::node::Node;
use crate::node_type::{Key, NodeType, Offset};
use crate::page_layout::{
    ToByte, INTERNAL_NODE_HEADER_SIZE, INTERNAL_NODE_NUM_CHILDREN_OFFSET,
    INTERNAL_NODE_NUM_CHILDREN_SIZE, IS_ROOT_OFFSET, LEAF_NODE_HEADER_SIZE,
    LEAF_NODE_NUM_PAIRS_OFFSET, LEAF_NODE_NUM_PAIRS_SIZE, NODE_TYPE_OFFSET, PAGE_SIZE,
    PARENT_POINTER_OFFSET, PARENT_POINTER_SIZE, PTR_SIZE,
};
use std::convert::TryFrom;

/// Value is a wrapper for a value in the page.
pub struct Value(pub usize);

/// A wrapper for a single page of memory (ie. 4096 bytes).
///
/// ### Common Node Header
///
/// The page header is always at the beginning of the page. It consists of a
/// single byte that indicates the node type (leaf or internal), a single byte
/// that indicates whether the page is the root of the tree, and an eight-byte
/// offset of the parent's page.
///
/// ```text
/// 0         1           2                                 10
/// +---------+-----------+---------------------------------+
/// | is_root | node_type | parent_offset (8 bytes)         |
/// +---------+-----------+---------------------------------+
/// ```
///
/// ### Cell Layout
///
/// We assume that all cells in the page are of the same type (ie. they hold
/// only keys or only key-values-pairs, and they are all either fixed-sized or
/// variable-sized but not a mix of both).
///
/// Internal nodes will hold only keys, and leaf nodes will hold only key-value
/// pairs. For now, all cells are variable-sized. It would be cool to make the
/// keys fixed-sized for all key types that implement `Sized`, but that's a
/// project for another day.
///
/// A variable-sized key cell is laid out as follows:
/// ```text
/// 0                    8                                  8 + key_size
/// +--------------------+----------------------------------+
/// | [u64] key_size     | [bytes] key                      |
/// +--------------------+----------------------------------+
/// ```
///
/// A variable-sized key-value pair is laid out as follows:
/// ```text
/// 0                           8
/// +---------------------------+---------------------------+ 16
/// | [u64] key_size            | [u64] value_size          |
/// +---------------------------+---------------------------+ 16 + key_size
/// | [bytes] key                                           |
/// +-------------------------------------------------------+ .. + value_size
/// | [bytes] value                                         |
/// +-------------------------------------------------------+
/// ```
pub struct Page {
    data: Box<[u8; PAGE_SIZE]>,
}

impl Page {
    pub fn new(data: [u8; PAGE_SIZE]) -> Page {
        Page {
            data: Box::new(data),
        }
    }

    /// Writes a given value (as BigEndian) at a certain offset, overriding
    /// values at that offset.
    pub fn write_value_at_offset(&mut self, offset: usize, value: usize) -> Result<(), Error> {
        if offset > PAGE_SIZE - PTR_SIZE {
            return Err(Error::UnexpectedError);
        }
        let bytes = value.to_be_bytes();
        self.data[offset..offset + PTR_SIZE].clone_from_slice(&bytes);
        Ok(())
    }

    /// Fetches a value calculated as BigEndian, sized to usize at the given offset.
    /// This function may error as the value might not fit into a usize.
    pub fn get_value_from_offset(&self, offset: usize) -> Result<usize, Error> {
        let bytes = &self.data[offset..offset + PTR_SIZE];
        let Value(res) = Value::try_from(bytes)?;
        Ok(res)
    }

    /// Pushes #size bytes from offset to end_offset inserts #size bytes from
    /// given slice, overwriting existing values.
    pub fn insert_bytes_at_offset(
        &mut self,
        bytes: &[u8],
        offset: usize,
        end_offset: usize,
        size: usize,
    ) -> Result<(), Error> {
        // This Should not occur - better verify.
        if end_offset + size > self.data.len() {
            return Err(Error::UnexpectedError);
        }
        for idx in (offset..=end_offset).rev() {
            self.data[idx + size] = self.data[idx]
        }
        self.data[offset..offset + size].clone_from_slice(bytes);
        Ok(())
    }

    /// Writes #size bytes at the given offset from the given slice,
    /// overriding previous values.
    pub fn write_bytes_at_offset(
        &mut self,
        bytes: &[u8],
        offset: usize,
        size: usize,
    ) -> Result<(), Error> {
        self.data[offset..offset + size].clone_from_slice(bytes);
        Ok(())
    }

    /// Fetches a slice of #size bytes at the given offset.
    pub fn get_ptr_from_offset(&self, offset: usize, size: usize) -> &[u8] {
        &self.data[offset..offset + size]
    }

    /// Returns the underlying array.
    pub fn get_data(&self) -> [u8; PAGE_SIZE] {
        *self.data
    }
}

/// Implement TryFrom<Box<Node>> for Page allowing for easier
/// serialization of data from a Node to an on-disk formatted page.
impl TryFrom<&Node> for Page {
    type Error = Error;
    fn try_from(node: &Node) -> Result<Page, Error> {
        let mut data: [u8; PAGE_SIZE] = [0x00; PAGE_SIZE];
        // is_root byte
        data[IS_ROOT_OFFSET] = node.is_root.to_byte();

        // node_type byte
        data[NODE_TYPE_OFFSET] = u8::from(&node.node_type);

        // parent offest
        if !node.is_root {
            match node.parent_offset {
                Some(Offset(parent_offset)) => data
                    [PARENT_POINTER_OFFSET..PARENT_POINTER_OFFSET + PARENT_POINTER_SIZE]
                    .clone_from_slice(&parent_offset.to_be_bytes()),
                // Expected an offset of an inner / leaf node.
                None => return Err(Error::UnexpectedError),
            };
        }

        match &node.node_type {
            NodeType::Internal(child_offsets, keys) => {
                data[INTERNAL_NODE_NUM_CHILDREN_OFFSET
                    ..INTERNAL_NODE_NUM_CHILDREN_OFFSET + INTERNAL_NODE_NUM_CHILDREN_SIZE]
                    .clone_from_slice(&child_offsets.len().to_be_bytes());

                let mut page_offset = INTERNAL_NODE_HEADER_SIZE;
                for Offset(child_offset) in child_offsets {
                    data[page_offset..page_offset + PTR_SIZE]
                        .clone_from_slice(&child_offset.to_be_bytes());
                    page_offset += PTR_SIZE;
                }

                for Key(key) in keys {
                    let key_bytes = key.as_bytes();
                    let key_size: usize = key_bytes.len();

                    // write the key_size
                    data[page_offset..page_offset + PTR_SIZE]
                        .clone_from_slice(&key_size.to_be_bytes());
                    page_offset += PTR_SIZE;
                    
                    // write the key as bytes to the back of the freespace
                    data[page_offset..page_offset + key_size].clone_from_slice(key_bytes);
                    page_offset += key_size;
                }
            }
            NodeType::Leaf(kv_pairs) => {
                // num of pairs
                let num_pairs = kv_pairs.len();
                data[LEAF_NODE_NUM_PAIRS_OFFSET
                    ..LEAF_NODE_NUM_PAIRS_OFFSET + LEAF_NODE_NUM_PAIRS_SIZE]
                    .clone_from_slice(&num_pairs.to_be_bytes());

                let mut page_offset = LEAF_NODE_HEADER_SIZE;
                for pair in kv_pairs {
                    let key_bytes = pair.key.as_bytes();
                    let key_size: usize = key_bytes.len();
                    let value_bytes = pair.value.as_bytes();
                    let value_size: usize = value_bytes.len();

                    // write the key_size followed by the value_size
                    data[page_offset..page_offset + PTR_SIZE]
                        .clone_from_slice(&key_size.to_be_bytes());
                    page_offset += PTR_SIZE;

                    data[page_offset..page_offset + PTR_SIZE]
                        .clone_from_slice(&value_size.to_be_bytes());
                    page_offset += PTR_SIZE;

                    // write the key as bytes
                    data[page_offset..page_offset + key_size].clone_from_slice(key_bytes);
                    page_offset += key_size;

                    // write the value as bytes
                    data[page_offset..page_offset + value_size].clone_from_slice(value_bytes);
                    page_offset += value_size;
                }
            }
            NodeType::Unexpected => return Err(Error::UnexpectedError),
        }

        Ok(Page::new(data))
    }
}

/// Attempts to convert a slice to an array of a fixed size (PTR_SIZE),
/// and then return the BigEndian value of the byte array.
impl TryFrom<&[u8]> for Value {
    type Error = Error;

    fn try_from(arr: &[u8]) -> Result<Self, Self::Error> {
        if arr.len() > PTR_SIZE {
            return Err(Error::TryFromSliceError("Unexpected Error: Array recieved is larger than the maximum allowed size of: 4096B."));
        }

        let mut truncated_arr = [0u8; PTR_SIZE];
        for (i, item) in arr.iter().enumerate() {
            truncated_arr[i] = *item;
        }

        Ok(Value(usize::from_be_bytes(truncated_arr)))
    }
}

mod tests {
    #[allow(unused_imports)]
    use crate::error::Error;

    #[test]
    fn node_to_page_works_for_leaf_node() -> Result<(), Error> {
        use crate::node::Node;
        use crate::node_type::{KeyValuePair, NodeType};
        use crate::page::Page;
        use std::convert::TryFrom;

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

        assert_eq!(res.is_root, some_leaf.is_root);
        assert_eq!(res.node_type, some_leaf.node_type);
        assert_eq!(res.parent_offset, some_leaf.parent_offset);
        Ok(())
    }

    #[test]
    fn node_to_page_works_for_internal_node() -> Result<(), Error> {
        use crate::node::Node;
        use crate::node_type::{Key, NodeType, Offset};
        use crate::page::Page;
        use crate::page_layout::PAGE_SIZE;
        use std::convert::TryFrom;

        let internal_node = Node::new(
            NodeType::Internal(
                vec![
                    Offset(PAGE_SIZE),
                    Offset(PAGE_SIZE * 2),
                    Offset(PAGE_SIZE * 3),
                    Offset(PAGE_SIZE * 4),
                ],
                vec![
                    Key("foo bar".to_string()),
                    Key("lebron".to_string()),
                    Key("ariana".to_string()),
                ],
            ),
            true,
            None,
        );

        // Serialize data.
        let page = Page::try_from(&internal_node)?;
        // Deserialize back the page.
        let res = Node::try_from(page)?;

        assert_eq!(res.is_root, internal_node.is_root);
        assert_eq!(res.node_type, internal_node.node_type);
        assert_eq!(res.parent_offset, internal_node.parent_offset);
        Ok(())
    }
}

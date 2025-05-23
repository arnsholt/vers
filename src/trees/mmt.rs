//! Min-Max tree implementation as described by Navarro and Sadakane in
//! [Fully Functional Static and Dynamic Succinct Trees](https://dl.acm.org/doi/pdf/10.1145/2601073).
//!
//! It supports the operations `fwd_search` and `bwd_search` to search positions in
//! parenthesis-expressions that have a specified excess of opening or closing parentheses.
//! It completes the operations in O(log n) time, where n is the number of blocks in the tree,
//! so if it is used with O(log n) block size, it achieves the O(log log n) time complexity of
//! the paper.
//!
//! The Min-Max tree is a complete binary tree that stores the minimum and maximum relative
//! excess values of parenthesis expressions in its nodes. Since the tree is complete, it can be
//! stored linearly.

use crate::BitVec;
use std::cmp::max;
use std::num::NonZeroUsize;

/// A singular node in a binary min-max tree that is part of the [`BpTree`] data structure.
///
/// [`BpTree`]: crate::trees::bp::BpTree
#[derive(Debug, Clone, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct ExcessNode {
    /// excess from l..=r in the node [l, r]
    total: i64,

    /// minimum (relative) excess in the node [l, r]
    min: i64,

    /// maximum (relative) excess in the node [l, r]
    max: i64,
}

/// A binary min-max tree that is part of the [`BpTree`] data structure.
///
/// [`BpTree`]: crate::trees::bp::BpTree
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct MinMaxTree {
    nodes: Box<[ExcessNode]>,
}

impl MinMaxTree {
    pub(crate) fn excess_tree(bit_vec: &BitVec, block_size: usize) -> Self {
        if bit_vec.is_empty() {
            return Self::default();
        }

        let num_leaves = bit_vec.len().div_ceil(block_size);
        let num_internal_nodes = max(1, (1 << (num_leaves as f64).log2().ceil() as usize) - 1);

        let mut nodes = vec![ExcessNode::default(); num_leaves + num_internal_nodes];
        let mut total_excess = 0;
        let mut min_excess = i64::MAX;
        let mut max_excess = i64::MIN;

        // bottom up construction
        for i in 0..bit_vec.len() {
            if i > 0 && i % block_size == 0 {
                nodes[num_internal_nodes + i / block_size - 1] = ExcessNode {
                    total: total_excess,
                    min: min_excess,
                    max: max_excess,
                };
                total_excess = 0;
                min_excess = i64::MAX;
                max_excess = i64::MIN;
            }
            total_excess += if bit_vec.is_bit_set_unchecked(i) {
                1
            } else {
                -1
            };
            min_excess = min_excess.min(total_excess);
            max_excess = max_excess.max(total_excess);
        }
        nodes[num_internal_nodes + num_leaves - 1] = ExcessNode {
            total: total_excess,
            min: min_excess,
            max: max_excess,
        };

        let mut current_level_size = max(1, num_leaves.next_power_of_two() / 2);
        let mut current_level_start = num_internal_nodes - current_level_size;
        loop {
            for i in 0..current_level_size {
                let left_child_index = (current_level_start + i) * 2 + 1;
                let right_child_index = (current_level_start + i) * 2 + 2;

                if left_child_index < nodes.len() {
                    if right_child_index < nodes.len() {
                        let left_child = &nodes[left_child_index];
                        let right_child = &nodes[right_child_index];
                        nodes[current_level_start + i] = ExcessNode {
                            total: left_child.total + right_child.total,
                            min: left_child.min.min(left_child.total + right_child.min),
                            max: left_child.max.max(left_child.total + right_child.max),
                        };
                    } else {
                        nodes[current_level_start + i] = nodes[left_child_index].clone();
                    }
                }
            }

            // if this was the root level, break the loop
            if current_level_size == 1 {
                break;
            }

            current_level_size /= 2;
            current_level_start -= current_level_size;
        }

        Self {
            nodes: nodes.into_boxed_slice(),
        }
    }

    pub(crate) fn total_excess(&self, index: usize) -> i64 {
        self.nodes[index].total
    }

    pub(crate) fn min_excess(&self, index: usize) -> i64 {
        self.nodes[index].min
    }

    pub(crate) fn max_excess(&self, index: usize) -> i64 {
        self.nodes[index].max
    }

    pub(crate) fn parent(&self, index: NonZeroUsize) -> Option<usize> {
        if index.get() < self.nodes.len() {
            Some((index.get() - 1) / 2)
        } else {
            None
        }
    }

    /// Get the index of the left child of the node at `index` if it exists
    pub(crate) fn left_child(&self, index: usize) -> Option<NonZeroUsize> {
        if index * 2 + 1 < self.nodes.len() {
            NonZeroUsize::new(index * 2 + 1)
        } else {
            None
        }
    }

    /// Get the index of the right child of the node at `index` if it exists
    pub(crate) fn right_child(&self, index: usize) -> Option<NonZeroUsize> {
        if index * 2 + 2 < self.nodes.len() {
            NonZeroUsize::new(index * 2 + 2)
        } else {
            None
        }
    }

    /// Get the index of the right sibling of the node at `index` if it exists
    pub(crate) fn right_sibling(&self, index: NonZeroUsize) -> Option<NonZeroUsize> {
        if index.get() % 2 == 1 {
            if index.get() + 1 >= self.nodes.len() {
                None
            } else {
                index.checked_add(1)
            }
        } else {
            None
        }
    }

    /// Get the index of the left sibling of the node at `index` if it exists
    #[allow(clippy::unused_self)] // self is used for consistency with other methods
    pub(crate) fn left_sibling(&self, index: NonZeroUsize) -> Option<NonZeroUsize> {
        if index.get() % 2 == 0 {
            // index is at least 2
            NonZeroUsize::new(index.get() - 1)
        } else {
            None
        }
    }

    /// Check if the node at `index` is a left child, or would be if it existed
    #[allow(clippy::unused_self)] // self is used for consistency with other methods
    pub(crate) fn is_left_child(&self, index: NonZeroUsize) -> bool {
        index.get() % 2 == 1
    }

    /// Get the index of the first leaf node in the tree
    fn first_leaf(&self) -> usize {
        debug_assert!(!self.nodes.is_empty());
        match self.nodes.len() {
            2 => 1,
            _ => self.nodes.len().div_ceil(2).next_power_of_two() - 1,
        }
    }

    /// Check if the given node index is a leaf. A leaf for the purpose of this method is defined
    /// as a node in the last level of the tree. There may be other nodes without children in the
    /// tree, but they are not considered leaves.
    pub(crate) fn is_leaf(&self, index: usize) -> bool {
        index >= self.first_leaf()
    }

    /// Forward search for the leaf node that contains the next position with the given excess.
    /// The search only searches for the block, not the exact position.
    /// It further assumes that the beginning block does not contain the position, so the search
    /// will never return the starting block.
    ///
    /// # Parameters
    /// - `begin`: The index of the leaf block to start the search from (the first leaf is indexed with 0).
    /// - `relative_excess`: The excess to search for relative to the excess at the end of the block.
    ///   That is, if a query at index `i` seeks excess `x`, and between `i` and the end of the
    ///   block `j` there is excess `y`, then the relative excess is `x - y`.
    pub(crate) fn fwd_search(&self, begin: usize, relative_excess: i64) -> Option<(usize, i64)> {
        if begin + self.first_leaf() >= self.nodes.len() {
            return None;
        }

        self.do_fwd_upwards_search(
            NonZeroUsize::new(begin + self.first_leaf()).unwrap(),
            relative_excess,
        )
        .map(|(node, relative_excess)| (node.get() - self.first_leaf(), relative_excess))
    }

    /// Backward search for the leaf node that contains the closest position with the given excess.
    /// The search only searches for the block, not the exact position.
    /// It further assumes that the beginning block does not contain the position, so the search
    /// will never return the starting block.
    ///
    /// # Parameters
    /// - `begin`: The index of the leaf block to start the search from (the first leaf is indexed with 0).
    /// - `relative_excess`: The excess to search for relative to the excess at the end of the block.
    ///   That is, if a query at index `i` seeks excess `x`, and between `i` and the start of the
    ///   block `j` there is excess `y`, then the relative excess is `x - y`.
    pub(crate) fn bwd_search(&self, begin: usize, relative_excess: i64) -> Option<(usize, i64)> {
        if begin + self.first_leaf() >= self.nodes.len() {
            return None;
        }
        self.do_bwd_upwards_search(
            NonZeroUsize::new(begin + self.first_leaf()).unwrap(),
            relative_excess,
        )
        .map(|(node, relative_excess)| (node.get() - self.first_leaf(), relative_excess))
    }

    /// Search up the tree for the block that contains the relative excess. We assume that the
    /// relative excess is not within the range of the block that this method is called on.
    /// We assume the excess is relative to the end of the block.
    fn do_fwd_upwards_search(
        &self,
        node: NonZeroUsize,
        relative_excess: i64,
    ) -> Option<(NonZeroUsize, i64)> {
        debug_assert!(node.get() < self.nodes.len());

        // if this is a right node, we need to go up
        #[allow(clippy::if_not_else)] // handle the easy case first for readability
        if !self.is_left_child(node) {
            let parent = NonZeroUsize::new(self.parent(node).unwrap());
            if let Some(parent) = parent {
                self.do_fwd_upwards_search(parent, relative_excess)
            } else {
                // if parent is the root, there is no further node to the right of us, no result
                None
            }
        } else {
            let right_sibling = self.right_sibling(node);
            // if we have a right sibling, check whether it contains the excess
            if let Some(right_sibling) = right_sibling {
                // if it does, we can go down (relative excess is already relative to end of current block)
                if self.min_excess(right_sibling.get()) <= relative_excess
                    && relative_excess <= self.max_excess(right_sibling.get())
                {
                    self.do_fwd_downwards_search(right_sibling.get(), relative_excess)
                } else {
                    // go up from the right sibling, adjusting the relative excess to the end of the right sibling
                    let parent = NonZeroUsize::new(self.parent(node).unwrap());
                    if let Some(parent) = parent {
                        self.do_fwd_upwards_search(
                            parent,
                            relative_excess - self.total_excess(right_sibling.get()),
                        )
                    } else {
                        None
                    }
                }
            } else {
                // no right sibling, the tree ends here
                None
            }
        }
    }

    /// Search down the tree for the block that contains the relative excess. We assume that the
    /// relative excess is within the range of the block that this method is called on.
    /// We assume the excess is relative to the beginning of the block.
    fn do_fwd_downwards_search(
        &self,
        node: usize,
        relative_excess: i64,
    ) -> Option<(NonZeroUsize, i64)> {
        debug_assert!(node < self.nodes.len());

        // if we arrived at a leaf, we are done. Since we assume that the relative excess is within
        // the range of the block given to the method call, we can return the node.
        if self.is_leaf(node) {
            return NonZeroUsize::new(node).map(|node| (node, relative_excess));
        }

        let left_child = self.left_child(node);
        if let Some(left_child) = left_child {
            if self.min_excess(left_child.get()) <= relative_excess
                && relative_excess <= self.max_excess(left_child.get())
            {
                self.do_fwd_downwards_search(left_child.get(), relative_excess)
            } else {
                let right_child = self.right_child(node);
                if let Some(right_child) = right_child {
                    let relative_excess = relative_excess - self.total_excess(left_child.get());
                    if self.min_excess(right_child.get()) <= relative_excess
                        && relative_excess <= self.max_excess(right_child.get())
                    {
                        self.do_fwd_downwards_search(right_child.get(), relative_excess)
                    } else {
                        unreachable!();
                    }
                } else {
                    unreachable!();
                }
            }
        } else {
            unreachable!();
        }
    }

    /// Search up the tree for the block that contains the relative excess. We assume that the
    /// relative excess is not within the range of the block that this method is called on.
    /// We assume the excess is relative to the beginning of the block.
    fn do_bwd_upwards_search(
        &self,
        node: NonZeroUsize,
        relative_excess: i64,
    ) -> Option<(NonZeroUsize, i64)> {
        debug_assert!(node.get() < self.nodes.len());

        // if this is a left node, we need to go up
        if self.is_left_child(node) {
            let parent = NonZeroUsize::new(self.parent(node).unwrap());
            if let Some(parent) = parent {
                self.do_bwd_upwards_search(parent, relative_excess)
            } else {
                // if parent is the root, there is no further node to the left of us, no result
                None
            }
        } else {
            let left_sibling = self.left_sibling(node);
            // if we have a left sibling, check whether it contains the excess
            if let Some(left_sibling) = left_sibling {
                // if it does, we can go down (relative excess is already relative to start of current block)
                if (relative_excess + self.total_excess(left_sibling.get()) == 0)
                    || (self.min_excess(left_sibling.get())
                        <= relative_excess + self.total_excess(left_sibling.get())
                        && relative_excess + self.total_excess(left_sibling.get())
                            <= self.max_excess(left_sibling.get()))
                {
                    self.do_bwd_downwards_search(left_sibling.get(), relative_excess)
                } else {
                    // go up from the left sibling, adjusting the relative excess to the start of the left sibling
                    let parent = NonZeroUsize::new(self.parent(node).unwrap());
                    if let Some(parent) = parent {
                        self.do_bwd_upwards_search(
                            parent,
                            relative_excess + self.total_excess(left_sibling.get()),
                        )
                    } else {
                        None
                    }
                }
            } else {
                // no right sibling, the tree ends here
                None
            }
        }
    }

    /// Search down the tree for the block that contains the relative excess. We assume that the
    /// relative excess is within the range of the block that this method is called on.
    /// We assume the excess is relative to the end of the block.
    fn do_bwd_downwards_search(
        &self,
        node: usize,
        relative_excess: i64,
    ) -> Option<(NonZeroUsize, i64)> {
        debug_assert!(node < self.nodes.len());

        // if we arrived at a leaf, we are done. Since we assume that the relative excess is within
        // the range of the block given to the method call, we can return the node.
        if self.is_leaf(node) {
            return NonZeroUsize::new(node).map(|node| (node, relative_excess));
        }

        let right_child = self.right_child(node);
        if let Some(right_child) = right_child {
            if (relative_excess + self.total_excess(right_child.get()) == 0)
                || (self.min_excess(right_child.get())
                    <= relative_excess + self.total_excess(right_child.get())
                    && relative_excess + self.total_excess(right_child.get())
                        <= self.max_excess(right_child.get()))
            {
                self.do_bwd_downwards_search(right_child.get(), relative_excess)
            } else {
                let left_child = self.left_child(node);
                if let Some(left_child) = left_child {
                    let relative_excess = relative_excess + self.total_excess(right_child.get());
                    if (relative_excess + self.total_excess(left_child.get()) == 0)
                        || (self.min_excess(left_child.get())
                            <= relative_excess + self.total_excess(left_child.get())
                            && relative_excess + self.total_excess(left_child.get())
                                <= self.max_excess(left_child.get()))
                    {
                        self.do_bwd_downwards_search(left_child.get(), relative_excess)
                    } else {
                        unreachable!();
                    }
                } else {
                    unreachable!();
                }
            }
        } else {
            unreachable!();
        }
    }

    /// Returns the number of bytes used on the heap for this structure. This does not include
    /// allocated space that is not used (e.g. by the allocation behavior of `Vec`).
    #[must_use]
    pub fn heap_size(&self) -> usize {
        self.nodes.len() * size_of::<ExcessNode>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BitVec;

    #[test]
    fn test_simple_excess_tree() {
        #[rustfmt::skip]
        let bv = BitVec::from_bits(&[
            1, 1, 1, 0, 0, 1, 1, 1,
            0, 1, 0, 1, 1, 1, 0, 0,
            1, 0, 0, 1, 0, 0, 0, 0,
        ]);

        let tree = MinMaxTree::excess_tree(&bv, 8);

        // three internal nodes, three leaves
        assert_eq!(tree.nodes.len(), 6);

        // leaf nodes
        assert_eq!(tree.nodes[3].total, 4);
        assert_eq!(tree.nodes[3].min, 1);
        assert_eq!(tree.nodes[3].max, 4);

        assert_eq!(tree.nodes[4].total, 0);
        assert_eq!(tree.nodes[4].min, -1);
        assert_eq!(tree.nodes[4].max, 2);

        assert_eq!(tree.nodes[5].total, -4);
        assert_eq!(tree.nodes[5].min, -4);
        assert_eq!(tree.nodes[5].max, 1);

        // root node
        assert_eq!(tree.nodes[0].total, 0); // the tree should be balanced
        assert_eq!(tree.nodes[0].min, 0);
        assert_eq!(tree.nodes[0].max, 6);

        // left child of the root
        assert_eq!(tree.nodes[1].total, 4);
        assert_eq!(tree.nodes[1].min, 1);
        assert_eq!(tree.nodes[1].max, 6);

        // right child of the root
        assert_eq!(tree.nodes[2].total, -4);
        assert_eq!(tree.nodes[2].min, -4);
        assert_eq!(tree.nodes[2].max, 1);
    }

    #[test]
    fn test_empty_excess_tree() {
        let bv = BitVec::new();
        let tree = MinMaxTree::excess_tree(&bv, 8);

        assert_eq!(tree.nodes.len(), 0);
    }

    #[test]
    fn test_excess_tree_navigation() {
        // expected tree layout:
        //      0
        //    /  \
        //   1    2
        //   /\  /\
        //  3  4 5 6
        //  /\/\/\/\
        // 7 8 9 10 11 12 - -
        let bv = BitVec::from_bits(&[0; 48]);
        let tree = MinMaxTree::excess_tree(&bv, 8);

        assert_eq!(tree.nodes.len(), 13); // 6 leaves + 7 internal nodes

        // check root
        assert_eq!(tree.left_child(0), NonZeroUsize::new(1));
        assert_eq!(tree.right_child(0), NonZeroUsize::new(2));

        // check full nodes
        for node in 1..=5 {
            assert_eq!(tree.left_child(node), NonZeroUsize::new(node * 2 + 1));
            assert_eq!(tree.right_child(node), NonZeroUsize::new(node * 2 + 2));
        }

        // check obsolete node
        assert_eq!(tree.left_child(6), None);
        assert_eq!(tree.right_child(6), None);

        // check siblings of first level
        assert_eq!(tree.left_sibling(NonZeroUsize::new(1).unwrap()), None);
        assert_eq!(
            tree.right_sibling(NonZeroUsize::new(1).unwrap()),
            NonZeroUsize::new(2)
        );

        assert_eq!(
            tree.left_sibling(NonZeroUsize::new(2).unwrap()),
            NonZeroUsize::new(1)
        );
        assert_eq!(tree.right_sibling(NonZeroUsize::new(2).unwrap()), None);

        // check siblings of leaf nodes
        assert_eq!(tree.left_sibling(NonZeroUsize::new(7).unwrap()), None);
        assert_eq!(
            tree.right_sibling(NonZeroUsize::new(7).unwrap()),
            NonZeroUsize::new(8)
        );

        // leaves are not connected to each other because we don't need it for the search primitives
        assert_eq!(
            tree.left_sibling(NonZeroUsize::new(8).unwrap()),
            NonZeroUsize::new(7)
        );
        assert_eq!(tree.right_sibling(NonZeroUsize::new(8).unwrap()), None);

        // check siblings of non-existent node
        assert_eq!(tree.left_sibling(NonZeroUsize::new(13).unwrap()), None);
        assert_eq!(tree.right_sibling(NonZeroUsize::new(13).unwrap()), None);

        // check parent of leaf nodes
        assert_eq!(tree.parent(NonZeroUsize::new(7).unwrap()), Some(3));
        assert_eq!(tree.parent(NonZeroUsize::new(8).unwrap()), Some(3));
        assert_eq!(tree.parent(NonZeroUsize::new(9).unwrap()), Some(4));
        assert_eq!(tree.parent(NonZeroUsize::new(10).unwrap()), Some(4));

        // check parent of first level nodes
        assert_eq!(tree.parent(NonZeroUsize::new(1).unwrap()), Some(0));
        assert_eq!(tree.parent(NonZeroUsize::new(2).unwrap()), Some(0));

        // check parent of non-existent node
        assert_eq!(tree.parent(NonZeroUsize::new(13).unwrap()), None);
    }

    #[test]
    fn test_empty_tree_navigation() {
        let bv = BitVec::new();
        let tree = MinMaxTree::excess_tree(&bv, 8);

        assert_eq!(tree.nodes.len(), 0);

        assert_eq!(tree.left_child(0), None);
        assert_eq!(tree.right_child(0), None);
        assert_eq!(tree.left_sibling(NonZeroUsize::new(1).unwrap()), None);
        assert_eq!(tree.right_sibling(NonZeroUsize::new(1).unwrap()), None);
        assert_eq!(tree.parent(NonZeroUsize::new(1).unwrap()), None);
    }

    #[test]
    fn test_simple_fwd_search() {
        #[rustfmt::skip]
        let bv = BitVec::from_bits(&[
            1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let tree = MinMaxTree::excess_tree(&bv, 8);

        assert_eq!(tree.nodes.len(), 6);
        assert_eq!(tree.total_excess(0), 0); // tree should be balanced

        // fwd search from the first block (index 3)
        for i in 0..8 {
            let block = tree.fwd_search(0, -i - 1);
            assert!(block.is_some(), "block for query {} not found", i);
            assert_eq!(
                block.unwrap().0,
                2,
                "query {} did not return block 2 but {}",
                i,
                block.unwrap().0
            );
        }

        // fwd search from the second block (index 4), searching the closing parenthesis of the
        // enclosing parentheses pair of the first position in the block (i.e. relative excess -1
        // from the end of the block)
        let block = tree.fwd_search(1, -1);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 2);

        // fwd search a position that is not in the tree, e.g. -9 from the first block
        let block = tree.fwd_search(0, -9);
        assert!(block.is_none());
    }

    #[test]
    fn test_fwd_search_with_multiple_blocks() {
        #[rustfmt::skip]
        let bv = BitVec::from_bits(&[
            1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 0, 0, 0,
            1, 1, 1, 1, 1, 0, 0, 0,
            0, 1, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let tree = MinMaxTree::excess_tree(&bv, 8);

        assert_eq!(tree.nodes.len(), 12);
        assert_eq!(tree.total_excess(0), 0); // tree should be balanced

        // fwd search something where the result is not the last node
        let block = tree.fwd_search(2, 1);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 3);

        let block = tree.fwd_search(1, -2);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 3);
    }

    #[test]
    fn test_fwd_search_relative_offsets() {
        #[rustfmt::skip]
        let bv = BitVec::from_bits(&[
            1, 1, 1, 0,
            1, 0, 1, 1, // excess 2
            1, 0, 1, 0, // min excess 0, max excess 1
            0, 0, 0, 0,
        ]);

        let tree = MinMaxTree::excess_tree(&bv, 4);

        // if the relative excess is calculated wrong, it will find block 5, since -1 + 2 = 1,
        // which is the max excess in block 5. Correct calculation of relative excess is -1 - 2 = -3
        let block = tree.fwd_search(0, -1);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 3);
    }

    #[test]
    fn test_simple_bwd_search() {
        #[rustfmt::skip]
        let bv = BitVec::from_bits(&[
            1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let tree = MinMaxTree::excess_tree(&bv, 8);

        assert_eq!(tree.nodes.len(), 6);
        assert_eq!(tree.total_excess(0), 0); // tree should be balanced

        // bwd search from the last block (index 5)
        for i in 0..8 {
            let block = tree.bwd_search(2, -i - 1);
            assert!(block.is_some(), "block for query {} not found", i);
            assert_eq!(
                block.unwrap().0,
                0,
                "query {} did not return block 0 but {}",
                i,
                block.unwrap().0
            );
        }

        // bwd search from the second block, searching the opening parenthesis of the
        // enclosing parentheses pair of the last position in the block (i.e. relative excess -1
        // from the start of the block)
        let block = tree.bwd_search(1, -1);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 0);

        // bwd search a position that is not in the tree, e.g. -9 from the last block
        let block = tree.fwd_search(2, -9);
        assert!(block.is_none());
    }

    #[test]
    fn test_bwd_search_with_multiple_blocks() {
        #[rustfmt::skip]
        let bv = BitVec::from_bits(&[
            1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 0, 0, 0,
            1, 1, 1, 1, 1, 0, 0, 0,
            0, 1, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let tree = MinMaxTree::excess_tree(&bv, 8);

        assert_eq!(tree.nodes.len(), 12);
        assert_eq!(tree.total_excess(0), 0); // tree should be balanced

        // bwd search something where the result is not the first node
        let block = tree.bwd_search(3, -1);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 2);

        let block = tree.bwd_search(3, -3);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 1);
    }

    #[test]
    fn test_bwd_search_relative_offsets() {
        #[rustfmt::skip]
        let bv = BitVec::from_bits(&[
            1, 1, 1, 0,
            1, 0, 1, 1, // excess 2
            1, 0, 1, 0, // min excess 0, max excess 1
            0, 0, 0, 0,
        ]);

        let tree = MinMaxTree::excess_tree(&bv, 4);

        let block = tree.bwd_search(3, -4);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 0);
    }

    #[test]
    fn test_incomplete_block() {
        #[rustfmt::skip]
        let bv = BitVec::from_bits(&[
            1, 1, 1, 1, 1, 1, 1, 0,
            0, 0, 0, 0, 0, 0
        ]);

        let tree = MinMaxTree::excess_tree(&bv, 8);

        assert_eq!(tree.nodes.len(), 3);

        let block = tree.fwd_search(0, -1);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 1);

        let block = tree.fwd_search(0, -2);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 1);
    }

    #[test]
    fn test_single_block() {
        let bv = BitVec::from_bits(&[1, 1, 1, 1, 0, 0, 0, 0]);

        let tree = MinMaxTree::excess_tree(&bv, 8);

        assert_eq!(tree.nodes.len(), 2);
    }

    #[test]
    fn test_leaf_calculation() {
        // test small tree
        let bv = BitVec::from_bits(&vec![0; 1000]);
        let tree = MinMaxTree::excess_tree(&bv, 1200);
        assert_eq!(tree.first_leaf(), 1);

        // test very large tree
        let bv = BitVec::from_bits(&vec![0; 1000]);
        let tree = MinMaxTree::excess_tree(&bv, 4);

        assert_eq!(tree.first_leaf(), 255)
    }

    #[test]
    fn test_relative_excess() {
        // test a tree with 3 layers and different downwards traversals
        #[rustfmt::skip]
        let bv = BitVec::from_bits(&[
            1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        let tree = MinMaxTree::excess_tree(&bv, 8);

        let block = tree.fwd_search(0, -6);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 5);

        // test if the relative excess is still correct (relative to the start of the fifth leaf)
        assert_eq!(block.unwrap().1, -6);

        let block = tree.bwd_search(5, -6);
        assert!(block.is_some());
        assert_eq!(block.unwrap().0, 0);
        assert_eq!(block.unwrap().1, -6);
    }
}

use seahash::SeaHasher;
use std::hash::BuildHasherDefault;
use std::rc::{Rc, Weak};
use weak_table::WeakHashSet;

use crate::automaton::*;

/// Fast hasher used for NdTreeNodes.
pub type NodeHasher = BuildHasherDefault<SeaHasher>;

/// A cached NdTreeNode.
pub type NdCachedNode<C, D> = Rc<NdTreeNode<C, D>>;

/// A cache of NdTreeNodes.
#[derive(Debug, Clone, Default)]
pub struct NdTreeCache<C: CellType, D: Dim> {
    /// A HashSet of all of the nodes.
    nodes: WeakHashSet<Weak<NdTreeNode<C, D>>, NodeHasher>,
    /// A cache of empty nodes at various layers.
    ///
    /// The element at index N is the empty node at layer N-1.
    empty_nodes: Vec<NdCachedNode<C, D>>,
}

impl<C: CellType, D: Dim> NdTreeCache<C, D> {
    /// Returns a new empty NdTreeNode cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the cached node with the given branches, creating it if it does
    /// not exist.
    pub fn get_node(&mut self, branches: Vec<NdTreeBranch<C, D>>) -> NdCachedNode<C, D> {
        // Create an NdBaseTreeNode (cheaper than a full NdTreeNode) for HashSet
        // lookup.
        let base_node = NdBaseTreeNode::from(branches);
        // If the node is already in the cache, return the one from the cache.
        if let Some(existing_node) = self.nodes.get(&base_node) {
            return existing_node;
        }
        // Otherwise, make it a full node, add it to the cache, and return it.
        let ret = Rc::new(NdTreeNode::from(base_node));
        self.nodes.insert(ret.clone());
        ret
    }
    /// Returns the NdTreeNode at the given layer with all default cells.
    pub fn get_empty_node(&mut self, layer: usize) -> NdCachedNode<C, D> {
        if let Some(ret) = self.empty_nodes.get(layer - 1) {
            // Cache hit
            ret.clone()
        } else {
            // Cache miss
            let branches = vec![self.get_empty_branch(layer - 1); D::TREE_BRANCHES];
            let ret = self.get_node(branches);
            // All lower entries in the cache have been filled by the recursive
            // call.
            self.empty_nodes.push(ret.clone());
            ret
        }
    }
    /// Returns the NdTreeBranch containing a node at the given layer with all
    /// default cells (or just an NdTreeBranch::Leaf of the default cell state).
    pub fn get_empty_branch(&mut self, layer: usize) -> NdTreeBranch<C, D> {
        match layer {
            0 => NdTreeBranch::Leaf(C::default()),
            _ => NdTreeBranch::Node(self.get_empty_node(layer)),
        }
    }
    /// Returns a cached node, using a function to generate each branch.
    pub fn get_node_from_fn<F: FnMut(&mut Self, ByteVec<D>) -> NdTreeBranch<C, D>>(
        &mut self,
        mut generator: F,
    ) -> NdCachedNode<C, D> {
        let branches = (0..D::TREE_BRANCHES)
            .map(ByteVec::from_array_idx)
            .map(|branch_idx| generator(self, branch_idx))
            .collect();
        self.get_node(branches)
    }
    /// Returns a cached node, using a function of the cell position to generate
    /// each cell state. This can only be used for relatively small nodes, since
    /// an IVec is used for the position vector.
    pub fn get_small_node_from_cell_fn<F: Fn(IVec<D>) -> C>(
        &mut self,
        layer: usize,
        offset: IVec<D>,
        generator: &F,
    ) -> NdCachedNode<C, D> {
        self.get_node_from_fn(|cache, branch_idx| {
            let branch_offset: IVec<D> = branch_idx.branch_offset(layer);
            let total_offset = &offset + branch_offset;
            if layer == 1 {
                NdTreeBranch::Leaf(generator(total_offset))
            } else {
                NdTreeBranch::Node(cache.get_small_node_from_cell_fn(
                    layer - 1,
                    total_offset,
                    generator,
                ))
            }
        })
    }
}

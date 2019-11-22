use seahash::SeaHasher;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::ops::Index;
use std::rc::{Rc, Weak};

use super::cache::*;
use crate::automaton::Rule;

/// A cached NdTreeNode.
pub type NdCachedTree<T, D, R> = Rc<NdTreeNode<T, D, R>>;

/// An NdTreeNode's child.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NdTreeBranch<T: CellType, D: Dim, R: Rule<T, D>> {
    /// All cells within this branch are the same cell state.
    Leaf(T),

    /// An interned subnode.
    Node(NdCachedTree<T, D, R>),
}

impl<T: CellType, D: Dim, R: Rule<T, D>> Default for NdTreeBranch<T, D, R> {
    fn default() -> Self {
        Self::Leaf(T::default())
    }
}

impl<T: CellType, D: Dim, R: Rule<T, D>> NdTreeBranch<T, D, R> {
    fn empty(cache: &mut NdTreeCache<T, D, R>, layer: usize) -> Self {
        if layer == 0 {
            Self::Leaf(T::default())
        } else {
            Self::Node(NdTreeNode::empty(cache, layer))
        }
    }
    fn is_empty(&self) -> bool {
        match self {
            NdTreeBranch::Leaf(cell_state) => *cell_state == T::default(),
            NdTreeBranch::Node(node) => node.is_empty(),
        }
    }
    fn layer(&self) -> usize {
        match self {
            NdTreeBranch::Leaf(_) => 0,
            NdTreeBranch::Node(node) => node.layer(),
        }
    }
}

/// A single node in the NdTree, which contains information about its layer
/// (base-2 logarithm of hypercube side length) and its children.
#[derive(Clone)]
pub struct NdTreeNode<T: CellType, D: Dim, R: Rule<T, D>> {
    /// The "layer" of this node (base-2 logarithm of hypercube side length).
    layer: usize,

    /// The branches of this node, stored as a flattened 2^d hypercube of nodes
    /// one layer lower.
    ///
    /// If layer == 1, then all of these must be `NdTreeBranch::Leaf`s. If layer
    /// > 1, then all of these must be `NdTreeBranch::Branch`es.
    ///
    /// Until rust-lang #44580 (RFC 2000) is resolved, there's no way to use
    /// D::NDIM as the array size. It might be worth implementing a custom
    /// unsafe type for this, but at the time of writing such an optimization
    /// would be entirely premature.
    branches: Vec<NdTreeBranch<T, D, R>>,

    /// This node's hash, based solely on the hashes of its branches.
    hash_code: u64,

    /// The LayerCache containing this node (which also tells us this node's
    /// layer).
    cache: RefCell<NdLayerCache<T, D, R>>,

    /// The population of this node.
    population: usize,

    /// The future inner nodes of this node, simulated for 2**(index)
    /// generations.
    futures: Vec<Option<NdCachedNode<T, D, R>>>,
}

impl<T: CellType, D: Dim, R: Rule<T, D>> Debug for NdTreeNode<T, D, R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NdTreeNode {{ branches: {:?} }}", self.branches)
    }
}

impl<T: CellType, D: Dim, R: Rule<T, D>> NdTreeNode<T, D, R> {
    /// Constructs a new empty NdTreeNode at a given layer.
    pub fn empty(cache: &mut NdLayerCache<T, D, R>) -> NdCachedTree<T, D, R> {
        if layer == 0 {
            panic!("Cannot construct NdTreeNode at layer 0.");
        }
        let branches = vec![NdTreeBranch::empty(cache, layer - 1); Self::BRANCHES];
        Self::with_branches(cache, branches)
    }
    /// Constructs a new NdTreeNode at a given layer and with the given branches.
    pub fn with_branches(
        cache: &mut NdTreeCache<T, D, R>,
        branches: Vec<NdTreeBranch<T, D, R>>,
    ) -> NdCachedTree<T, D, R> {
        // Check that there are the right number of branches.
        if branches.len() != Self::BRANCHES {
            panic!(
                "NdTreeNode of {} dimensions must have {} branches; got {:?} instead.",
                D::NDIM,
                Self::BRANCHES,
                branches
            );
        }
        // Check that the branches are all at the same layer (and infer this
        // node's layer based on them).
        let mut branch_layers = branches.iter().map(|branch| match branch {
            NdTreeBranch::Leaf(_) => 0,
            NdTreeBranch::Node(node) => node.layer,
        });
        let layer = branch_layers.next().unwrap() + 1;
        if !branch_layers.all(|branch_layer| branch_layer == layer - 1) {
            panic!("NdTreeNode branches have different layers: {:?}", branches);
        }
        // Compute the hash code.
        let mut hasher = SeaHasher::new();
        layer.hash(&mut hasher);
        branches.hash(&mut hasher);
        // Construct the node.
        let node = Self {
            layer,
            branches,
            hash_code: hasher.finish(),
            phantom: PhantomData,
        };
        node.intern(cache)
    }
    /// Checks whether an equivalent node is present in the cache. If it is,
    /// destroys this one and returns the equivalent node from the cache; if
    /// not, adds this node to the cache and returns it.
    fn intern(self, cache: &mut NdTreeCache<T, D, R>) -> NdCachedTree<T, D, R> {
        // Construct the Rc and add it to the cache.
        cache.get_key(&self).clone().unwrap_or_else(|| {
            let ret = Rc::new(self);
            cache.insert(ret.clone(), Default::default());
            ret
        })
    }

    pub fn layer(&self) -> usize {
        self.layer
    }
    pub fn branches(&self) -> &Vec<NdTreeBranch<T, D, R>> {
        &self.branches
    }
    pub fn hash_code(&self) -> u64 {
        self.hash_code
    }

    /// Returns the length of a single side of the hypersquare contained in this
    /// subtree.
    pub fn len(&self) -> usize {
        // layer = 1 => len = 2
        // layer = 2 => len = 4
        // layer = 3 => len = 8
        // etc.
        1 << self.layer
    }
    /// Returns the bounding rectangle for this node, with the origin as the
    /// lower bound.
    pub fn rect(&self) -> NdRect<D> {
        Self::rect_at_layer(self.layer)
    }
    /// Returns the bounding rectangle for a node at the given layer, with the
    /// origin as the lower bound.
    pub fn rect_at_layer(layer: usize) -> NdRect<D> {
        NdRect::span(NdVec::origin(), NdVec::origin() + ((1 << layer) - 1))
    }
    /// Returns true iff there are no non-default cells inside this node.
    pub fn is_empty(&self) -> bool {
        self.branches.iter().all(NdTreeBranch::is_empty)
    }

    /// The number of branches for this many dimensions (2^d).
    pub const BRANCHES: usize = 1 << D::NDIM;
    /// The bitmask for branch indices.
    const BRANCH_IDX_BITMASK: usize = Self::BRANCHES - 1;
    /// Computes the "branch index" corresponding to this node's child
    /// containing the given position.
    ///
    /// Each nth layer corresponds to the nth bit of each axis, which can either
    /// be 0 or 1. The "branch index" is a number in 0..(2 ** d) composed
    /// from these bits; each bit in the branch index is taken from a different
    /// axis. It's like a bitwise NdVec.
    fn branch_idx(&self, pos: NdVec<D>) -> usize {
        let mut ret = 0;
        for ax in D::axes() {
            ret <<= 1;
            ret |= (pos[ax] as usize >> (self.layer - 1)) & 1;
        }
        ret
    }
    /// Computes the vector offset for the given branch of this node.
    fn branch_offset(&self, branch_idx: usize) -> NdVec<D> {
        Self::branch_offset_at_layer(self.layer, branch_idx)
    }
    /// Computes the vector offset for the given branch of a node at the given
    /// layer.
    pub fn branch_offset_at_layer(layer: usize, branch_idx: usize) -> NdVec<D> {
        let mut ret = NdVec::origin();
        let halfway: isize = 1 << (layer - 1);
        for ax in D::axes() {
            // If the current bit of the branch index is 1, add half of the
            // length of this node to the corresponding axis in the result.
            let axis_bit_idx = D::NDIM - 1 - ax as usize;
            let axis_bit = (branch_idx as isize >> axis_bit_idx) & 1;
            ret[ax] += halfway * axis_bit;
        }
        ret
    }

    /// "Zooms out" of the current tree by a factor of two; returns a new
    /// NdCachedTree with the contents of this one centered in an empty
    /// grid.
    pub fn expand_centered(&self, cache: &mut NdTreeCache<T, D, R>) -> NdCachedTree<T, D, R> {
        let new_branches = self
            .branches
            .iter()
            .enumerate()
            .map(|(branch_idx, old_branch)| {
                // Create a new node with this node's Nth branch in the opposite
                // corner. (Bitwise XOR of branch index produces the branch index of
                // the opposite corner.) E.g. This node's northwest branch is placed
                // in the southeast branch of a new node, which will be in the
                // northwest branch of the result.
                let mut inner_branches =
                    vec![NdTreeBranch::empty(cache, self.layer - 1); Self::BRANCHES];
                inner_branches[branch_idx ^ Self::BRANCH_IDX_BITMASK] = old_branch.clone();
                NdTreeBranch::Node(Self::with_branches(cache, inner_branches))
            })
            .collect();
        NdTreeNode::with_branches(cache, new_branches)
    }
    /// "Zooms in" to the current tree as much as possible without losing
    /// non-empty cells; returns a new NdCachedTree with the contents of this one.
    pub fn contract_centered(&self, cache: &mut NdTreeCache<T, D, R>) -> NdCachedTree<T, D, R> {
        let mut ret = self.clone().intern(cache);
        while ret.layer > 1 && self.population(cache) == ret.get_inner(cache).population(cache) {
            ret = ret.get_inner(cache);
        }
        ret
    }

    /// Returns the cell value at the given position, modulo the node size.
    pub fn get_cell(&self, pos: NdVec<D>) -> T {
        match &self.branches[self.branch_idx(pos)] {
            NdTreeBranch::Leaf(cell_state) => *cell_state,
            NdTreeBranch::Node(node) => node.get_cell(pos),
        }
    }
    /// Constructs a new node with the cell at the given position, modulo the node
    /// size, having the given value.
    pub fn set_cell(
        &self,
        cache: &mut NdTreeCache<T, D, R>,
        pos: NdVec<D>,
        cell_state: T,
    ) -> NdCachedTree<T, D, R> {
        let mut new_branches = self.branches.clone();
        // Get the branch containing the given cell.
        let branch = &mut new_branches[self.branch_idx(pos)];
        match branch {
            // The branch is a single cell, so set that cell.
            NdTreeBranch::Leaf(old_cell_state) => *old_cell_state = cell_state,
            // The branch is a node, so recurse on that node.
            NdTreeBranch::Node(node) => *node = node.set_cell(cache, pos, cell_state),
        }
        Self::with_branches(cache, new_branches)
    }

    fn get_inner(&self, cache: &mut NdTreeCache<T, D, R>) -> NdCachedTree<T, D, R> {
        self.get_subtree(
            cache,
            self.layer - 1,
            NdVec::origin() + self.len() as isize / 4,
        )
    }
    /// Constructs a new node at the given layer whose lower bound is the given
    /// offset. This operation may be expensive if coordinates of the offset
    /// have prime factors other than 2.
    pub fn get_subtree(
        &self,
        cache: &mut NdTreeCache<T, D, R>,
        layer: usize,
        offset: NdVec<D>,
    ) -> NdCachedTree<T, D, R> {
        if layer == 0 {
            panic!("Cannot get subtree at layer 0");
        }
        if let NdTreeBranch::Node(node) = self.get_subtree_branch(cache, layer, offset) {
            node
        } else {
            panic!("Requested subtree at layer {}, but got single cell", layer);
        }
    }
    /// Does the same thing as get_subtree(), but returns an NdTreeBranch
    /// instead of an NdTreeNode (and thus is able to return a single cell).
    pub fn get_subtree_branch(
        &self,
        cache: &mut NdTreeCache<T, D, R>,
        layer: usize,
        offset: NdVec<D>,
    ) -> NdTreeBranch<T, D, R> {
        let result_rect = Self::rect_at_layer(layer) + offset;
        // Check bounds.
        if !self.rect().contains(result_rect) {
            panic!(
                "Subtree {{ layer: {}, offset: {:?} }} out of bounds for layer {}",
                layer, offset, self.layer
            );
        }
        // If it fits within this node, and it's the same layer/size as this
        // node, then it's exactly the same as this node.
        if layer == self.layer {
            return NdTreeBranch::Node(self.clone().intern(cache));
        }
        // Check whether the result is a subtree of a single branch of this
        // node.
        let min_branch_idx = self.branch_idx(result_rect.min());
        let max_branch_idx = self.branch_idx(result_rect.max());
        if min_branch_idx == max_branch_idx {
            // If it is, just delegate to that branch.
            let branch_idx = min_branch_idx;
            match &self.branches[branch_idx] {
                NdTreeBranch::Leaf(cell_state) => NdTreeBranch::Leaf(*cell_state),
                NdTreeBranch::Node(node) => {
                    node.get_subtree_branch(cache, layer, offset - self.branch_offset(branch_idx))
                }
            }
        } else {
            // If it isn't, then divide and conquer.
            let mut new_branches = Vec::with_capacity(Self::BRANCHES);
            for branch_idx in 0..Self::BRANCHES {
                new_branches.push(self.get_subtree_branch(
                    cache,
                    layer - 1,
                    offset + Self::branch_offset_at_layer(layer, branch_idx),
                ));
            }
            NdTreeBranch::Node(Self::with_branches(cache, new_branches))
        }
    }

    /// Returns the minimum layer that can compute the states of its inner cells
    /// after one generation using the given transition function.
    ///
    /// If a rule has a neighborhood with a high radius, then the radius of the
    /// node needs to be at least twice the neighborhood's radius in order to
    /// simulate its "inner node" (the node one layer down, centered on this
    /// one) for a single generation.
    ///
    /// More formally, a node at layer `L` can simulate any automaton with a
    /// radius `r` if `r <= 2**L / 4`. An exception is made for `r = 0`, which
    /// requires layer 2 rather than layer 0 or 1.
    pub fn min_sim_layer<R: Rule<T, D>>(rule: &R) -> usize {
        let r = rule.radius();
        let mut min_layer = 2;
        while r > (1 << min_layer) / 4 {
            min_layer += 1;
        }
        min_layer
    }

    /// Returns the base-2 log of the maximum number of generations for which a
    /// node at the given layer can compute the states of its inner cells.
    ///
    /// In other words: A node at a given layer can only simulate a limited
    /// number of generations before it runs out of information. This function
    /// returns the base-2 log of that number of generations.
    ///
    /// In general, a node at layer `L` can simulate any automaton with a radius
    /// `r` for `2 ** (L - r - 2)` generations, so this function returns `L - r
    /// - 2`. In the case where this value is negative, it returns None instead.
    pub fn max_gen_exp_at_layer<R: Rule<T, D>>(layer: usize, rule: &R) -> Option<usize> {
        let min_sim_layer = Self::min_sim_layer(rule);
        if layer >= min_sim_layer {
            Some(layer - min_sim_layer)
        } else {
            None
        }
    }

    /// Returns the base-2 log of the maximum number of generations for this
    /// node can compute the states of its inner cells.
    ///
    /// See NdTreeNode::max_sim_gen_exponent_at_layer() for more information.
    pub fn max_gen_exp<R: Rule<T, D>>(&self, rule: &R) -> Option<usize> {
        Self::max_gen_exp_at_layer(self.layer, rule)
    }

    pub fn get_non_default(
        &self,
        cache: &mut NdTreeCache<T, D, R>,
        offset: NdVec<D>,
    ) -> Vec<NdVec<D>> {
        let mut ret = vec![];
        if self.population(cache) != 0 {
            for (branch_idx, branch) in self.branches.iter().enumerate() {
                let branch_offset = offset + self.branch_offset(branch_idx);
                match branch {
                    NdTreeBranch::Leaf(cell_state) => {
                        if *cell_state != T::default() {
                            ret.push(branch_offset);
                        }
                    }
                    NdTreeBranch::Node(node) => {
                        ret.extend(node.get_non_default(cache, branch_offset));
                    }
                }
            }
        }
        ret
    }
}

impl<T: CellType, D: Dim, R: Rule<T, D>> Index<NdVec<D>> for NdTreeNode<T, D, R> {
    type Output = T;
    fn index(&self, pos: NdVec<D>) -> &T {
        match &self.branches[self.branch_idx(pos)] {
            NdTreeBranch::Leaf(cell_state) => cell_state,
            NdTreeBranch::Node(node) => &node[pos],
        }
    }
}

impl<T: CellType, D: Dim, R: Rule<T, D>> Eq for NdTreeNode<T, D, R> {}
impl<T: CellType, D: Dim, R: Rule<T, D>> PartialEq for NdTreeNode<T, D, R> {
    fn eq(&self, rhs: &Self) -> bool {
        // Check for pointer equality (very fast; guarantees true).
        std::ptr::eq(self, rhs)
            // If that fails, check hash codes (very fast; guarantees false).
            || (self.hash_code() == rhs.hash_code()
                // If neither of those worked, we have to check the hard way.
                && self.layer() == rhs.layer()
                && self.branches() == rhs.branches())
    }
}

impl<T: CellType, D: Dim, R: Rule<T, D>> Hash for NdTreeNode<T, D, R> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        // We already cached our own hash; just rehash that if you want to.
        self.hash_code().hash(hasher);
    }
}
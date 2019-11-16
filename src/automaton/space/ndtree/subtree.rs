use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NdSubTree<T: CellType, D: Dim> {
    node: NdTreeNodeRef<T, D>,
}

/// An interned NdTreeNode.
pub type NdTreeNodeRef<T, D> = Rc<NdTreeNode<T, D>>;

/// A single node in the NdTree, which contains information about its layer
/// (base-2 logarithm of hypercube side length) and its children.
#[derive(Debug, Clone)]
// TODO: custom Debug implementation
pub struct NdTreeNode<T: CellType, D: Dim> {
    /// The "layer" of this node (base-2 logarithm of hypercube side length).
    pub(super) layer: usize,

    /// The child of this node, which is either a single cell state or a 2^d
    /// hypercube of nodes.
    ///
    /// If layer == 0, then it must be a single cell state.
    pub(super) child: NdTreeChild<T, D>,

    pub(super) hash_code: u64,

    pub(super) tree_set: NdTreeCache<T, D>,

    phantom: PhantomData<D>,
}

/// An NdTreeNode's child.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NdTreeChild<T: CellType, D: Dim> {
    /// All cells within this node are the same cell state.
    Leaf(T),

    /// A 2^d hypercube of interned subnodes.
    ///
    /// Until rust-lang #44580 (RFC 2000) is resolved, there's no way to use
    /// D::NDIM as the array size. It might be worth implementing a custom
    /// unsafe type for this, but at the time of writing such an optimization
    /// would be entirely premature.
    Branch(Vec<NdSubTree<T, D>>),
}

impl<T: CellType, D: Dim> Default for NdSubTree<T, D> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: CellType, D: Dim> NdSubTree<T, D> {
    /// Create a new empty N-dimensional tree stucture with an empty node cache.
    pub fn new() -> Self {
        let tree_set = NdTreeCache::default();
        let node = NdTreeNode::empty(tree_set, 0).intern();
        Self { node }
    }
}

impl<T: CellType, D: Dim> NdTreeNode<T, D> {
    /// Constructs a new empty NdTreeNode at a given layer.
    fn empty(tree_set: NdTreeCache<T, D>, layer: usize) -> Self {
        Self::with_child(tree_set, layer, NdTreeChild::default())
    }
    /// Constructs a new NdTreeNode at a given layer and with a given child.
    fn with_child(tree_set: NdTreeCache<T, D>, layer: usize, child: NdTreeChild<T, D>) -> Self {
        let mut hasher = SeaHasher::new();
        layer.hash(&mut hasher);
        child.hash(&mut hasher);
        let ret = Self {
            layer,
            child,
            hash_code: hasher.finish(),
            tree_set,
            phantom: PhantomData,
        };
        ret
    }
    /// Checks whether an equivalent node is present in the cache. If it is,
    /// destroys this one and returns the equivalent node from the cache; if
    /// not, adds this node to the cache and returns it.
    fn intern(self) -> NdTreeNodeRef<T, D> {
        let existing_node = self.tree_set.borrow().get(&self).clone();
        existing_node.unwrap_or_else(|| {
            let ret = Rc::new(self);
            ret.tree_set.borrow_mut().insert(ret.clone());
            ret
        })
    }

    // fn get_subtree(&self, pos: NdVec<D>, layer: usize) -> NdSubTree<T, D> {
    //     match &self.child {
    //         NdTreeChild::Leaf(cell_state) => NdSubTree {
    //             node: Self::with_child(self.tree_set, layer, self.child).intern(),
    //         },
    //         NdTreeChild::Branch(branches) => panic!(),
    //     }
    // }

    // pub fn get_subtree_inner(&self, pos: NdVec<D>, layer: usize) -> T {}

    // pub fn get(&self, pos: NdVec<D>) -> T {
    //     match &self.child {
    //         NdTreeChild::Leaf(cell_state) => *cell_state,
    //         NdTreeChild::Branch(branches) => branches[pos.branch_index_top()].node.get_inner(pos),
    //     }
    // }

    // fn get_inner(&self, pos: NdVec<D>) -> T {
    //     match &self.child {
    //         NdTreeChild::Leaf(cell_state) => *cell_state,
    //         NdTreeChild::Branch(branches) => {
    //             branches[pos.branch_index(self.layer)].node.get_inner(pos)
    //         }
    //     }
    // }
}

impl<T: CellType, D: Dim> Default for NdTreeChild<T, D> {
    fn default() -> Self {
        Self::Leaf(T::default())
    }
}
//! Basic N-dimensional arrays for smaller patterns.

use num::traits::Pow;
use num::ToPrimitive;
use std::ops::{Index, IndexMut};

use super::*;

/// A basic N-dimensional array, implemented using a flat Vec<T>.
pub struct NdArray<T, D: Dim> {
    size: UVec<D>,
    data: Vec<T>,
}

// Create an NdArray from the cells in an NdTreeNode.
impl<C: CellType, D: Dim> From<&NdTreeNode<C, D>> for NdArray<C, D> {
    fn from(node: &NdTreeNode<C, D>) -> Self {
        let count = node
            .len()
            .pow(D::NDIM)
            .to_usize()
            .expect("Cannot make NdArray using such a large node");
        let size = UVec::repeat(node.len().to_usize().unwrap());

        let mut data = Vec::with_capacity(count);
        for idx in 0..count {
            data.push(node[unflatten_idx(&size, idx)]);
        }
        assert_eq!(count, data.len());

        Self { size, data }
    }
}
impl<C: CellType, D: Dim> From<&NdCachedNode<C, D>> for NdArray<C, D> {
    fn from(node: &NdCachedNode<C, D>) -> Self {
        Self::from(&**node)
    }
}

// Get or set an element in an NdArray.
impl<T, D: Dim> Index<&IVec<D>> for NdArray<T, D> {
    type Output = T;
    fn index(&self, pos: &IVec<D>) -> &T {
        &self.data[self.flatten_idx(pos)]
    }
}
impl<T, D: Dim> IndexMut<&IVec<D>> for NdArray<T, D> {
    fn index_mut(&mut self, pos: &IVec<D>) -> &mut T {
        let idx = self.flatten_idx(pos);
        &mut self.data[idx]
    }
}

impl<T, D: Dim> NdArray<T, D> {
    fn flatten_idx(&self, pos: &IVec<D>) -> usize {
        flatten_idx(&self.size, pos)
    }
    /// Returns an NdArraySlice of this NdArray with no offset.
    pub fn slice(&self) -> NdArraySlice<T, D> {
        self.into()
    }
    /// Returns an NdArraySlice of this NdArray with the given offset.
    pub fn offset_slice(&self, offset: IVec<D>) -> NdArraySlice<T, D> {
        let array = self;
        NdArraySlice { array, offset }
    }
    /// Returns the size vector of this NdArray.
    pub fn size(&self) -> &UVec<D> {
        &self.size
    }
    /// Returns the rectangle of this NdArray.
    pub fn rect(&self) -> URect<D> {
        URect::new(UVec::origin(), self.size.clone())
    }
}

/// An offset immutable slice of a NdArray.
pub struct NdArraySlice<'a, T, D: Dim> {
    array: &'a NdArray<T, D>,
    offset: IVec<D>,
}
impl<'a, T, D: Dim> AsRef<NdArray<T, D>> for NdArraySlice<'a, T, D> {
    fn as_ref(&self) -> &NdArray<T, D> {
        self.array
    }
}
impl<'a, T, D: Dim> From<&'a NdArray<T, D>> for NdArraySlice<'a, T, D> {
    fn from(array: &'a NdArray<T, D>) -> Self {
        let offset = IVec::origin();
        Self { array, offset }
    }
}

impl<'a, T, D: Dim> Index<&IVec<D>> for NdArraySlice<'a, T, D> {
    type Output = T;
    fn index(&self, pos: &IVec<D>) -> &T {
        &self.array[&(pos - self.offset.clone()).convert()]
    }
}

impl<'a, T, D: Dim> NdArraySlice<'a, T, D> {
    /// Returns the rectangle of this NdArraySlice.
    pub fn rect(&self) -> IRect<D> {
        IRect::new(
            -self.offset.clone(),
            IVec::from_fn(|ax| self.array.size[ax] as isize),
        )
    }
}

/// Converts a usize array index into an NdVec position.
fn unflatten_idx<D: Dim>(size: &UVec<D>, mut idx: usize) -> IVec<D> {
    let mut ret = IVec::origin();
    assert!(idx < size.product() as usize);
    for &ax in D::axes() {
        ret[ax] = idx.rem_euclid(size[ax]) as isize;
        idx = idx.div_euclid(size[ax]);
    }
    ret
}

/// Converts an NdVec position into a usize array index.
fn flatten_idx<D: Dim>(size: &UVec<D>, pos: &IVec<D>) -> usize {
    let mut ret = 0;
    let mut stride = 1;
    for &ax in D::axes() {
        assert!(0 <= pos[ax] && pos[ax] < size[ax] as isize);
        ret += pos[ax] as usize * stride;
        stride *= size[ax];
    }
    ret
}

/// A 1D array.
pub type Array1D<T> = NdArray<T, Dim1D>;
/// A 2D array.
pub type Array2D<T> = NdArray<T, Dim2D>;
/// A 3D array.
pub type Array3D<T> = NdArray<T, Dim3D>;
/// A 4D array.
pub type Array4D<T> = NdArray<T, Dim4D>;
/// A 5D array.
pub type Array5D<T> = NdArray<T, Dim5D>;
/// A 6D array.
pub type Array6D<T> = NdArray<T, Dim6D>;

/// An offset slice of a 1D array.
pub type ArraySlice1D<'a, T> = NdArraySlice<'a, T, Dim1D>;
/// An offset slice of a 2D array.
pub type ArraySlice2D<'a, T> = NdArraySlice<'a, T, Dim2D>;
/// An offset slice of a 3D array.
pub type ArraySlice3D<'a, T> = NdArraySlice<'a, T, Dim3D>;
/// An offset slice of a 4D array.
pub type ArraySlice4D<'a, T> = NdArraySlice<'a, T, Dim4D>;
/// An offset slice of a 5D array.
pub type ArraySlice5D<'a, T> = NdArraySlice<'a, T, Dim5D>;
/// An offset slice of a 6D array.
pub type ArraySlice6D<'a, T> = NdArraySlice<'a, T, Dim6D>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that flatten_idx() and unflatten_idx() are inverses and never
    /// return values out of range.
    #[test]
    fn test_flatten_unflatten_ndarray_idx() {
        let size: UVec4D = NdVec([4, 5, 6, 7]);
        let rect: IRect4D = NdRect::new(NdVec::origin(), size.as_ivec());
        let count = rect.count() as usize;
        for pos in rect.iter() {
            let flat_idx: usize = flatten_idx(&size, &pos);
            assert!(flat_idx < count);
            assert_eq!(pos, unflatten_idx(&size, flat_idx));
        }
    }
}

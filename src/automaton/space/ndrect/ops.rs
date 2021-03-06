//! Operations on NdRects that are equivalent to applying the operation to the
//! minimum and maximum corners of the hyperrectangle.

use std::ops::*;

use super::*;

// Implement addition and subtraction on anything that can be added/subtracted
// to/from an NdVec.
impl<D: DimFor<N>, N: NdVecNum, X> Add<X> for NdRect<D, N>
where
    NdVec<D, N>: NdRectVec + Add<X, Output = NdVec<D, N>>,
{
    type Output = Self;
    fn add(self, operand: X) -> Self {
        Self {
            start: self.start + operand,
            size: self.size,
        }
    }
}
impl<D: DimFor<N>, N: NdVecNum, X> AddAssign<X> for NdRect<D, N>
where
    NdVec<D, N>: NdRectVec + AddAssign<X>,
{
    fn add_assign(&mut self, operand: X) {
        self.start += operand
    }
}
impl<D: DimFor<N>, N: NdVecNum, X> Sub<X> for NdRect<D, N>
where
    NdVec<D, N>: NdRectVec + Sub<X, Output = NdVec<D, N>>,
{
    type Output = Self;
    fn sub(self, operand: X) -> Self {
        Self {
            start: self.start - operand,
            size: self.size,
        }
    }
}
impl<D: DimFor<N>, N: NdVecNum, X> SubAssign<X> for NdRect<D, N>
where
    NdVec<D, N>: NdRectVec + SubAssign<X>,
{
    fn sub_assign(&mut self, operand: X) {
        self.start -= operand
    }
}

// Integer multiplication is special, because bounds are inclusive.
// Multiplication by a negative number panics.
impl<D: DimFor<N>, N: NdVecNum, X> Mul<X> for NdRect<D, N>
where
    NdVec<D, N>: NdRectVec,
    Self: MulAssign<X>,
{
    type Output = Self;
    fn mul(self, operand: X) -> Self {
        let mut ret = self;
        ret *= operand;
        ret
    }
}
impl<D: DimFor<N>, N: NdVecNum, X: Copy> MulAssign<X> for NdRect<D, N>
where
    NdVec<D, N>: NdRectVec + MulAssign<X>,
{
    fn mul_assign(&mut self, operand: X) {
        // Call span() rather than constructing directly because if we multiply
        // by a negative number then the min and max might swap, so we need to
        // double-check the bounds.
        self.start *= operand;
        self.size *= operand;
        for &ax in D::Dim::axes() {
            assert!(
                self.size[ax] > N::zero(),
                "Cannot multiply an NdRect by a negative value"
            );
        }
    }
}

// Implement integer division.
impl<D: DimFor<N>, N: NdVecNum + Integer> NdRect<D, N>
where
    NdVec<D, N>: NdRectVec,
{
    /// "Outward-rounded" integer division; returns the largest rectangle that is
    /// the given fraction of the size of the original.
    pub fn div_outward(&self, other: &N) -> Self {
        Self::span(self.min().div_floor(other), self.max().div_ceil(other))
    }
}

// Implement float division.
impl<D: DimFor<N>, N: NdVecNum + Float, X> Div<X> for NdRect<D, N>
where
    NdVec<D, N>: NdRectVec,
    Self: DivAssign<X>,
{
    type Output = Self;
    fn div(self, operand: X) -> Self {
        let mut ret = self;
        ret /= operand;
        ret
    }
}
impl<D: DimFor<N>, N: NdVecNum + Float, X: Copy> DivAssign<X> for NdRect<D, N>
where
    NdVec<D, N>: NdRectVec + DivAssign<X>,
{
    fn div_assign(&mut self, operand: X) {
        self.start /= operand;
        self.size /= operand;
    }
}

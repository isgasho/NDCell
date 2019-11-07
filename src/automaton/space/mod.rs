//! Everything relating to the spatial "universe" of an automaton, including
//! grid topology and vector operations.

use std::cmp::Eq;
use std::default::Default;
use std::fmt::Debug;
use std::hash::Hash;

mod grid;
mod vector;
mod vector_container;

pub use grid::Grid;
pub use vector::Vector;
pub use vector_container::{CellVector, ChunkVector, LocalVector};

/// A "trait alias" for ndarray::Dimension + std::cmp::Eq + std::hash::Hash so
/// that it can be used in HashMaps.
pub trait Dimension: ndarray::Dimension + Eq + Hash {}
impl<T: ndarray::Dimension + Eq + Hash> Dimension for T {}

/// A "trait alias" for a cell type that has a "default" value and can be copied
/// for free or near-free.
pub trait Cell: Debug + Copy + Default + Eq {}
impl<T: Debug + Copy + Default + Eq> Cell for T {}

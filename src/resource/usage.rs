use std::fmt::Debug;
use std::ops::{BitOr, BitOrAssign};

/// Access type combination
pub trait Usage: Debug + Copy + BitOr<Output = Self> + BitOrAssign {
    /// Create empty combinations of usage types.
    fn none() -> Self;

    /// Create usage instance that combines all possible usage types
    fn all() -> Self;
}

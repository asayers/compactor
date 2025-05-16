mod compactor;
mod date;
pub mod policy;
mod resolution;
mod time;
mod types;

pub use crate::compactor::Compactor;
pub use crate::date::Date;
pub use crate::resolution::Resolution;
pub use crate::time::ResTime;
pub use crate::types::{AmPm, TimeOfDay};

/// aka. `Semigroup` in Haskell-speak
pub trait Aggregate: Sized {
    /// Does **not** need to be commutative
    fn merge(&mut self, other: Self);
}

impl<T> Aggregate for Vec<T> {
    fn merge(&mut self, mut other: Self) {
        self.append(&mut other);
    }
}

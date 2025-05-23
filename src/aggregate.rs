/// aka. `Semigroup` in Haskell-speak
pub trait Aggregate: Sized {
    /// Does **not** need to be commutative
    fn merge(&mut self, other: Self);
}

impl<T: Aggregate> Aggregate for Option<T> {
    fn merge(&mut self, other: Self) {
        if let Some(this) = self {
            if let Some(other) = other {
                this.merge(other)
            }
        } else {
            *self = other;
        }
    }
}

impl<T> Aggregate for Vec<T> {
    fn merge(&mut self, mut other: Self) {
        self.append(&mut other);
    }
}

pub struct Min<T>(pub T);
impl<T: PartialOrd> Aggregate for Min<T> {
    fn merge(&mut self, other: Self) {
        if self.0 > other.0 {
            *self = other;
        }
    }
}

pub struct Max<T>(pub T);
impl<T: PartialOrd> Aggregate for Max<T> {
    fn merge(&mut self, other: Self) {
        if self.0 < other.0 {
            *self = other;
        }
    }
}

pub struct First<T>(pub T);
impl<T> Aggregate for First<T> {
    fn merge(&mut self, _: Self) {
        // no-op!
    }
}

pub struct Last<T>(pub T);
impl<T> Aggregate for Last<T> {
    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

pub struct Candlestick<T> {
    pub first: First<T>,
    pub last: Last<T>,
    pub min: Min<T>,
    pub max: Max<T>,
}

impl<T: Clone> From<T> for Candlestick<T> {
    fn from(x: T) -> Self {
        Candlestick {
            first: First(x.clone()),
            last: Last(x.clone()),
            min: Min(x.clone()),
            max: Max(x),
        }
    }
}

impl<T: PartialOrd> Aggregate for Candlestick<T> {
    fn merge(&mut self, other: Self) {
        self.first.merge(other.first);
        self.last.merge(other.last);
        self.min.merge(other.min);
        self.max.merge(other.max);
    }
}

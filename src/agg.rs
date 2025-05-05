use crate::{ResTime, Resolution};
use jiff::civil::Date;

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

// TODO: RLE the dates?
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct Aggs<T>(pub Vec<(Date, ResTime, T)>);

impl<T: Aggregate> Aggs<T> {
    pub fn push(&mut self, date: Date, time: ResTime, x: T) -> Result<(), &'static str> {
        if let Some(last) = self.0.last_mut() {
            match last.0.cmp(&date).then(last.1.coarse_cmp(time)) {
                std::cmp::Ordering::Less => (),
                std::cmp::Ordering::Equal => {
                    last.2.merge(x);
                    return Ok(());
                }
                std::cmp::Ordering::Greater => return Err("Non-monotonic push"),
            }
        }
        self.0.push((date, time, x));
        Ok(())
    }

    pub fn compact(&mut self, res: Resolution, up_to: Date) {
        let mut start = None;
        let mut end = None;
        for (i, x) in self.0.iter().enumerate() {
            if x.1.resolution() <= res {
                // Already compacted - skip
                continue;
            }
            if x.0 > up_to {
                // Out of range
                break;
            }
            start = start.or(Some(i));
            end = Some(i);
        }
        let Some((start, end)) = start.zip(end) else {
            return;
        };
        let mut merged: Vec<(Date, ResTime, T)> = vec![];
        for (date, mut time, agg) in self.0.splice(start..end, []) {
            time.reduce_to(res);
            if let Some(head) = merged.last_mut() {
                if head.0 == date && head.1 == time {
                    head.2.merge(agg);
                    continue;
                }
            }
            merged.push((date, time, agg));
        }
        self.0.splice(start..start, merged);
    }
}

/*
struct Replacement<T>(std::rc::Rc<std::cell::Cell<Option<I>>>);
impl<T> IntoIterator for Replacement<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.take().unwrap().into_iter()
    }
}
struct BetterSplice<I>(std::vec::Splice<Replacement<I>>);

impl<T> BetterSplice<T> {
    fn finish(self, xs: impl IntoIterator<Item = T>) {}
}

fn vec_splice() {
        let replacement = Replacement(std::rc::Rc::new(std::cell::Cell::new(None)));
        let mut iter = self
            .0
            .splice(start..=end, Replacement(replacement.0.clone()));
        let xs = vec![];
        while let Some(x) = iter.next() {}
        replacement.0.set(Some(xs));
        std::mem::drop(iter);
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agg() {
        let mut agg = Aggs::default();
        for d in 15..20 {
            let date = Date::new(2023, 1, d).unwrap();
            let t = ResTime::default();
            for h in 8..15 {
                agg.push(date, t.with_hour(h), vec![d as u32 * 100 + h as u32])
                    .unwrap();
            }
        }
        eprintln!("{agg:#?}");
        agg.compact(Resolution::TimeOfDay, Date::new(2023, 1, 17).unwrap());
        eprintln!("{agg:#?}");
        // agg.compact(Resolution::TimeOfDay, Date::new(2023, 1, 19).unwrap());
        // eprintln!("{agg:#?}");
        // agg.compact(Resolution::AmPm, Date::new(2023, 1, 16).unwrap());
        // eprintln!("{agg:#?}");
        // agg.compact(Resolution::Day, Date::new(2023, 2, 16).unwrap());
        // eprintln!("{agg:#?}");
        panic!();
    }
}

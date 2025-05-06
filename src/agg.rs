use crate::{ResTime, Resolution};
use jiff::civil::Date;
use std::{cmp::Ordering, marker::PhantomData};

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

pub struct CompactorBuilder<T>(Vec<(u16 /* days */, Resolution)>, PhantomData<T>);

impl<T> CompactorBuilder<T> {
    /// Allow this compactor to keep data at resolution `res` for up to
    /// `num_days` days
    pub fn add_policy(mut self, num_days: u16, res: Resolution) -> Self {
        self.0.push((num_days, res));
        self
    }

    pub fn build(self) -> Compactor<T> {
        let mut policy = self.0;
        policy.sort();
        assert!(policy.iter().map(|x| x.0).is_sorted());
        assert!(policy.iter().map(|x| std::cmp::Reverse(x.1)).is_sorted());
        // TODO: Remove dominated policies instead of panicking
        Compactor {
            data: vec![],
            policy: policy.into(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Compactor<T> {
    // TODO: RLE the dates?
    pub data: Vec<(Date, ResTime, T)>,
    pub policy: Box<[(u16 /* days */, Resolution)]>,
}

impl<T: Aggregate> Compactor<T> {
    pub fn new() -> CompactorBuilder<T> {
        CompactorBuilder(vec![], PhantomData)
    }

    pub fn push(&mut self, date: Date, mut time: ResTime, x: T) -> Result<(), &'static str> {
        let Some(max_res) = self.policy.first().map(|x| x.1) else {
            // If there are no policies, it means we're not allowed to store
            // any data
            return Ok(());
        };
        time.reduce_to(max_res);

        let Some(last) = self.data.last_mut() else {
            // It's the first item
            self.data.push((date, time, x));
            return Ok(());
        };

        // Check the date
        match last.0.cmp(&date) {
            Ordering::Equal => (), // The common case
            Ordering::Greater => return Err("Non-monotonic push"),
            Ordering::Less => {
                // It's a new day.  We need to evaluate the policies
                self.data.push((date, time, x));
                // TODO: The compactions could be combined... but it doesn't
                // matter: this isn't the fast path
                for (days, res) in &self.policy {
                    // FIXME
                    let up_to = date - jiff::Span::new().days(*days);
                    compact(&mut self.data, *res, up_to);
                }
                // Remove data no longer covered by any policy
                let max_retention = self.policy.last().unwrap().0;
                let up_to = date - jiff::Span::new().days(max_retention);
                let remove = self.data.iter().position(|x| x.0 >= up_to).unwrap_or(0);
                self.data.splice(0..remove, []);
                return Ok(());
            }
        }

        // Check the time
        match last.1.partial_cmp(&time) {
            // `None` means that `time` is at a different resolution level to
            // `last`.  In other words, there has just been a compaction, with
            // no new data pushed since.  I don't think this is possible.
            None => panic!("Compacted head"),
            Some(Ordering::Less) => self.data.push((date, time, x)), // no compaction
            Some(Ordering::Equal) => last.2.merge(x),
            Some(Ordering::Greater) => return Err("Non-monotonic push"),
        }
        Ok(())
    }
}

/// `up_to` is inclusive
fn compact<T: Aggregate>(data: &mut Vec<(Date, ResTime, T)>, res: Resolution, up_to: Date) {
    let mut start = None;
    let mut end = None;
    for (i, x) in data.iter().enumerate() {
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
    for (date, mut time, agg) in data.splice(start..=end, []) {
        time.reduce_to(res);
        if let Some(head) = merged.last_mut() {
            if head.0 == date && head.1 == time {
                head.2.merge(agg);
                continue;
            }
        }
        merged.push((date, time, agg));
    }
    data.splice(start..start, merged);

    // Sanity check:
    for (date, time, _) in data {
        if *date <= up_to {
            assert!(time.resolution() <= res);
        }
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
    use jiff::civil::date;

    #[test]
    fn test_agg() {
        let mut agg = Compactor::new()
            .add_policy(2, Resolution::Hour)
            .add_policy(5, Resolution::AmPm)
            .build();
        let mut simple = vec![];
        for d in 15..20 {
            let date = date(2023, 1, d);
            let t = ResTime::default();
            for h in 8..15 {
                let x = d as u32 * 100 + h as u32;
                agg.push(date, t.with_hour(h), vec![x]).unwrap();
                simple.push(x);
            }
        }
        eprintln!("{agg:#?}");
        assert!(agg.data.iter().flat_map(|x| &x.2).eq(&simple));
        for (d, time, _) in &agg.data {
            if *d <= date(2023, 1, 17) {
                assert_eq!(time.resolution(), Resolution::TimeOfDay);
            }
        }
        eprintln!("{agg:#?}");
        assert!(agg.data.iter().flat_map(|x| &x.2).eq(&simple));
        eprintln!("{agg:#?}");
        {
            let date = date(2023, 1, 21);
            let t = ResTime::default();
            for h in 8..15 {
                let x = 21 as u32 * 100 + h as u32;
                agg.push(date, t.with_hour(h), vec![x]).unwrap();
                simple.push(x);
            }
        }
        eprintln!("{agg:#?}");
        panic!();
    }
}

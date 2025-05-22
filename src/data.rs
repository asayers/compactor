use crate::{Aggregate, Date, Resolution, Time, policy::Policy};
use core::fmt;

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// TODO: RLE the dates?
pub(crate) struct CompactedData<T>(pub(crate) Vec<(Date, Time, T)>);

impl<T> Default for CompactedData<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: fmt::Debug> fmt::Debug for CompactedData<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        for (date, time, x) in &self.0 {
            map.entry(&format_args!("{date} {time}"), x);
        }
        map.finish()
    }
}

impl<T: Aggregate> CompactedData<T> {
    /// Remove data on days up to and including `up_to`
    fn discard(&mut self, up_to: Date) {
        let remove = self
            .0
            .iter()
            .position(|x| x.0 > up_to)
            .unwrap_or(self.0.len());
        self.0.drain(..remove);
    }

    /// Compact data on days up to and including `up_to`, reducing the
    /// resolution to (at most) `res`
    fn compact(&mut self, up_to: Date, res: Resolution) {
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
        let merged = with_max_res(res, self.0.splice(start..=end, [])).collect::<Vec<_>>();
        self.0.splice(start..start, merged);

        // Sanity check:
        for (date, time, _) in &self.0 {
            if *date <= up_to {
                assert!(time.resolution() <= res);
            }
        }
    }

    // TODO: The compactions could be combined... but it doesn't matter: this
    // isn't the fast path
    pub(crate) fn apply_policy(&mut self, policy: &Policy, date: Date) {
        let date = jiff::civil::date(date.year, date.month, date.day);

        // Remove data no longer covered by any policy
        let up_to = date - jiff::Span::new().days(policy.max_retention);
        let up_to = Date {
            year: up_to.year(),
            month: up_to.month(),
            day: up_to.day(),
        };
        self.discard(up_to);

        for (days, res) in &policy.compaction_rules {
            let up_to = date - jiff::Span::new().days(*days);
            let up_to = Date {
                year: up_to.year(),
                month: up_to.month(),
                day: up_to.day(),
            };
            self.compact(up_to, *res);
        }
    }
}

pub(crate) fn with_max_res<T: Aggregate>(
    res: Resolution,
    xs: impl Iterator<Item = (Date, Time, T)>,
) -> impl Iterator<Item = (Date, Time, T)> {
    let mut cur: Option<(Date, Time, T)> = None;
    xs.map(Some).chain([None]).filter_map(move |x| match x {
        Some((date, mut time, x)) => {
            time.reduce_to(res);
            if let Some(cur) = &mut cur {
                if cur.0 == date && cur.1 == time {
                    cur.2.merge(x);
                    return None;
                }
            }
            cur.replace((date, time, x))
        }
        None => cur.take(),
    })
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

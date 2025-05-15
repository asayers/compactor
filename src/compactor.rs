use crate::{Aggregate, Policy, ResTime, Resolution};
use core::fmt;
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Date {
    pub year: i16,
    pub month: i8,
    pub day: i8,
}
impl fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
impl From<jiff::civil::Date> for Date {
    fn from(date: jiff::civil::Date) -> Self {
        Date {
            year: date.year(),
            month: date.month(),
            day: date.day(),
        }
    }
}
#[cfg(feature = "chrono")]
impl From<chrono::NaiveDate> for Date {
    fn from(date: chrono::NaiveDate) -> Self {
        use chrono::Datelike;
        Date {
            year: date.year() as i16,
            month: date.month() as i8,
            day: date.day() as i8,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// TODO: RLE the dates?
pub struct CompactedData<T>(Vec<(Date, ResTime, T)>);

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
        self.0.splice(0..remove, []);
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
        let mut merged: Vec<(Date, ResTime, T)> = vec![];
        for (date, mut time, agg) in self.0.splice(start..=end, []) {
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

        // Sanity check:
        for (date, time, _) in &self.0 {
            if *date <= up_to {
                assert!(time.resolution() <= res);
            }
        }
    }

    // TODO: The compactions could be combined... but it doesn't matter: this
    // isn't the fast path
    fn apply_policy(&mut self, policy: &Policy, date: Date) {
        let date = jiff::civil::date(date.year, date.month, date.day);

        // Remove data no longer covered by any policy
        let up_to = date - jiff::Span::new().days(policy.max_retention);
        self.discard(up_to.into());

        for (days, res) in &policy.compaction_rules {
            let up_to = date - jiff::Span::new().days(*days);
            self.compact(up_to.into(), *res);
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Compactor<T> {
    policy: Policy,
    data: CompactedData<T>,
}

impl<T> From<Policy> for Compactor<T> {
    fn from(policy: Policy) -> Self {
        Self {
            policy,
            data: CompactedData::default(),
        }
    }
}

impl<T> Compactor<T> {
    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    pub fn is_empty(&self) -> bool {
        self.data.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.0.len()
    }

    /// Goes from old -> new
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Date, ResTime, &T)> {
        self.data.0.iter().map(|(d, t, x)| (*d, *t, x))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PushError {
    NonMonotonic,
}

impl<T: Aggregate> Compactor<T> {
    pub fn push(
        &mut self,
        date: impl Into<Date>,
        time: impl Into<ResTime>,
        x: T,
    ) -> Result<(), PushError> {
        let date = date.into();
        let mut time = time.into();
        time.reduce_to(self.policy.max_res);

        let Some(last) = self.data.0.last_mut() else {
            // It's the first item
            self.data.0.push((date, time, x));
            return Ok(());
        };

        // Check the date
        match last.0.cmp(&date) {
            Ordering::Equal => (), // The common case
            Ordering::Greater => return Err(PushError::NonMonotonic),
            Ordering::Less => {
                // It's a new day.  We need to evaluate the policies
                self.data.0.push((date, time, x));
                self.data.apply_policy(&self.policy, date);
                return Ok(());
            }
        }

        // Check the time
        // `partial_cmp() == None` means that `time` is at a different
        // resolution level to `last`.  In other words, there has just been
        // a compaction, with no new data pushed since.  I don't think this
        // is possible.
        let ord = last.1.partial_cmp(&time).expect("Compacted head");
        match ord {
            Ordering::Less => self.data.0.push((date, time, x)), // no compaction
            Ordering::Equal => last.2.merge(x),
            Ordering::Greater => return Err(PushError::NonMonotonic),
        }
        Ok(())
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
    fn test_dominated_policies() {
        assert!(
            Compactor::<()>::new()
                .keep_for_days(5, Resolution::Hour)
                .keep_for_days(2, Resolution::AmPm)
                .build()
                .is_err()
        );
        assert!(
            Compactor::<()>::new()
                .keep_for_days(2, Resolution::AmPm)
                .keep_for_days(5, Resolution::Hour)
                .build()
                .is_err()
        );
        assert!(
            Compactor::<()>::new()
                .keep_for_days(2, Resolution::Hour)
                .keep_for_days(2, Resolution::AmPm)
                .build()
                .is_err()
        );
    }

    #[test]
    fn test_duplicate_policies() {
        let x = Compactor::<()>::new()
            .keep_for_days(2, Resolution::Hour)
            .keep_for_days(2, Resolution::Hour)
            .build();
        let y = Compactor::<()>::new()
            .keep_for_days(2, Resolution::Hour)
            .build();
        assert_eq!(x, y);
    }

    fn time(h: u8, m: u8, s: u8) -> ResTime {
        ResTime::default()
            .with_hour(h)
            .with_minute(m)
            .with_second(s)
    }

    #[test]
    fn test_one_day() -> Result<(), PushError> {
        let mut agg = Compactor::new()
            .keep_for_days(1, Resolution::Day)
            .build()
            .unwrap();
        agg.push(date(2023, 1, 1), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 1), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 1), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![(date(2023, 1, 1).into(), ResTime::WHOLE_DAY, vec![1, 2, 3])]
        );
        agg.push(date(2023, 1, 2), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 2), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 2), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![(date(2023, 1, 2).into(), ResTime::WHOLE_DAY, vec![1, 2, 3])]
        );
        agg.push(date(2023, 1, 3), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 3), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 3), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![(date(2023, 1, 3).into(), ResTime::WHOLE_DAY, vec![1, 2, 3])]
        );
        Ok(())
    }

    #[test]
    fn test_two_days() -> Result<(), PushError> {
        let mut agg = Compactor::new()
            .keep_for_days(2, Resolution::Day)
            .build()
            .unwrap();
        agg.push(date(2023, 1, 1), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 1), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 1), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![(date(2023, 1, 1).into(), ResTime::WHOLE_DAY, vec![1, 2, 3])]
        );
        agg.push(date(2023, 1, 2), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 2), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 2), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), ResTime::WHOLE_DAY, vec![1, 2, 3]),
                (date(2023, 1, 2), ResTime::WHOLE_DAY, vec![1, 2, 3])
            ]
        );
        agg.push(date(2023, 1, 3), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 3), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 3), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 2), ResTime::WHOLE_DAY, vec![1, 2, 3]),
                (date(2023, 1, 3), ResTime::WHOLE_DAY, vec![1, 2, 3])
            ]
        );
        Ok(())
    }

    #[test]
    fn test_ampm() -> Result<(), PushError> {
        let mut agg = Compactor::new()
            .keep_for_days(1, Resolution::AmPm)
            .keep_for_days(2, Resolution::Day)
            .build()
            .unwrap();
        agg.push(date(2023, 1, 1), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 1), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), ResTime::AM, vec![1]),
                (date(2023, 1, 1), ResTime::PM, vec![2]),
            ]
        );
        agg.push(date(2023, 1, 2), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 2), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), ResTime::WHOLE_DAY, vec![1, 2]),
                (date(2023, 1, 2), ResTime::AM, vec![1]),
                (date(2023, 1, 2), ResTime::PM, vec![2]),
            ]
        );
        agg.push(date(2023, 1, 3), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 3), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 2), ResTime::WHOLE_DAY, vec![1, 2]),
                (date(2023, 1, 3), ResTime::AM, vec![1]),
                (date(2023, 1, 3), ResTime::PM, vec![2]),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_3_level() -> Result<(), PushError> {
        let mut agg = Compactor::new()
            .keep_for_days(2, Resolution::AmPm)
            .keep_for_days(3, Resolution::Day)
            .keep_for_days(1, Resolution::Hour)
            .build()
            .unwrap();
        agg.push(date(2023, 1, 1), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 1), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), ResTime::from_hour(11), vec![1]),
                (date(2023, 1, 1), ResTime::from_hour(13), vec![2]),
            ]
        );
        agg.push(date(2023, 1, 2), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 2), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), ResTime::AM, vec![1]),
                (date(2023, 1, 1), ResTime::PM, vec![2]),
                (date(2023, 1, 2), ResTime::from_hour(11), vec![1]),
                (date(2023, 1, 2), ResTime::from_hour(13), vec![2]),
            ]
        );
        agg.push(date(2023, 1, 3), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 3), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), ResTime::WHOLE_DAY, vec![1, 2]),
                (date(2023, 1, 2), ResTime::AM, vec![1]),
                (date(2023, 1, 2), ResTime::PM, vec![2]),
                (date(2023, 1, 3), ResTime::from_hour(11), vec![1]),
                (date(2023, 1, 3), ResTime::from_hour(13), vec![2]),
            ]
        );
        agg.push(date(2023, 1, 4), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 4), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 2), ResTime::WHOLE_DAY, vec![1, 2]),
                (date(2023, 1, 3), ResTime::AM, vec![1]),
                (date(2023, 1, 3), ResTime::PM, vec![2]),
                (date(2023, 1, 4), ResTime::from_hour(11), vec![1]),
                (date(2023, 1, 4), ResTime::from_hour(13), vec![2]),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_agg() {
        let mut agg = Compactor::new()
            .keep_for_days(2, Resolution::Hour)
            .keep_for_days(4, Resolution::AmPm)
            .keep_for_days(6, Resolution::Day)
            .build()
            .unwrap();
        let mut simple = vec![];
        for d in 10..20 {
            let date = date(2023, 1, d);
            let t = ResTime::default();
            for h in 8..15 {
                let x = d as u32 * 100 + h as u32;
                agg.push(date, t.with_hour(h), vec![x]).unwrap();
                simple.push(x);
            }
        }
        eprintln!("{agg:#?}");
        assert_eq!(agg.iter().flat_map(|x| x.2).count(), 7 * 6);
        for (x, y) in agg.iter().flat_map(|x| x.2).rev().zip(simple.iter().rev()) {
            assert_eq!(x, y);
        }
        for (d, time, _) in agg.iter() {
            if d >= date(2023, 1, 18) {
                assert_eq!(time.resolution(), Resolution::Hour, "{d}");
            } else if d >= date(2023, 1, 16) {
                assert_eq!(time.resolution(), Resolution::AmPm, "{d}");
            } else {
                assert_eq!(time.resolution(), Resolution::Day, "{d}");
            }
        }
        eprintln!("{agg:#?}");
        assert!(
            agg.iter()
                .flat_map(|x| x.2)
                .rev()
                .zip(simple.iter().rev())
                .all(|(x, y)| x == y)
        );
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
    }
}

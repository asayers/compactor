use crate::{
    Aggregate, Date, Resolution, Time,
    data::*,
    policy::{Policy, PolicyBuilder, PolicyError},
};
use std::{cmp::Ordering, marker::PhantomData};

/// Stores data at gradually diminishing resolution
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

pub struct CompactorBuilder<T>(PolicyBuilder, PhantomData<T>);

impl<T> Default for CompactorBuilder<T> {
    fn default() -> Self {
        CompactorBuilder(PolicyBuilder::default(), PhantomData)
    }
}

impl<T> CompactorBuilder<T> {
    pub fn keep_for_days(mut self, num_days: u16, res: Resolution) -> Self {
        self.0 = self.0.keep_for_days(num_days, res);
        self
    }

    pub fn build(self) -> Result<Compactor<T>, PolicyError> {
        self.0.build().map(Compactor::from)
    }
}

impl<T> Compactor<T> {
    pub fn new() -> CompactorBuilder<T> {
        CompactorBuilder::default()
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
        time: impl Into<Time>,
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

    /// Update the current date without pushing any new data.  This can be used
    /// to force compaction.
    pub fn update_date(&mut self, date: impl Into<Date>) {
        let date = date.into();
        if self.data.0.last_mut().is_some_and(|last| date > last.0) {
            self.data.apply_policy(&self.policy, date);
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
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Date, Time, &T)> {
        self.data.0.iter().map(|(d, t, x)| (*d, *t, x))
    }
}

// Should this be `where &T: Aggregate` instead?
impl<T: Aggregate + Clone> Compactor<T> {
    /// Goes from old -> new
    pub fn iter_with_max_resolution(
        &self,
        res: Resolution,
    ) -> impl Iterator<Item = (Date, Time, T)> {
        with_max_res(res, self.data.0.iter().map(|(d, t, x)| (*d, *t, x.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(h: u8, m: u8, s: u8) -> Time {
        Time::default().with_hour(h).with_minute(m).with_second(s)
    }
    fn date(year: i16, month: i8, day: i8) -> Date {
        Date { year, month, day }
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
            vec![(date(2023, 1, 1), Time::WHOLE_DAY, vec![1, 2, 3])]
        );
        agg.push(date(2023, 1, 2), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 2), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 2), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![(date(2023, 1, 2), Time::WHOLE_DAY, vec![1, 2, 3])]
        );
        agg.push(date(2023, 1, 3), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 3), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 3), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![(date(2023, 1, 3), Time::WHOLE_DAY, vec![1, 2, 3])]
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
            vec![(date(2023, 1, 1), Time::WHOLE_DAY, vec![1, 2, 3])]
        );
        agg.push(date(2023, 1, 2), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 2), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 2), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), Time::WHOLE_DAY, vec![1, 2, 3]),
                (date(2023, 1, 2), Time::WHOLE_DAY, vec![1, 2, 3])
            ]
        );
        agg.push(date(2023, 1, 3), time(13, 1, 0), vec![1])?;
        agg.push(date(2023, 1, 3), time(13, 2, 0), vec![2])?;
        agg.push(date(2023, 1, 3), time(13, 3, 0), vec![3])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 2), Time::WHOLE_DAY, vec![1, 2, 3]),
                (date(2023, 1, 3), Time::WHOLE_DAY, vec![1, 2, 3])
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
                (date(2023, 1, 1), Time::AM, vec![1]),
                (date(2023, 1, 1), Time::PM, vec![2]),
            ]
        );
        agg.push(date(2023, 1, 2), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 2), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), Time::WHOLE_DAY, vec![1, 2]),
                (date(2023, 1, 2), Time::AM, vec![1]),
                (date(2023, 1, 2), Time::PM, vec![2]),
            ]
        );
        agg.push(date(2023, 1, 3), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 3), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 2), Time::WHOLE_DAY, vec![1, 2]),
                (date(2023, 1, 3), Time::AM, vec![1]),
                (date(2023, 1, 3), Time::PM, vec![2]),
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
                (date(2023, 1, 1), Time::from_hour(11), vec![1]),
                (date(2023, 1, 1), Time::from_hour(13), vec![2]),
            ]
        );
        agg.push(date(2023, 1, 2), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 2), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), Time::AM, vec![1]),
                (date(2023, 1, 1), Time::PM, vec![2]),
                (date(2023, 1, 2), Time::from_hour(11), vec![1]),
                (date(2023, 1, 2), Time::from_hour(13), vec![2]),
            ]
        );
        agg.push(date(2023, 1, 3), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 3), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 1), Time::WHOLE_DAY, vec![1, 2]),
                (date(2023, 1, 2), Time::AM, vec![1]),
                (date(2023, 1, 2), Time::PM, vec![2]),
                (date(2023, 1, 3), Time::from_hour(11), vec![1]),
                (date(2023, 1, 3), Time::from_hour(13), vec![2]),
            ]
        );
        agg.push(date(2023, 1, 4), time(11, 0, 0), vec![1])?;
        agg.push(date(2023, 1, 4), time(13, 0, 0), vec![2])?;
        assert_eq!(
            agg.data.0,
            vec![
                (date(2023, 1, 2), Time::WHOLE_DAY, vec![1, 2]),
                (date(2023, 1, 3), Time::AM, vec![1]),
                (date(2023, 1, 3), Time::PM, vec![2]),
                (date(2023, 1, 4), Time::from_hour(11), vec![1]),
                (date(2023, 1, 4), Time::from_hour(13), vec![2]),
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
            let t = Time::default();
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
            let t = Time::default();
            for h in 8..15 {
                let x = 21 as u32 * 100 + h as u32;
                agg.push(date, t.with_hour(h), vec![x]).unwrap();
                simple.push(x);
            }
        }
        eprintln!("{agg:#?}");
    }
}

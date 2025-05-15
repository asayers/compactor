use crate::{Compactor, Resolution};
use std::marker::PhantomData;

/// Describes how data should be compacted
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    // Goes from (distant, low-res) to (recent, high-res)
    pub(crate) compaction_rules: Box<[(Days, Resolution)]>,
    pub(crate) max_res: Resolution,
    pub(crate) max_retention: Days,
}

type Days = u16;

pub struct CompactorBuilder<T>(Vec<(u16 /* days */, Resolution)>, PhantomData<T>);

impl<T> Compactor<T> {
    pub fn new() -> CompactorBuilder<T> {
        CompactorBuilder(vec![], PhantomData)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PolicyError {
    ZeroRetention,
    PolicyAppliesForZeroDays,
    SomePoliciesDominateOthers,
}

impl<T> CompactorBuilder<T> {
    /// Allow this compactor to keep data at resolution `res` for up to
    /// `num_days` days
    pub fn keep_for_days(mut self, num_days: u16, res: Resolution) -> Self {
        self.0.push((num_days, res));
        self
    }

    pub fn into_policy(self) -> Result<Policy, PolicyError> {
        let mut raw_policy = self.0;
        if raw_policy.is_empty() {
            return Err(PolicyError::ZeroRetention);
        }
        for (x, _) in &raw_policy {
            if *x == 0 {
                return Err(PolicyError::PolicyAppliesForZeroDays);
            }
        }
        raw_policy.sort_by(|x, y| x.cmp(y).reverse());
        raw_policy.dedup();
        if !raw_policy.iter().is_sorted_by_key(|x| x.1) {
            return Err(PolicyError::SomePoliciesDominateOthers);
        }
        let max_res = raw_policy.last().unwrap().1;
        let max_retention = raw_policy.first().unwrap().0;
        let days = raw_policy.iter().map(|x| x.0).skip(1);
        let ress = raw_policy.iter().map(|x| x.1);
        let policy = days.zip(ress).collect();
        Ok(Policy {
            compaction_rules: policy,
            max_res,
            max_retention,
        })
    }

    pub fn build(self) -> Result<Compactor<T>, PolicyError> {
        self.into_policy().map(Compactor::from)
    }
}

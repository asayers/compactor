use crate::Resolution;
use core::fmt;

type Days = u16;

/// Describes how data should be compacted
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Policy {
    // Goes from (distant, low-res) to (recent, high-res)
    pub(crate) compaction_rules: Box<[(Days, Resolution)]>,
    pub(crate) max_res: Resolution,
    pub(crate) max_retention: Days,
}

impl fmt::Display for Policy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            writeln!(f, "Initial: {}-resolution", self.max_res)?;
            for (d, res) in self.compaction_rules.iter().rev() {
                writeln!(f, "After {d} days: reduce to {res}-resolution")?;
            }
            write!(f, "After {} days: delete", self.max_retention)?;
        } else {
            write!(f, "{}", self.max_res)?;
            for (d, res) in self.compaction_rules.iter().rev() {
                write!(f, " →  ({d}d) {res}")?;
            }
            write!(f, " →  ({}d) delete", self.max_retention)?;
        }
        Ok(())
    }
}

impl Policy {
    pub fn new() -> PolicyBuilder {
        PolicyBuilder::default()
    }
}

#[derive(Default)]
pub struct PolicyBuilder(Vec<(Days, Resolution)>);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PolicyError {
    ZeroRetention,
    PolicyAppliesForZeroDays,
    SomePoliciesDominateOthers,
}

impl PolicyBuilder {
    /// Allow this compactor to keep data at resolution `res` for up to
    /// `num_days` days
    pub fn keep_for_days(mut self, num_days: u16, res: Resolution) -> Self {
        self.0.push((num_days, res));
        self
    }

    pub fn build(self) -> Result<Policy, PolicyError> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt() {
        let policy = Policy::new()
            .keep_for_days(1, Resolution::FiveSecond)
            .keep_for_days(2, Resolution::FifteenSecond)
            .keep_for_days(5, Resolution::Minute)
            .keep_for_days(10, Resolution::FiveMinute)
            .keep_for_days(30, Resolution::FifteenMinute)
            .keep_for_days(90, Resolution::Hour)
            .keep_for_days(180, Resolution::AmPm)
            .keep_for_days(365, Resolution::Day)
            .build()
            .unwrap();
        assert_eq!(
            format!("{:#}", policy),
            "Initial: 5s-resolution\n\
            After 1 days: reduce to 15s-resolution\n\
            After 2 days: reduce to minute-resolution\n\
            After 5 days: reduce to 5m-resolution\n\
            After 10 days: reduce to 15m-resolution\n\
            After 30 days: reduce to hour-resolution\n\
            After 90 days: reduce to AM/PM-resolution\n\
            After 180 days: reduce to day-resolution\n\
            After 365 days: delete"
        );
        assert_eq!(
            policy.to_string(),
            "5s →  (1d) 15s →  (2d) minute →  (5d) 5m →  (10d) 15m \
            →  (30d) hour →  (90d) AM/PM →  (180d) day →  (365d) delete"
        );
    }

    #[test]
    fn test_dominated_policies() {
        assert!(
            PolicyBuilder::default()
                .keep_for_days(5, Resolution::Hour)
                .keep_for_days(2, Resolution::AmPm)
                .build()
                .is_err()
        );
        assert!(
            PolicyBuilder::default()
                .keep_for_days(2, Resolution::AmPm)
                .keep_for_days(5, Resolution::Hour)
                .build()
                .is_err()
        );
        assert!(
            PolicyBuilder::default()
                .keep_for_days(2, Resolution::Hour)
                .keep_for_days(2, Resolution::AmPm)
                .build()
                .is_err()
        );
    }

    #[test]
    fn test_duplicate_policies() {
        let x = PolicyBuilder::default()
            .keep_for_days(2, Resolution::Hour)
            .keep_for_days(2, Resolution::Hour)
            .build();
        let y = PolicyBuilder::default()
            .keep_for_days(2, Resolution::Hour)
            .build();
        assert_eq!(x, y);
    }
}

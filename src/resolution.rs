use linearize::{Linearize, LinearizeExt};
use std::{ops::Div, time::Duration};

/// There are 19 resolutions available:
///
/// * milli, 5ms, 10ms, 50ms, 100ms, 500ms
/// * second, 5s, 15s, 30s
/// * minute, 5m, 15m, 30m
/// * hour, 3h, 6h, 12h (am/pm)
/// * whole day
///
/// The `Ord` impl follows natural-language: `x < y` means that x is
/// lower-resolution than y.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Resolution {
    Day,
    AmPm,
    TimeOfDay,
    ThreeHour,
    Hour,
    ThirtyMinute,
    FifteenMinute,
    FiveMinute,
    Minute,
    ThirtySecond,
    FifteenSecond,
    FiveSecond,
    Second,
    FiveHundredMilli,
    HundredMilli,
    FiftyMilli,
    TenMilli,
    FiveMilli,
    Millisecond,
}

impl Resolution {
    pub const fn width(self) -> std::time::Duration {
        match self {
            Resolution::Day => Duration::from_secs(24 * 60 * 60),
            Resolution::AmPm => Duration::from_secs(12 * 60 * 60),
            Resolution::TimeOfDay => Duration::from_secs(6 * 60 * 60),
            Resolution::ThreeHour => Duration::from_secs(3 * 60 * 60),
            Resolution::Hour => Duration::from_secs(60 * 60),
            Resolution::ThirtyMinute => Duration::from_secs(30 * 60),
            Resolution::FifteenMinute => Duration::from_secs(15 * 60),
            Resolution::FiveMinute => Duration::from_secs(5 * 60),
            Resolution::Minute => Duration::from_secs(60),
            Resolution::ThirtySecond => Duration::from_secs(30),
            Resolution::FifteenSecond => Duration::from_secs(15),
            Resolution::FiveSecond => Duration::from_secs(5),
            Resolution::Second => Duration::from_secs(1),
            Resolution::FiveHundredMilli => Duration::from_millis(500),
            Resolution::HundredMilli => Duration::from_millis(100),
            Resolution::FiftyMilli => Duration::from_millis(50),
            Resolution::TenMilli => Duration::from_millis(10),
            Resolution::FiveMilli => Duration::from_millis(5),
            Resolution::Millisecond => Duration::from_millis(1),
        }
    }
}

impl From<Resolution> for std::time::Duration {
    fn from(value: Resolution) -> Self {
        value.width()
    }
}

impl Resolution {
    pub fn coarser(self) -> Option<Self> {
        Resolution::from_linear(self.linearize().checked_sub(1)?)
    }

    pub fn finer(self) -> Option<Self> {
        Resolution::from_linear(self.linearize().checked_add(1)?)
    }

    /// `from` is inclusive, `to` is exclusive.  `from` should be finer than
    /// `to`.
    pub(crate) fn range(
        from: Resolution,
        to: Resolution,
    ) -> impl DoubleEndedIterator<Item = Resolution> {
        let from = from.linearize();
        let to = to.linearize();
        Resolution::variants()
            .skip(to + 1)
            .take(from.saturating_sub(to))
            .rev()
    }
}

impl Div for Resolution {
    type Output = u32;

    fn div(self, rhs: Self) -> Self::Output {
        let mut ret = 1;
        for res in Resolution::range(rhs, self) {
            ret *= res.subdivision() as u32;
        }
        ret
    }
}

impl Resolution {
    pub(crate) fn subdivision(self) -> u8 {
        match self {
            Resolution::Day => 0,
            Resolution::AmPm => 2,
            Resolution::TimeOfDay => 2,
            Resolution::ThreeHour => 2,
            Resolution::Hour => 3,
            Resolution::ThirtyMinute => 2,
            Resolution::FifteenMinute => 2,
            Resolution::FiveMinute => 3,
            Resolution::Minute => 5,
            Resolution::ThirtySecond => 2,
            Resolution::FifteenSecond => 2,
            Resolution::FiveSecond => 3,
            Resolution::Second => 5,
            Resolution::FiveHundredMilli => 2,
            Resolution::HundredMilli => 5,
            Resolution::FiftyMilli => 2,
            Resolution::TenMilli => 5,
            Resolution::FiveMilli => 2,
            Resolution::Millisecond => 5,
        }
    }

    pub(crate) fn n_bits(self) -> u8 {
        match self {
            Resolution::Day => 0,
            Resolution::AmPm => 1,
            Resolution::TimeOfDay => 1,
            Resolution::ThreeHour => 1,
            Resolution::Hour => 2,
            Resolution::ThirtyMinute => 1,
            Resolution::FifteenMinute => 1,
            Resolution::FiveMinute => 2,
            Resolution::Minute => 3,
            Resolution::ThirtySecond => 1,
            Resolution::FifteenSecond => 1,
            Resolution::FiveSecond => 2,
            Resolution::Second => 3,
            Resolution::FiveHundredMilli => 1,
            Resolution::HundredMilli => 3,
            Resolution::FiftyMilli => 1,
            Resolution::TenMilli => 3,
            Resolution::FiveMilli => 1,
            Resolution::Millisecond => 3,
        }
    }

    pub(crate) fn trailing_zeros(self) -> u8 {
        match self {
            Resolution::Day => 31,
            Resolution::AmPm => 30,
            Resolution::TimeOfDay => 29,
            Resolution::ThreeHour => 28,
            Resolution::Hour => 26,
            Resolution::ThirtyMinute => 25,
            Resolution::FifteenMinute => 24,
            Resolution::FiveMinute => 22,
            Resolution::Minute => 19,
            Resolution::ThirtySecond => 18,
            Resolution::FifteenSecond => 17,
            Resolution::FiveSecond => 15,
            Resolution::Second => 12,
            Resolution::FiveHundredMilli => 11,
            Resolution::HundredMilli => 8,
            Resolution::FiftyMilli => 7,
            Resolution::TenMilli => 4,
            Resolution::FiveMilli => 3,
            Resolution::Millisecond => 0,
        }
    }

    pub(crate) fn from_trailing_zeros(x: u8) -> Self {
        match x {
            0 => Resolution::Millisecond,
            3 => Resolution::FiveMilli,
            4 => Resolution::TenMilli,
            7 => Resolution::FiftyMilli,
            8 => Resolution::HundredMilli,
            11 => Resolution::FiveHundredMilli,
            12 => Resolution::Second,
            15 => Resolution::FiveSecond,
            17 => Resolution::FifteenSecond,
            18 => Resolution::ThirtySecond,
            19 => Resolution::Minute,
            22 => Resolution::FiveMinute,
            24 => Resolution::FifteenMinute,
            25 => Resolution::ThirtyMinute,
            26 => Resolution::Hour,
            28 => Resolution::ThreeHour,
            29 => Resolution::TimeOfDay,
            30 => Resolution::AmPm,
            31 => Resolution::Day,
            _ => panic!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x_in_y() {
        assert_eq!(Resolution::Minute / Resolution::Second, 60);
        assert_eq!(Resolution::Hour / Resolution::Minute, 60);
        assert_eq!(Resolution::Day / Resolution::Hour, 24);
    }

    #[test]
    fn test_enough_bits() {
        for res in Resolution::variants() {
            let has = res.n_bits() as u32;
            let required = if res.subdivision() == 0 {
                0
            } else if res.subdivision().is_power_of_two() {
                (res.subdivision() as u32).ilog2()
            } else {
                (res.subdivision() as u32).ilog2() + 1
            };
            assert!(
                has == required,
                "{res:?}: {has} != log2({})={required}",
                res.subdivision()
            );
        }
    }

    #[test]
    fn test_mask() {
        let mask =
            |res: Resolution| -> u32 { !(u32::MAX << res.n_bits()) << res.trailing_zeros() + 1 };
        assert_eq!(mask(Resolution::Second), 0b1110_000000000000);
        assert_eq!(mask(Resolution::FiveSecond), 0b110000_000000000000);
        assert_eq!(mask(Resolution::FifteenSecond), 0b1000000_000000000000);
        assert_eq!(mask(Resolution::ThirtySecond), 0b10000000_000000000000);
    }

    #[test]
    fn test_range() {
        assert_eq!(
            Resolution::range(Resolution::Second, Resolution::Minute).collect::<Vec<_>>(),
            vec![
                Resolution::Second,
                Resolution::FiveSecond,
                Resolution::FifteenSecond,
                Resolution::ThirtySecond,
            ]
        );
    }

    #[test]
    fn test_trailing_zeros() {
        for res in Resolution::variants() {
            assert_eq!(Resolution::from_trailing_zeros(res.trailing_zeros()), res)
        }
    }

    #[test]
    fn test_n_bits() {
        for res in Resolution::variants() {
            let n_bits = res.coarser().map_or(31, |x| x.trailing_zeros()) - res.trailing_zeros();
            assert_eq!(res.n_bits(), n_bits, "{res:?}",)
        }
    }

    #[test]
    fn test_width() {
        for (res1, res2) in Resolution::variants()
            .rev()
            .zip(Resolution::variants().rev().skip(1))
        {
            assert_eq!(
                res1.width() * res1.subdivision() as u32,
                res2.width(),
                "{res1:?}"
            )
        }
    }
}

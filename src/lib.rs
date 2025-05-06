mod types;

use crate::types::*;
use linearize::{Linearize, LinearizeExt};
use std::{fmt, num::NonZero, ops::Div};

/// A time with a resolution
///
/// Including a resolution means each `ResTime` value actually identifies a
/// certain time _interval_.  You can lower the resolution, which increases the
/// width of the interval.
///
/// The intervals at one resolution all fall completely inside a single interval
/// at any lower resolution.  This means that the intervals representable by
/// `ResTime` form a tree, where each node is contained by its parent, and
/// spanned by its children.  You can think of values of `ResTime` as paths into
/// that tree, and the resolution is how deep into the tree the path goes.
///
/// At maximum resolution, this type identifies a specific second within a year.
/// You can think of it as a "second-of-year" type, but where the bit pattern
/// is designed to go through all the commonly-used time units (week, half-hour,
/// etc).  Lower-resolution variants are the same type, but with some number of
/// lower bits unavailable.  In other words, the resolution is simply the number
/// of available bits.
///
/// We indicate the resolution with a unary encoding in the lower bits.
/// Starting from the LSB, there are some number of zeroes, followed by a one.
/// The number of zeroes gives you the resolution.  After the one, all bits
/// represent actual time data.
///
/// This is a nice encoding, because (A) storing the resolution only costs a
/// single bit, and (B) the positions of the time-data bits always have the same
/// meaning, regardless of resolution (eg. bit 18, if it exists, always tells
/// you whether it's morning or afternoon).  Since there's always a one-bit
/// somewhere, the "all-zeroes" bit pattern is invalid and can be used to
/// represent th e `None` case of `Option<ResTime>`.
///
/// The `Eq` impl considers resolution significant.  Two `ResTime`s at
/// different resolutions will never compare equal, even if one contains
/// the other. Likewise, ordering only exists between `ResTime`s of the same
/// resolution. Within a resolution, `ResTime`s are totally ordered in the
/// expected way. If you want to compare values of different resolutions, see
/// [ResTime::coarse_cmp].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResTime(NonZero<u32>);

impl PartialOrd for ResTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.0.trailing_zeros() == other.0.trailing_zeros() {
            Some(self.0.cmp(&other.0))
        } else {
            None
        }
    }
}

// Bits:
//
// 00 => millis
// 01 => 2ms [SKIP]
// 02 => 4ms [SKIP]
// 03 => 8ms [SKIP]
// 04 => 10s of millis
// 05 => 20ms [SKIP]
// 06 => 40ms [SKIP]
// 07 => 90ms [SKIP]
// 08 => 100s of millis
// 09 => 200ms [SKIP]
// 10 => 400ms [SKIP]
// 11 => 800ms [SKIP]
// 12 => second
// 13 => 2s [SKIP]
// 14 => 4s [SKIP]
// 15 => 5s
// 16 => 10s [SKIP]
// 17 => 15s
// 18 => 30s
// 19 => minute
// 20 => 2m [SKIP]
// 21 => 4m [SKIP]
// 22 => 5m
// 23 => 10m [SKIP]
// 24 => 15m
// 25 => 30m
// 26 => hour
// 27 => 2h [SKIP]
// 28 => 3h
// 29 => 6h
// 30 => 12h (am/pm)

impl ResTime {
    pub fn resolution(self) -> Resolution {
        Resolution::from_trailing_zeros(self.0.trailing_zeros() as u8)
    }

    pub fn reduce_to(&mut self, res: Resolution) {
        assert!(res <= self.resolution());
        let mut x = self.0.get();
        x &= u32::MAX << res.trailing_zeros();
        x |= 1 << res.trailing_zeros();
        self.0 = NonZero::new(x).unwrap();
    }

    pub fn with_res(self, res: Resolution) -> Option<Self> {
        if res > self.resolution() {
            return None;
        }
        let mut x = self.0.get();
        x &= u32::MAX << res.trailing_zeros();
        x |= 1 << res.trailing_zeros();
        Some(ResTime(NonZero::new(x).unwrap()))
    }

    /// Compare two values by first coarsening them to the lower of their two
    /// resolutions.  This gives results consistent with `partial_cmp()`, but
    /// not `eq()`.  This function will return Ordering::Eq when one value is
    /// inside the other, whereas `eq()` would return `false`.
    pub fn coarse_cmp(self, other: ResTime) -> std::cmp::Ordering {
        let zeroes = self.0.trailing_zeros().max(other.0.trailing_zeros());
        let mut x = self.0.get();
        x &= u32::MAX << zeroes;
        x |= 1 << zeroes;
        let mut y = other.0.get();
        y &= u32::MAX << zeroes;
        y |= 1 << zeroes;
        x.cmp(&y)
    }
}

impl Default for ResTime {
    fn default() -> Self {
        Self::WHOLE_DAY
    }
}

impl ResTime {
    pub const WHOLE_DAY: Self =
        ResTime(NonZero::new(0b10000000_00000000_00000000_00000000).unwrap());

    pub fn new() -> Self {
        Self::WHOLE_DAY
    }
}

// TODO: Simplify this code by using `Resolution::subdivision()`
impl ResTime {
    /// `from` is inclusive, `to` is exclusive.  `from` should be finer than
    /// `to`.
    fn with_res_bits(self, from: Resolution, to: Resolution, x: u8) -> Option<Self> {
        // TODO: We could allow setting eg. the month of an `ResTime` which
        // already has second resolution.  But we'd need to be consider whether
        // we want to allow eg. starting with February, setting the quarter to
        // Q4, and ending up with November.  Is that what people want?  Maybe!
        if self.resolution() != to {
            return None;
        }
        let mut bits = self.0.get();
        bits &= !Resolution::mask_all(from, to); // clear the range
        // There shouldn't be any actual data there, but the "data starts
        // here" marker bit needs to be cleared.
        let mut x = x as u32;
        x <<= 1;
        x |= 0b1; // The new marker bit
        x <<= from.trailing_zeros();
        bits |= x as u32;
        Some(ResTime(NonZero::new(bits).unwrap()))
    }

    pub fn try_with_meridian(self, x: Meridian) -> Option<Self> {
        self.with_res_bits(Resolution::Meridian, Resolution::Day, x.into())
    }

    pub fn try_with_time_of_day(self, x: TimeOfDay) -> Option<Self> {
        self.with_res_bits(Resolution::TimeOfDay, Resolution::Day, x.into())
    }

    pub fn try_with_hour(self, x: u8) -> Option<Self> {
        if x > 23 {
            return None;
        }
        self.with_res_bits(Resolution::ThreeHour, Resolution::Day, x / 3)?
            .with_res_bits(Resolution::Hour, Resolution::ThreeHour, x % 3)
    }

    pub fn try_with_minute(self, x: u8) -> Option<Self> {
        if x > 59 {
            return None;
        }
        self.with_res_bits(Resolution::FifteenMinute, Resolution::Hour, x / 15)?
            .with_res_bits(
                Resolution::FiveMinute,
                Resolution::FifteenMinute,
                (x % 15) / 5,
            )?
            .with_res_bits(Resolution::Minute, Resolution::FiveMinute, x % 5)
    }

    pub fn try_with_second(self, x: u8) -> Option<Self> {
        if x > 59 {
            return None;
        }
        self.with_res_bits(Resolution::FifteenSecond, Resolution::Minute, x / 15)?
            .with_res_bits(
                Resolution::FiveSecond,
                Resolution::FifteenSecond,
                (x % 15) / 5,
            )?
            .with_res_bits(Resolution::Second, Resolution::FiveSecond, x % 5)
    }

    pub fn try_with_millis(self, x: u16) -> Option<Self> {
        if x > 999 {
            return None;
        }
        self.with_res_bits(
            Resolution::HundredMilli,
            Resolution::Second,
            (x / 100) as u8,
        )?
        .with_res_bits(
            Resolution::TenMilli,
            Resolution::HundredMilli,
            (x / 10) as u8,
        )?
        .with_res_bits(
            Resolution::Millisecond,
            Resolution::TenMilli,
            (x % 10) as u8,
        )
    }
}

impl ResTime {
    pub fn with_meridian(self, x: Meridian) -> Self {
        self.try_with_meridian(x).unwrap_or(self)
    }
    pub fn with_time_of_day(self, x: TimeOfDay) -> Self {
        self.try_with_time_of_day(x).unwrap_or(self)
    }
    /// 0-23
    pub fn with_hour(self, x: u8) -> Self {
        self.try_with_hour(x).unwrap_or(self)
    }
    /// 0-59
    pub fn with_minute(self, x: u8) -> Self {
        self.try_with_minute(x).unwrap_or(self)
    }
    /// 0-59
    pub fn with_second(self, x: u8) -> Self {
        self.try_with_second(x).unwrap_or(self)
    }
    /// 0-999
    pub fn with_millis(self, x: u16) -> Self {
        self.try_with_millis(x).unwrap_or(self)
    }

    /// 0-999
    pub fn set_millis(&mut self, x: u16) {
        *self = self.with_millis(x);
    }
    /// 0-59
    pub fn set_second(&mut self, x: u8) {
        *self = self.with_second(x);
    }
    /// 0-59
    pub fn set_minute(&mut self, x: u8) {
        *self = self.with_minute(x);
    }
    /// 0-23
    pub fn set_hour(&mut self, x: u8) {
        *self = self.with_hour(x);
    }
    pub fn set_time_of_day(&mut self, x: TimeOfDay) {
        *self = self.with_time_of_day(x);
    }
    pub fn set_meridian(&mut self, x: Meridian) {
        *self = self.with_meridian(x);
    }
}

impl ResTime {
    /// `from` is inclusive, `to` is exclusive.  `from` should be finer than
    /// `to`.
    fn get_res_bits(self, from: Resolution, to: Resolution) -> Option<u8> {
        if self.resolution() < from {
            return None;
        }
        let mut bits = self.0.get();
        bits >>= from.trailing_zeros() + 1;
        let n_bits = from.available_bits() - to.available_bits();
        bits &= !(u32::MAX << n_bits);
        Some(bits as u8)
    }

    /// 0-999
    pub fn millis(self) -> Option<u16> {
        let mut ret = self.get_res_bits(Resolution::HundredMilli, Resolution::Second)? as u16 * 100;
        ret += self
            .get_res_bits(Resolution::TenMilli, Resolution::HundredMilli)
            .unwrap_or(0) as u16
            * 10;
        ret += self
            .get_res_bits(Resolution::Millisecond, Resolution::TenMilli)
            .unwrap_or(0) as u16;
        Some(ret)
    }
    /// 0-59
    // FIXME: Returns None when resolution=30s
    pub fn second(self) -> Option<u8> {
        let mut ret = self.get_res_bits(Resolution::FifteenSecond, Resolution::Minute)? * 15;
        ret += self
            .get_res_bits(Resolution::FiveSecond, Resolution::FifteenSecond)
            .unwrap_or(0)
            * 5;
        ret += self
            .get_res_bits(Resolution::Second, Resolution::FiveSecond)
            .unwrap_or(0);
        Some(ret)
    }
    /// 0-59
    // FIXME: Returns None when resolution=30m
    pub fn minute(self) -> Option<u8> {
        let mut ret = self.get_res_bits(Resolution::FifteenMinute, Resolution::Hour)? * 15;
        ret += self
            .get_res_bits(Resolution::FiveMinute, Resolution::FifteenMinute)
            .unwrap_or(0)
            * 5;
        ret += self
            .get_res_bits(Resolution::Minute, Resolution::FiveMinute)
            .unwrap_or(0);
        Some(ret)
    }
    /// 0-23
    // FIXME: Returns None when resolution=6h/12h
    pub fn hour(self) -> Option<u8> {
        let mut ret = self.get_res_bits(Resolution::ThreeHour, Resolution::Day)? * 3;
        ret += self
            .get_res_bits(Resolution::Hour, Resolution::ThreeHour)
            .unwrap_or(0);
        Some(ret)
    }
    pub fn time_of_day(self) -> Option<TimeOfDay> {
        let x = self.get_res_bits(Resolution::TimeOfDay, Resolution::Day)?;
        Some(x.try_into().unwrap())
    }
    pub fn meridian(self) -> Option<Meridian> {
        let x = self.get_res_bits(Resolution::Meridian, Resolution::Day)?;
        Some(x.try_into().unwrap())
    }
}

impl fmt::Display for ResTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(hour) = self.hour() else {
            if let Some(x) = self.time_of_day() {
                write!(f, "{x}")?;
            } else if let Some(x) = self.meridian() {
                write!(f, "{x}")?;
            } else {
                write!(f, "whole day")?;
            }
            return Ok(());
        };
        write!(f, "{hour:02}")?;
        if let Some(minute) = self.minute() {
            write!(f, ":{minute:02}")?;
        } else {
            write!(f, ":00")?;
            return Ok(());
        }
        if let Some(second) = self.second() {
            write!(f, ":{second:02}")?;
        }
        if let Some(millis) = self.millis() {
            eprintln!("{millis}");
            match self.resolution() {
                Resolution::HundredMilli => write!(f, ".{:01}", millis / 100)?,
                Resolution::TenMilli => write!(f, ".{:02}", millis / 10)?,
                Resolution::Millisecond => write!(f, ".{millis:03}")?,
                _ => panic!(),
            }
        }
        Ok(())
    }
}

/// There are 13 resolutions available:
///
/// * second, 5s, 15s, 30s
/// * minute, 5m, 15m, 30m
/// * hour, 3h, 6h, 12h (am/pm)
///
/// The `Ord` impl follows natural-language: `x < y` means that x is
/// lower-resolution than y.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum Resolution {
    Day,
    Meridian,
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
    // TODO: We could add 500ms
    HundredMilli,
    // TODO: We could add 50ms
    TenMilli,
    Millisecond,
}

impl Resolution {
    pub fn coarser(self) -> Option<Self> {
        Resolution::from_linear(self.linearize().checked_sub(1)?)
    }

    pub fn finer(self) -> Option<Self> {
        Resolution::from_linear(self.linearize().checked_add(1)?)
    }
}

impl Resolution {
    fn available_bits(self) -> u8 {
        match self {
            Resolution::Day => 0,
            Resolution::Meridian => 1,
            Resolution::TimeOfDay => 2,
            Resolution::ThreeHour => 3,
            Resolution::Hour => 5,
            Resolution::ThirtyMinute => 6,
            Resolution::FifteenMinute => 7,
            Resolution::FiveMinute => 9,
            Resolution::Minute => 12,
            Resolution::ThirtySecond => 13,
            Resolution::FifteenSecond => 14,
            Resolution::FiveSecond => 16,
            Resolution::Second => 19,
            Resolution::HundredMilli => 23,
            Resolution::TenMilli => 27,
            Resolution::Millisecond => 31,
        }
    }

    fn n_bits(self) -> u8 {
        self.available_bits() - self.coarser().map_or(0, |x| x.available_bits())
    }

    fn mask(self) -> u32 {
        !(u32::MAX << self.n_bits()) << self.trailing_zeros() + 1
    }

    fn mask_all(from: Resolution, to: Resolution) -> u32 {
        Resolution::range(from, to).fold(0, |mask, res| mask | res.mask())
    }

    fn trailing_zeros(self) -> u8 {
        31 - self.available_bits()
    }

    fn from_trailing_zeros(x: u8) -> Self {
        match x {
            00 => Resolution::Millisecond,
            04 => Resolution::TenMilli,
            08 => Resolution::HundredMilli,
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
            30 => Resolution::Meridian,
            31 => Resolution::Day,
            _ => panic!(),
        }
    }

    fn subdivision(self) -> u8 {
        match self {
            Resolution::Day => 0,
            Resolution::Meridian => 2,
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
            Resolution::HundredMilli => 10,
            Resolution::TenMilli => 10,
            Resolution::Millisecond => 10,
        }
    }

    /// `from` is inclusive, `to` is exclusive.  `from` should be finer than
    /// `to`.
    fn range(from: Resolution, to: Resolution) -> impl Iterator<Item = Resolution> {
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

// #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
// pub struct Resolution(u8);

// impl Resolution {
//     pub const MAX: Resolution = Resolution(29);

//     pub const YEAR: Resolution = Resolution(29);
//     pub const DAY: Resolution = Resolution(19);
//     pub const MINUTE: Resolution = Resolution(7);
//     pub const SECOND: Resolution = Resolution(0);

//     fn is_valid(self) -> bool {
//         !matches!(
//             self.0,
//             1 | 2 | 4 | 8 | 9 | 11 | 15 | 20 | 21 | 23 | 24 | 26 | 29..
//         )
//     }

//     // fn is_major(self) -> bool {
//     //     matches!(self.0, 0 | 7 | 14 | 19 | 22 | 25)
//     // }

//     pub fn next_coarser(mut self) -> Option<Self> {
//         loop {
//             self.0 = self.0.checked_sub(1)?;
//             if self.is_valid() {
//                 return Some(self);
//             }
//         }
//     }
// }

// impl fmt::Display for Resolution {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self.0 {
//             0 => f.write_str("second"),
//             3 => f.write_str("5-second"),
//             5 => f.write_str("15-second"),
//             6 => f.write_str("30-second"),
//             7 => f.write_str("minute"),
//             10 => f.write_str("5-minute"),
//             12 => f.write_str("15-minute"),
//             13 => f.write_str("30-minute"),
//             14 => f.write_str("hour"),
//             15 => f.write_str("2-hour"),
//             16 => f.write_str("3-hour"),
//             17 => f.write_str("6-hour"),
//             18 => f.write_str("am/pm"),
//             19 => f.write_str("day"),
//             22 => f.write_str("week"),
//             25 => f.write_str("month"),
//             27 => f.write_str("quarter"),
//             28 => f.write_str("half"),
//             29 => f.write_str("year"),
//             _ => panic!(),
//         }
//     }
// }

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
        assert_eq!(Resolution::Second.mask(), 0b1110_000000000000);
        assert_eq!(Resolution::FiveSecond.mask(), 0b110000_000000000000);
        assert_eq!(Resolution::FifteenSecond.mask(), 0b1000000_000000000000);
        assert_eq!(Resolution::ThirtySecond.mask(), 0b10000000_000000000000);
        assert_eq!(
            Resolution::mask_all(Resolution::Second, Resolution::Minute),
            0b11111110_000000000000
        );
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
    fn test_set_get() {
        let mut x = ResTime::default();
        x.set_hour(11);
        for minute in 0..59 {
            let x = x.with_minute(minute);
            assert_eq!(x.minute(), Some(minute), "{:#b}", x.0);
        }
        x.set_minute(43);
        for second in 0..59 {
            let x = x.with_second(second);
            assert_eq!(x.second(), Some(second), "{:#b}", x.0);
        }
    }

    #[test]
    fn test_trailing_zeros() {
        for res in Resolution::variants() {
            assert_eq!(Resolution::from_trailing_zeros(res.trailing_zeros()), res)
        }
    }

    #[test]
    fn test_fmt() {
        let mut x = ResTime::default();
        assert_eq!(x.to_string(), "whole day");
        x.set_hour(11);
        assert_eq!(x.to_string(), "11:00");
        x.set_minute(56);
        assert_eq!(x.to_string(), "11:56");
        x.set_second(24);
        assert_eq!(x.to_string(), "11:56:24");
        x.reduce_to(Resolution::FiveSecond);
        assert_eq!(x.to_string(), "11:56:20");
        x.reduce_to(Resolution::FifteenSecond);
        assert_eq!(x.to_string(), "11:56:15");
        x.reduce_to(Resolution::Minute);
        assert_eq!(x.to_string(), "11:56");
        x.reduce_to(Resolution::FiveMinute);
        assert_eq!(x.to_string(), "11:55");
        x.reduce_to(Resolution::FifteenMinute);
        assert_eq!(x.to_string(), "11:45");
        x.reduce_to(Resolution::Hour);
        assert_eq!(x.to_string(), "11:00");
        x.reduce_to(Resolution::Day);
        assert_eq!(x.to_string(), "whole day");
    }

    #[test]
    fn example() {
        let t = ResTime::new()
            .with_hour(15)
            .with_minute(7)
            .with_second(24)
            .with_millis(75);
        eprintln!("{:#b}", t.0);
        eprintln!("{:?}", t.resolution());
        assert_eq!(t.to_string(), "15:07:24.075");
        for res in Resolution::variants() {
            eprintln!("{res:?}");
            let actual = t.with_res(res).unwrap().to_string();
            let expected = match res {
                Resolution::Millisecond => "15:07:24.075",
                Resolution::TenMilli => "15:07:24.07",
                Resolution::HundredMilli => "15:07:24.0",
                Resolution::Second => "15:07:24",
                Resolution::FiveSecond => "15:07:20",
                Resolution::FifteenSecond => "15:07:15",
                // FIXME: Should be: "15:07:00",
                Resolution::ThirtySecond => "15:07",
                Resolution::Minute => "15:07",
                Resolution::FiveMinute => "15:05",
                Resolution::FifteenMinute => "15:00",
                Resolution::ThirtyMinute => "15:00",
                Resolution::Hour => "15:00",
                Resolution::ThreeHour => "15:00",
                Resolution::TimeOfDay => "afternoon",
                Resolution::Meridian => "PM",
                Resolution::Day => "whole day",
            };
            assert_eq!(actual, expected);
        }
    }
}

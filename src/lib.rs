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
// 00 => second
// 01 => 2s [SKIP]
// 02 => 4s [SKIP]
// 03 => 5s
// 04 => 10s [SKIP]
// 05 => 15s
// 06 => 30s
// 07 => minute
// 08 => 2m [SKIP]
// 09 => 4m [SKIP]
// 10 => 5m
// 11 => 10m [SKIP]
// 12 => 15m
// 13 => 30m
// 14 => hour
// 15 => 2h [SKIP]
// 16 => 3h
// 17 => 6h
// 18 => 12h (am/pm)
// 19 => weekday
// 20 => 2d [SKIP]
// 21 => 4d [SKIP]
// 22 => week-of-month
// 23 => 2w [SKIP]
// 24 => 4w [SKIP]
// 25 => month
// 26 => 2m [SKIP]
// 27 => quarter
// 28 => half
// 29 => year

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
        Self::THIS_YEAR
    }
}

impl ResTime {
    pub const LAST_YEAR: Self =
        ResTime(NonZero::new(0b00100000_00000000_00000000_00000000).unwrap());
    pub const THIS_YEAR: Self =
        ResTime(NonZero::new(0b01100000_00000000_00000000_00000000).unwrap());
    pub const NEXT_YEAR: Self =
        ResTime(NonZero::new(0b10100000_00000000_00000000_00000000).unwrap());

    pub fn new() -> Self {
        Self::THIS_YEAR
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

    pub fn try_with_half(self, x: Half) -> Option<Self> {
        self.with_res_bits(Resolution::Half, Resolution::Year, x.into())
    }

    pub fn try_with_quarter(self, x: Quarter) -> Option<Self> {
        self.with_res_bits(Resolution::Quarter, Resolution::Year, x.into())
    }

    pub fn try_with_month(self, x: Month) -> Option<Self> {
        let x: u8 = x.into();
        let quarter = x / 3;
        let month = x % 3;
        self.with_res_bits(Resolution::Quarter, Resolution::Year, quarter)?
            .with_res_bits(Resolution::Month, Resolution::Quarter, month)
    }

    pub fn try_with_week(self, x: Week) -> Option<Self> {
        self.with_res_bits(Resolution::Week, Resolution::Month, x.into())
    }

    pub fn try_with_day(self, x: Weekday) -> Option<Self> {
        self.with_res_bits(Resolution::Day, Resolution::Week, x.into())
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
}

impl ResTime {
    pub fn with_half(self, x: Half) -> Self {
        self.try_with_half(x).unwrap_or(self)
    }
    pub fn with_quarter(self, x: Quarter) -> Self {
        self.try_with_quarter(x).unwrap_or(self)
    }
    pub fn with_month(self, x: Month) -> Self {
        self.try_with_month(x).unwrap_or(self)
    }
    pub fn with_week(self, x: Week) -> Self {
        self.try_with_week(x).unwrap_or(self)
    }
    pub fn with_day(self, x: Weekday) -> Self {
        self.try_with_day(x).unwrap_or(self)
    }
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
    pub fn set_day(&mut self, x: Weekday) {
        *self = self.with_day(x);
    }
    pub fn set_week(&mut self, x: Week) {
        *self = self.with_week(x);
    }
    pub fn set_month(&mut self, x: Month) {
        *self = self.with_month(x);
    }
    pub fn set_quarter(&mut self, x: Quarter) {
        *self = self.with_quarter(x);
    }
    pub fn set_half(&mut self, x: Half) {
        *self = self.with_half(x);
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
    pub fn day(self) -> Option<Weekday> {
        let x = self.get_res_bits(Resolution::Day, Resolution::Week)?;
        Some(x.try_into().unwrap())
    }
    pub fn week(self) -> Option<Week> {
        let x = self.get_res_bits(Resolution::Week, Resolution::Month)?;
        Some(x.try_into().unwrap())
    }
    pub fn month(self) -> Option<Month> {
        let mo = self.get_res_bits(Resolution::Month, Resolution::Quarter)?;
        let qu = self.get_res_bits(Resolution::Quarter, Resolution::Year)?;
        let x = qu * 3 + mo;
        Some(x.try_into().unwrap())
    }
    pub fn quarter(self) -> Option<Quarter> {
        let x = self.get_res_bits(Resolution::Quarter, Resolution::Year)?;
        Some(x.try_into().unwrap())
    }
    pub fn half(self) -> Option<Half> {
        let x = self.get_res_bits(Resolution::Half, Resolution::Year)?;
        Some(x.try_into().unwrap())
    }
    fn year(self) -> Year {
        let mut bits = self.0.get();
        bits >>= Resolution::Year.trailing_zeros() + 1;
        let n_bits = Resolution::Year.available_bits();
        bits &= !(u32::MAX << n_bits);
        (bits as u8).try_into().unwrap()
    }
}

impl fmt::Display for ResTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.year())?;
        if let Some(month) = self.month() {
            write!(f, "{month}")?;
        } else if let Some(quarter) = self.quarter() {
            write!(f, "{quarter}")?;
            return Ok(());
        } else if let Some(half) = self.half() {
            write!(f, "{half}")?;
            return Ok(());
        }
        let Some(week) = self.week() else {
            return Ok(());
        };
        write!(f, "-{week}")?;
        let Some(day) = self.day() else { return Ok(()) };
        write!(f, " {day}")?;
        let Some(hour) = self.hour() else {
            if let Some(x) = self.time_of_day() {
                write!(f, " {x}")?;
            } else if let Some(x) = self.meridian() {
                write!(f, " {x}")?;
            }
            return Ok(());
        };
        write!(f, " {hour:02}")?;
        if let Some(minute) = self.minute() {
            write!(f, ":{minute:02}")?;
        } else {
            write!(f, ":00")?;
            return Ok(());
        }
        if let Some(second) = self.second() {
            write!(f, ":{second:02}")?;
        }
        Ok(())
    }
}

/// There are 17 resolutions available:
///
/// * second, 5s, 15s, 30s
/// * minute, 5m, 15m, 30m
/// * hour, 3h, 6h, 12h (am/pm)
/// * day (mon/tues/...)
/// * week (w1, ..., w5)
/// * month, quarter, half
/// * year
///
/// The `Ord` impl follows natural-language: `x < y` means that x is
/// lower-resolution than y.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum Resolution {
    Year,
    Half,
    Quarter,
    Month,
    Week,
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
            Resolution::Year => 2,
            Resolution::Half => 3,
            Resolution::Quarter => 4,
            Resolution::Month => 6,
            Resolution::Week => 9,
            Resolution::Day => 12,
            Resolution::Meridian => 13,
            Resolution::TimeOfDay => 14,
            Resolution::ThreeHour => 15,
            Resolution::Hour => 17,
            Resolution::ThirtyMinute => 18,
            Resolution::FifteenMinute => 19,
            Resolution::FiveMinute => 21,
            Resolution::Minute => 24,
            Resolution::ThirtySecond => 25,
            Resolution::FifteenSecond => 26,
            Resolution::FiveSecond => 28,
            Resolution::Second => 31,
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
            0 => Resolution::Second,
            3 => Resolution::FiveSecond,
            5 => Resolution::FifteenSecond,
            6 => Resolution::ThirtySecond,
            7 => Resolution::Minute,
            10 => Resolution::FiveMinute,
            12 => Resolution::FifteenMinute,
            13 => Resolution::ThirtyMinute,
            14 => Resolution::Hour,
            16 => Resolution::ThreeHour,
            17 => Resolution::TimeOfDay,
            18 => Resolution::Meridian,
            19 => Resolution::Day,
            22 => Resolution::Week,
            25 => Resolution::Month,
            27 => Resolution::Quarter,
            28 => Resolution::Half,
            29 => Resolution::Year,
            _ => panic!(),
        }
    }

    fn subdivision(self) -> u8 {
        match self {
            Resolution::Year => 4,
            Resolution::Half => 2,
            Resolution::Quarter => 2,
            Resolution::Month => 3,
            Resolution::Week => 5,
            Resolution::Day => 7,
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
            let required = if res.subdivision().is_power_of_two() {
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
        assert_eq!(Resolution::Second.mask(), 0b1110);
        assert_eq!(Resolution::FiveSecond.mask(), 0b110000);
        assert_eq!(Resolution::FifteenSecond.mask(), 0b1000000);
        assert_eq!(Resolution::ThirtySecond.mask(), 0b10000000);
        assert_eq!(
            Resolution::mask_all(Resolution::Second, Resolution::Minute),
            0b11111110
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
        for half in Half::variants() {
            let x = x.with_half(half);
            assert_eq!(x.half(), Some(half), "{:#b}", x.0);
        }
        for quarter in Quarter::variants() {
            assert_eq!(x.with_quarter(quarter).quarter(), Some(quarter));
        }
        for month in Month::variants() {
            let x = x.with_month(month);
            assert_eq!(x.month(), Some(month), "{:#b}", x.0);
        }
        x.set_month(Month::May);
        for week in Week::variants() {
            let x = x.with_week(week);
            assert_eq!(x.week(), Some(week), "{:#b}", x.0);
        }
        x.set_week(Week::W3);
        for day in Weekday::variants() {
            let x = x.with_day(day);
            assert_eq!(x.day(), Some(day), "{:#b}", x.0);
        }
        x.set_day(Weekday::Tuesday);
        for hour in 0..23 {
            let x = x.with_hour(hour);
            assert_eq!(x.hour(), Some(hour), "{:#b}", x.0);
        }
        x.set_hour(11);
        for minute in 0..59 {
            let x = x.with_minute(minute);
            assert_eq!(x.minute(), Some(minute), "{:#b}", x.0);
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
        assert_eq!(x.to_string(), "");
        assert_eq!(x.with_half(Half::H2).to_string(), "H2");
        assert_eq!(x.with_quarter(Quarter::Q3).to_string(), "Q3");
        x.set_month(Month::April);
        assert_eq!(x.to_string(), "Apr");
        x.set_week(Week::W3);
        assert_eq!(x.to_string(), "Apr-w3");
        x.set_day(Weekday::Tuesday);
        assert_eq!(x.to_string(), "Apr-w3 Tue");
        x.set_hour(11);
        assert_eq!(x.to_string(), "Apr-w3 Tue 11:00");
        x.set_minute(56);
        assert_eq!(x.to_string(), "Apr-w3 Tue 11:56");
        x.set_second(24);
        assert_eq!(x.to_string(), "Apr-w3 Tue 11:56:24");
        eprintln!("{:#b}", x.0);
        x.reduce_to(Resolution::FiveSecond);
        eprintln!("{:#b}", x.0);
        assert_eq!(x.to_string(), "Apr-w3 Tue 11:56:20");
        x.reduce_to(Resolution::FifteenSecond);
        assert_eq!(x.to_string(), "Apr-w3 Tue 11:56:15");
        x.reduce_to(Resolution::Minute);
        assert_eq!(x.to_string(), "Apr-w3 Tue 11:56");
        x.reduce_to(Resolution::FiveMinute);
        assert_eq!(x.to_string(), "Apr-w3 Tue 11:55");
        x.reduce_to(Resolution::FifteenMinute);
        assert_eq!(x.to_string(), "Apr-w3 Tue 11:45");
        x.reduce_to(Resolution::Hour);
        assert_eq!(x.to_string(), "Apr-w3 Tue 11:00");
        x.reduce_to(Resolution::Day);
        assert_eq!(x.to_string(), "Apr-w3 Tue");
        x.reduce_to(Resolution::Week);
        assert_eq!(x.to_string(), "Apr-w3");
        x.reduce_to(Resolution::Month);
        assert_eq!(x.to_string(), "Apr");
        x.reduce_to(Resolution::Quarter);
        assert_eq!(x.to_string(), "Q2");
        x.reduce_to(Resolution::Half);
        assert_eq!(x.to_string(), "H1");
        x.reduce_to(Resolution::Year);
        assert_eq!(x.to_string(), "");
    }

    #[test]
    fn example() {
        let t = ResTime::new()
            .with_month(Month::October)
            .with_week(Week::W2)
            .with_day(Weekday::Thursday)
            .with_hour(15)
            .with_minute(7)
            .with_second(24);
        assert_eq!(t.to_string(), "Oct-w2 Thu 15:07:24");
        for res in Resolution::variants() {
            let actual = t.with_res(res).unwrap().to_string();
            let expected = match res {
                Resolution::Second => "Oct-w2 Thu 15:07:24",
                Resolution::FiveSecond => "Oct-w2 Thu 15:07:20",
                Resolution::FifteenSecond => "Oct-w2 Thu 15:07:15",
                // FIXME: Should be: "Oct-w2 Thu 15:07:00",
                Resolution::ThirtySecond => "Oct-w2 Thu 15:07",
                Resolution::Minute => "Oct-w2 Thu 15:07",
                Resolution::FiveMinute => "Oct-w2 Thu 15:05",
                Resolution::FifteenMinute => "Oct-w2 Thu 15:00",
                Resolution::ThirtyMinute => "Oct-w2 Thu 15:00",
                Resolution::Hour => "Oct-w2 Thu 15:00",
                Resolution::ThreeHour => "Oct-w2 Thu 15:00",
                Resolution::TimeOfDay => "Oct-w2 Thu afternoon",
                Resolution::Meridian => "Oct-w2 Thu PM",
                Resolution::Day => "Oct-w2 Thu",
                Resolution::Week => "Oct-w2",
                Resolution::Month => "Oct",
                Resolution::Quarter => "Q4",
                Resolution::Half => "H2",
                Resolution::Year => "",
            };
            assert_eq!(actual, expected);
        }
    }
}

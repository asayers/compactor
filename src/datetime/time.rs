use super::{AmPm, Resolution, SixHour};
use std::{fmt, num::NonZero};

/// A time with a resolution
///
/// Start with `Time::new()`, which is a timestamp at the lowest resolution
/// possible, and covers the whole day.  Increase the resolution using the
/// `Time::with_*` methods.  Reducude the resolution using [`Time::reduce_to`].
///
/// Consider these two times:
///
/// ```
/// # use compactor::Time;
/// let t1 = Time::new().with_hour(15).with_minute(27).with_second(17);
/// assert_eq!(t1.to_string(), "15:27:17");
///
/// let t2 = t1.with_millis(0);
/// assert_eq!(t2.to_string(), "15:27:17.000");
///
/// assert_ne!(t1, t2);
/// ```
///
/// In most date/time libraries, if you don't specify the number of sub-second
/// millis, it just sets them to zero. As a result, these values like `t1` and
/// `t2` here would be considered equal.  Not so with `compactor::Time`!  Adding
/// `.with_millis(0)` resulted in a _different value_.  Previously the number of
/// sub-second millis was unknown.  Now it is known to be zero.
///
/// ## Equality and ordering
///
/// As illustrated above, the `Eq` impl considers resolution significant.  Two
/// `Time`s at different resolutions will never compare equal, even if one
/// contains the other.
///
/// Likewise, ordering only exists between `Time`s of the same resolution.
/// Within a resolution, `Time`s are totally ordered in the expected way.
/// If you want to compare values of different resolutions, take a look at
/// [Time::coarse_cmp].
///
/// ## A day-spanning tree of time intervals
///
/// Including a resolution means each `Time` value actually identifies a
/// certain time _interval_.  You can lower the resolution, which increases the
/// width of the interval.
///
/// The intervals at one resolution all fall completely inside a single interval
/// at any lower resolution.  This means that the intervals representable by
/// `Time` form a tree, where each node is contained by its parent, and
/// spanned by its children.  You can think of values of `Time` as paths into
/// that tree, and the resolution is how deep into the tree the path goes.
///
/// At maximum resolution, this type identifies a specific second within a year.
/// You can think of it as a "second-of-year" type, but where the bit pattern
/// is designed to go through all the commonly-used time units (week, half-hour,
/// etc).  Lower-resolution variants are the same type, but with some number of
/// lower bits unavailable.  In other words, the resolution is simply the number
/// of available bits.
///
/// ## How it's encoded
///
/// We indicate the resolution with a unary encoding in the lower bits.
/// Starting from the LSB, there are some number of zeroes, followed by a one.
/// The number of zeroes gives you the resolution.  After the one, all bits
/// represent actual time data.
///
/// This is a nice encoding, because (A) storing the resolution only costs a
/// single bit, and (B) the positions of the time-data bits always have the same
/// meaning, regardless of resolution (eg. bit 18, if it exists, always tells
/// you whether it's morning or afternoon).
///
/// A bonus: since there's always a one-bit somewhere, the "all-zeroes"
/// bit pattern is invalid, and can be used to represent the `None` case of
/// `Option<Time>`.
#[derive(Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Time(NonZero<u32>);

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.0.trailing_zeros() == other.0.trailing_zeros() {
            Some(self.0.cmp(&other.0))
        } else {
            None
        }
    }
}

#[cfg(feature = "jiff")]
impl From<jiff::civil::Time> for Time {
    fn from(t: jiff::civil::Time) -> Self {
        Time::new()
            .with_hour(t.hour() as u8)
            .with_minute(t.minute() as u8)
            .with_second(t.second() as u8)
            .with_millis(t.millisecond() as u16)
    }
}

#[cfg(feature = "jiff")]
impl From<Time> for jiff::civil::Time {
    fn from(t: Time) -> Self {
        jiff::civil::time(
            t.hour() as i8,
            t.minute() as i8,
            t.second() as i8,
            t.millis() as i32,
        )
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::NaiveTime> for Time {
    fn from(t: chrono::NaiveTime) -> Self {
        use chrono::Timelike;
        Time::new()
            .with_hour(t.hour() as u8)
            .with_minute(t.minute() as u8)
            .with_second(t.second() as u8)
            .with_millis((t.nanosecond() / 1_000_000) as u16)
    }
}

#[cfg(feature = "chrono")]
impl From<Time> for chrono::NaiveTime {
    fn from(t: Time) -> Self {
        chrono::NaiveTime::from_hms_milli_opt(
            t.hour() as u32,
            t.minute() as u32,
            t.second() as u32,
            t.millis() as u32,
        )
        .unwrap()
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

impl Time {
    pub fn resolution(self) -> Resolution {
        Resolution::from_trailing_zeros(self.0.trailing_zeros() as u8)
    }

    /// Has no effect if `res` is higher than the current resolution
    pub fn reduce_to(&mut self, res: Resolution) {
        if res >= self.resolution() {
            return;
        }
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
        Some(Time(NonZero::new(x).unwrap()))
    }

    /// Expects the data bits to be in their correct positions, but for there to
    /// be no resolution marker.
    fn from_bits(mut x: u32, res: Resolution) -> Self {
        x &= u32::MAX << res.trailing_zeros();
        x |= 1 << res.trailing_zeros();
        Time(NonZero::new(x).unwrap())
    }

    /// Compare two values by first coarsening them to the lower of their two
    /// resolutions.  This gives results consistent with `partial_cmp()`, but
    /// not `eq()`.  This function will return Ordering::Eq when one value is
    /// inside the other, whereas `eq()` would return `false`.
    pub fn coarse_cmp(self, other: Time) -> std::cmp::Ordering {
        let zeroes = self.0.trailing_zeros().max(other.0.trailing_zeros());
        let mut x = self.0.get();
        x &= u32::MAX << zeroes;
        x |= 1 << zeroes;
        let mut y = other.0.get();
        y &= u32::MAX << zeroes;
        y |= 1 << zeroes;
        x.cmp(&y)
    }

    // pub fn start(self) -> jiff::civil::Time {
    //     todo!()
    // }

    // pub fn end(self) -> jiff::civil::Time {
    //     todo!()
    // }
}

impl Default for Time {
    fn default() -> Self {
        Self::WHOLE_DAY
    }
}

impl Time {
    /// Covers the whole day
    pub fn new() -> Self {
        Self::WHOLE_DAY
    }
}

impl Time {
    pub const WHOLE_DAY: Self = Time(NonZero::new(0b10000000_00000000_00000000_00000000).unwrap());

    /// ```
    /// # use compactor::datetime::{AmPm, Time};
    /// assert_eq!(Time::AM, Time::new().with_am_pm(AmPm::AM));
    /// ```
    pub const AM: Self = Time(NonZero::new(0b01000000_00000000_00000000_00000000).unwrap());

    /// ```
    /// # use compactor::datetime::{AmPm, Time};
    /// assert_eq!(Time::PM, Time::new().with_am_pm(AmPm::PM));
    /// ```
    pub const PM: Self = Time(NonZero::new(0b11000000_00000000_00000000_00000000).unwrap());

    /// ```
    /// # use compactor::datetime::{Time, SixHour};
    /// assert_eq!(Time::NIGHT, Time::new().with_time_of_day(SixHour::Night));
    /// ```
    pub const NIGHT: Self = Time(NonZero::new(0b00100000_00000000_00000000_00000000).unwrap());

    /// ```
    /// # use compactor::datetime::{Time, SixHour};
    /// assert_eq!(Time::MORNING, Time::new().with_time_of_day(SixHour::Morning));
    /// ```
    pub const MORNING: Self = Time(NonZero::new(0b01100000_00000000_00000000_00000000).unwrap());

    /// ```
    /// # use compactor::datetime::{Time, SixHour};
    /// assert_eq!(Time::AFTERNOON, Time::new().with_time_of_day(SixHour::Afternoon));
    /// ```
    pub const AFTERNOON: Self = Time(NonZero::new(0b10100000_00000000_00000000_00000000).unwrap());

    /// ```
    /// # use compactor::datetime::{Time, SixHour};
    /// assert_eq!(Time::EVENING, Time::new().with_time_of_day(SixHour::Evening));
    /// ```
    pub const EVENING: Self = Time(NonZero::new(0b11100000_00000000_00000000_00000000).unwrap());

    pub fn from_hour(h: u8) -> Self {
        Time::new().with_hour(h)
    }
}

fn set_res_bits(bits: &mut u32, res: Resolution, x: &mut u32) {
    // Clear the range.  There shouldn't be any actual data there, but the
    // "data starts here" marker bit might be in here.
    let mask = !(u32::MAX << res.n_bits()) << (res.trailing_zeros() + 1);
    *bits &= !mask;
    let subdivision = res.subdivision() as u32;
    *bits |= (*x % subdivision) << (res.trailing_zeros() + 1);
    *x /= subdivision;
}

impl Time {
    pub fn try_with_am_pm(self, x: AmPm) -> Option<Self> {
        if self.resolution() != Resolution::Day {
            return None;
        }
        let mut x = u8::from(x) as u32;
        let mut ret = self.0.get();
        set_res_bits(&mut ret, Resolution::AmPm, &mut x);
        Some(Time::from_bits(ret, Resolution::AmPm))
    }

    pub fn try_with_time_of_day(self, x: SixHour) -> Option<Self> {
        if self.resolution() != Resolution::Day {
            return None;
        }
        let mut x = u8::from(x) as u32;
        let mut ret = self.0.get();
        set_res_bits(&mut ret, Resolution::SixHour, &mut x);
        set_res_bits(&mut ret, Resolution::AmPm, &mut x);
        Some(Time::from_bits(ret, Resolution::SixHour))
    }

    pub fn try_with_hour(self, x: u8) -> Option<Self> {
        if x > 23 {
            return None;
        }
        if self.resolution() != Resolution::Day {
            return None;
        }
        let mut x = x as u32;
        let mut ret = self.0.get();
        set_res_bits(&mut ret, Resolution::Hour, &mut x);
        set_res_bits(&mut ret, Resolution::ThreeHour, &mut x);
        set_res_bits(&mut ret, Resolution::SixHour, &mut x);
        set_res_bits(&mut ret, Resolution::AmPm, &mut x);
        Some(Time::from_bits(ret, Resolution::Hour))
    }

    pub fn try_with_minute(self, x: u8) -> Option<Self> {
        if x > 59 {
            return None;
        }
        if self.resolution() != Resolution::Hour {
            return None;
        }
        let mut x = x as u32;
        let mut ret = self.0.get();
        set_res_bits(&mut ret, Resolution::Minute, &mut x);
        set_res_bits(&mut ret, Resolution::FiveMinute, &mut x);
        set_res_bits(&mut ret, Resolution::FifteenMinute, &mut x);
        set_res_bits(&mut ret, Resolution::ThirtyMinute, &mut x);
        Some(Time::from_bits(ret, Resolution::Minute))
    }

    pub fn try_with_second(self, x: u8) -> Option<Self> {
        if x > 59 {
            return None;
        }
        if self.resolution() != Resolution::Minute {
            return None;
        }
        let mut x = x as u32;
        let mut ret = self.0.get();
        set_res_bits(&mut ret, Resolution::Second, &mut x);
        set_res_bits(&mut ret, Resolution::FiveSecond, &mut x);
        set_res_bits(&mut ret, Resolution::FifteenSecond, &mut x);
        set_res_bits(&mut ret, Resolution::ThirtySecond, &mut x);
        Some(Time::from_bits(ret, Resolution::Second))
    }

    pub fn try_with_millis(self, x: u16) -> Option<Self> {
        if x > 999 {
            return None;
        }
        if self.resolution() != Resolution::Second {
            return None;
        }
        let mut x = x as u32;
        let mut ret = self.0.get();
        set_res_bits(&mut ret, Resolution::Millisecond, &mut x);
        set_res_bits(&mut ret, Resolution::FiveMilli, &mut x);
        set_res_bits(&mut ret, Resolution::TenMilli, &mut x);
        set_res_bits(&mut ret, Resolution::FiftyMilli, &mut x);
        set_res_bits(&mut ret, Resolution::HundredMilli, &mut x);
        set_res_bits(&mut ret, Resolution::FiveHundredMilli, &mut x);
        Some(Time::from_bits(ret, Resolution::Millisecond))
    }
}

impl Time {
    pub fn with_am_pm(self, x: AmPm) -> Self {
        self.try_with_am_pm(x).unwrap_or(self)
    }
    pub fn with_time_of_day(self, x: SixHour) -> Self {
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
    pub fn set_time_of_day(&mut self, x: SixHour) {
        *self = self.with_time_of_day(x);
    }
    pub fn set_am_pm(&mut self, x: AmPm) {
        *self = self.with_am_pm(x);
    }
}

impl Time {
    fn add_res(self, res: Resolution, x: &mut u32) {
        let subdivision = res.subdivision() as u32;
        *x *= subdivision;
        if res <= self.resolution() {
            let mut bits = self.0.get();
            bits >>= res.trailing_zeros() + 1;
            let n_bits = res.n_bits();
            bits &= !(u32::MAX << n_bits);
            *x += bits;
        }
    }

    /// 0-999
    pub fn millis(self) -> u16 {
        let mut ret = 0;
        self.add_res(Resolution::FiveHundredMilli, &mut ret);
        self.add_res(Resolution::HundredMilli, &mut ret);
        self.add_res(Resolution::FiftyMilli, &mut ret);
        self.add_res(Resolution::TenMilli, &mut ret);
        self.add_res(Resolution::FiveMilli, &mut ret);
        self.add_res(Resolution::Millisecond, &mut ret);
        ret as u16
    }
    /// 0-59
    pub fn second(self) -> u8 {
        let mut ret = 0;
        self.add_res(Resolution::ThirtySecond, &mut ret);
        self.add_res(Resolution::FifteenSecond, &mut ret);
        self.add_res(Resolution::FiveSecond, &mut ret);
        self.add_res(Resolution::Second, &mut ret);
        ret as u8
    }
    /// 0-59
    pub fn minute(self) -> u8 {
        let mut ret = 0;
        self.add_res(Resolution::ThirtyMinute, &mut ret);
        self.add_res(Resolution::FifteenMinute, &mut ret);
        self.add_res(Resolution::FiveMinute, &mut ret);
        self.add_res(Resolution::Minute, &mut ret);
        ret as u8
    }
    /// 0-23
    pub fn hour(self) -> u8 {
        let mut ret = 0;
        self.add_res(Resolution::AmPm, &mut ret);
        self.add_res(Resolution::SixHour, &mut ret);
        self.add_res(Resolution::ThreeHour, &mut ret);
        self.add_res(Resolution::Hour, &mut ret);
        ret as u8
    }
    pub fn time_of_day(self) -> Option<SixHour> {
        if self.resolution() < Resolution::AmPm {
            return None;
        }
        let mut ret = 0;
        self.add_res(Resolution::AmPm, &mut ret);
        self.add_res(Resolution::SixHour, &mut ret);
        Some((ret as u8).try_into().unwrap())
    }
    pub fn am_pm(self) -> Option<AmPm> {
        if self.resolution() < Resolution::AmPm {
            return None;
        }
        let mut ret = 0;
        self.add_res(Resolution::AmPm, &mut ret);
        Some((ret as u8).try_into().unwrap())
    }
}

impl fmt::Debug for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Time(")?;
        let mut map = f.debug_map();
        for res in Resolution::range(self.resolution(), Resolution::Day).rev() {
            let mut bits = 0;
            self.add_res(res, &mut bits);
            map.entry(&res, &bits);
        }
        map.finish()?;
        f.write_str(")")?;
        Ok(())
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.resolution() {
            Resolution::Day => f.write_str("whole day"),
            Resolution::AmPm => write!(f, "{}", self.am_pm().unwrap()),
            Resolution::SixHour => write!(f, "{}", self.time_of_day().unwrap()),
            _ => {
                write!(f, "{:02}:{:02}", self.hour(), self.minute())?;
                if self.resolution() > Resolution::Minute {
                    write!(f, ":{:02}", self.second())?;
                }
                match self.resolution() {
                    Resolution::HundredMilli | Resolution::FiveHundredMilli => {
                        write!(f, ".{:01}", self.millis() / 100)?
                    }
                    Resolution::TenMilli | Resolution::FiftyMilli => {
                        write!(f, ".{:02}", self.millis() / 10)?
                    }
                    Resolution::Millisecond | Resolution::FiveMilli => {
                        write!(f, ".{:03}", self.millis())?
                    }
                    _ => (),
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linearize::LinearizeExt;

    #[test]
    fn test_set_get() {
        let mut x = Time::default();
        for hour in 0..24 {
            let x = x.with_hour(hour);
            assert_eq!(x.hour(), hour, "{:#b}", x.0);
        }
        x.set_hour(11);
        for minute in 0..60 {
            let x = x.with_minute(minute);
            assert_eq!(x.minute(), minute, "{:#b}", x.0);
        }
        x.set_minute(43);
        for second in 0..60 {
            let x = x.with_second(second);
            assert_eq!(x.second(), second, "{:#b}", x.0);
        }
        x.set_second(59);
        for millis in 0..1000 {
            let x = x.with_millis(millis);
            assert_eq!(x.millis(), millis, "{:#b}", x.0);
        }
    }

    #[test]
    fn test_fmt() {
        let mut x = Time::default();
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
    fn test_am_pm() {
        let t = Time::new()
            .with_hour(0)
            .with_minute(0)
            .with_second(0)
            .with_millis(0);
        assert_eq!(t.am_pm(), Some(AmPm::AM));
        let t = Time::new()
            .with_hour(11)
            .with_minute(59)
            .with_second(59)
            .with_millis(999);
        assert_eq!(t.am_pm(), Some(AmPm::AM));
        let t = Time::new()
            .with_hour(12)
            .with_minute(0)
            .with_second(0)
            .with_millis(0);
        assert_eq!(t.am_pm(), Some(AmPm::PM));
        let t = Time::new()
            .with_hour(23)
            .with_minute(59)
            .with_second(59)
            .with_millis(999);
        assert_eq!(t.am_pm(), Some(AmPm::PM));
    }

    #[test]
    fn test_time_of_day() {
        let t = Time::new().with_hour(0).with_minute(0);
        assert_eq!(t.time_of_day(), Some(SixHour::Night));
        let t = Time::new().with_hour(5).with_minute(59);
        assert_eq!(t.time_of_day(), Some(SixHour::Night));
        let t = Time::new().with_hour(6).with_minute(0);
        assert_eq!(t.time_of_day(), Some(SixHour::Morning));
        let t = Time::new().with_hour(11).with_minute(59);
        assert_eq!(t.time_of_day(), Some(SixHour::Morning));
        let t = Time::new().with_hour(12).with_minute(0);
        assert_eq!(t.time_of_day(), Some(SixHour::Afternoon));
        let t = Time::new().with_hour(17).with_minute(59);
        assert_eq!(t.time_of_day(), Some(SixHour::Afternoon));
        let t = Time::new().with_hour(18).with_minute(0);
        assert_eq!(t.time_of_day(), Some(SixHour::Evening));
        let t = Time::new().with_hour(23).with_minute(59);
        assert_eq!(t.time_of_day(), Some(SixHour::Evening));
    }

    #[test]
    fn test_res_fmt() {
        let t = Time::new()
            .with_hour(15)
            .with_minute(7)
            .with_second(24)
            .with_millis(76);
        eprintln!("{t}, res={:?}, {:#b}", t.resolution(), t.0);
        assert_eq!(t.to_string(), "15:07:24.076");
        for res in Resolution::variants() {
            let actual = t.with_res(res).unwrap().to_string();
            eprintln!("{res:?} => {:?} => {}", t.with_res(res), actual);
            let expected = match res {
                Resolution::Millisecond => "15:07:24.076",
                Resolution::FiveMilli => "15:07:24.075",
                Resolution::TenMilli => "15:07:24.07",
                Resolution::FiftyMilli => "15:07:24.05",
                Resolution::HundredMilli => "15:07:24.0",
                Resolution::FiveHundredMilli => "15:07:24.0",
                Resolution::Second => "15:07:24",
                Resolution::FiveSecond => "15:07:20",
                Resolution::FifteenSecond => "15:07:15",
                Resolution::ThirtySecond => "15:07:00",
                Resolution::Minute => "15:07",
                Resolution::FiveMinute => "15:05",
                Resolution::FifteenMinute => "15:00",
                Resolution::ThirtyMinute => "15:00",
                Resolution::Hour => "15:00",
                Resolution::ThreeHour => "15:00",
                Resolution::SixHour => "afternoon",
                Resolution::AmPm => "PM",
                Resolution::Day => "whole day",
            };
            assert_eq!(actual, expected);
        }
    }
}

use core::fmt;

/// Just a date
///
/// Nothing interesting about this.  It's just a date.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
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

#[cfg(feature = "jiff")]
impl From<jiff::civil::Date> for Date {
    fn from(date: jiff::civil::Date) -> Self {
        Date {
            year: date.year(),
            month: date.month(),
            day: date.day(),
        }
    }
}

#[cfg(feature = "jiff")]
impl From<Date> for jiff::civil::Date {
    fn from(date: Date) -> Self {
        jiff::civil::date(date.year, date.month, date.day)
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

#[cfg(feature = "chrono")]
impl From<Date> for chrono::NaiveDate {
    fn from(date: Date) -> Self {
        chrono::NaiveDate::from_ymd_opt(date.year as i32, date.month as u32, date.day as u32)
            .unwrap()
    }
}

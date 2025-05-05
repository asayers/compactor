use core::fmt;
use linearize::Linearize;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum TimeOfDay {
    /// 24:00--06:00
    Night,
    /// 06:00--12:00
    Morning,
    /// 12:00--18:00
    Afternoon,
    /// 18:00--24:00
    Evening,
}

impl fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeOfDay::Night => f.write_str("night"),
            TimeOfDay::Morning => f.write_str("morning"),
            TimeOfDay::Afternoon => f.write_str("afternoon"),
            TimeOfDay::Evening => f.write_str("evening"),
        }
    }
}
impl TryFrom<u8> for TimeOfDay {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TimeOfDay::Night),
            1 => Ok(TimeOfDay::Morning),
            2 => Ok(TimeOfDay::Afternoon),
            3 => Ok(TimeOfDay::Evening),
            _ => Err(()),
        }
    }
}
impl From<TimeOfDay> for u8 {
    fn from(value: TimeOfDay) -> Self {
        match value {
            TimeOfDay::Night => 0,
            TimeOfDay::Morning => 1,
            TimeOfDay::Afternoon => 2,
            TimeOfDay::Evening => 3,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum Meridian {
    AM,
    PM,
}

impl fmt::Display for Meridian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Meridian::AM => f.write_str("AM"),
            Meridian::PM => f.write_str("PM"),
        }
    }
}
impl TryFrom<u8> for Meridian {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Meridian::AM),
            1 => Ok(Meridian::PM),
            _ => Err(()),
        }
    }
}
impl From<Meridian> for u8 {
    fn from(value: Meridian) -> Self {
        match value {
            Meridian::AM => 0,
            Meridian::PM => 1,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Weekday::Monday => f.write_str("Mon"),
            Weekday::Tuesday => f.write_str("Tue"),
            Weekday::Wednesday => f.write_str("Wed"),
            Weekday::Thursday => f.write_str("Thu"),
            Weekday::Friday => f.write_str("Fri"),
            Weekday::Saturday => f.write_str("Sat"),
            Weekday::Sunday => f.write_str("Sun"),
        }
    }
}
impl TryFrom<u8> for Weekday {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Weekday::Monday),
            1 => Ok(Weekday::Tuesday),
            2 => Ok(Weekday::Wednesday),
            3 => Ok(Weekday::Thursday),
            4 => Ok(Weekday::Friday),
            5 => Ok(Weekday::Saturday),
            6 => Ok(Weekday::Sunday),
            _ => Err(()),
        }
    }
}
impl From<Weekday> for u8 {
    fn from(value: Weekday) -> Self {
        match value {
            Weekday::Monday => 0,
            Weekday::Tuesday => 1,
            Weekday::Wednesday => 2,
            Weekday::Thursday => 3,
            Weekday::Friday => 4,
            Weekday::Saturday => 5,
            Weekday::Sunday => 6,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum Week {
    W1,
    W2,
    W3,
    W4,
    W5,
}

impl fmt::Display for Week {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Week::W1 => f.write_str("w1"),
            Week::W2 => f.write_str("w2"),
            Week::W3 => f.write_str("w3"),
            Week::W4 => f.write_str("w4"),
            Week::W5 => f.write_str("w5"),
        }
    }
}
impl TryFrom<u8> for Week {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Week::W1),
            1 => Ok(Week::W2),
            2 => Ok(Week::W3),
            3 => Ok(Week::W4),
            4 => Ok(Week::W5),
            _ => Err(()),
        }
    }
}
impl From<Week> for u8 {
    fn from(value: Week) -> Self {
        match value {
            Week::W1 => 0,
            Week::W2 => 1,
            Week::W3 => 2,
            Week::W4 => 3,
            Week::W5 => 4,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum Month {
    January,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Month::January => f.write_str("Jan"),
            Month::February => f.write_str("Feb"),
            Month::March => f.write_str("Mar"),
            Month::April => f.write_str("Apr"),
            Month::May => f.write_str("May"),
            Month::June => f.write_str("Jun"),
            Month::July => f.write_str("Jul"),
            Month::August => f.write_str("Aug"),
            Month::September => f.write_str("Sep"),
            Month::October => f.write_str("Oct"),
            Month::November => f.write_str("Nov"),
            Month::December => f.write_str("Dec"),
        }
    }
}
impl TryFrom<u8> for Month {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Month::January),
            1 => Ok(Month::February),
            2 => Ok(Month::March),
            3 => Ok(Month::April),
            4 => Ok(Month::May),
            5 => Ok(Month::June),
            6 => Ok(Month::July),
            7 => Ok(Month::August),
            8 => Ok(Month::September),
            9 => Ok(Month::October),
            10 => Ok(Month::November),
            11 => Ok(Month::December),
            _ => Err(()),
        }
    }
}
impl From<Month> for u8 {
    fn from(value: Month) -> Self {
        match value {
            Month::January => 0,
            Month::February => 1,
            Month::March => 2,
            Month::April => 3,
            Month::May => 4,
            Month::June => 5,
            Month::July => 6,
            Month::August => 7,
            Month::September => 8,
            Month::October => 9,
            Month::November => 10,
            Month::December => 11,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum Quarter {
    Q1,
    Q2,
    Q3,
    Q4,
}

impl fmt::Display for Quarter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Quarter::Q1 => f.write_str("Q1"),
            Quarter::Q2 => f.write_str("Q2"),
            Quarter::Q3 => f.write_str("Q3"),
            Quarter::Q4 => f.write_str("Q4"),
        }
    }
}
impl TryFrom<u8> for Quarter {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Quarter::Q1),
            1 => Ok(Quarter::Q2),
            2 => Ok(Quarter::Q3),
            3 => Ok(Quarter::Q4),
            _ => Err(()),
        }
    }
}
impl From<Quarter> for u8 {
    fn from(value: Quarter) -> Self {
        match value {
            Quarter::Q1 => 0,
            Quarter::Q2 => 1,
            Quarter::Q3 => 2,
            Quarter::Q4 => 3,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum Half {
    H1,
    H2,
}

impl fmt::Display for Half {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Half::H1 => f.write_str("H1"),
            Half::H2 => f.write_str("H2"),
        }
    }
}
impl TryFrom<u8> for Half {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Half::H1),
            1 => Ok(Half::H2),
            _ => Err(()),
        }
    }
}
impl From<Half> for u8 {
    fn from(value: Half) -> Self {
        match value {
            Half::H1 => 0,
            Half::H2 => 1,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Year(i8);

impl fmt::Display for Year {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            ..-1 => write!(f, "({} years ago) ", self.0),
            -1 => f.write_str("(last year) "),
            0 => f.write_str(""),
            1 => f.write_str("(next year) "),
            2.. => write!(f, "({} years from now) ", self.0),
        }
    }
}
impl TryFrom<u8> for Year {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Year(-1)),
            1 => Ok(Year(0)),
            2 => Ok(Year(1)),
            _ => Err(()),
        }
    }
}
impl From<Year> for u8 {
    fn from(value: Year) -> Self {
        match value {
            Year(-1) => 0,
            Year(0) => 1,
            Year(1) => 2,
            _ => panic!(),
        }
    }
}

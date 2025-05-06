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

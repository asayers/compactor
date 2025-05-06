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
pub enum AmPm {
    AM,
    PM,
}

impl fmt::Display for AmPm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AmPm::AM => f.write_str("AM"),
            AmPm::PM => f.write_str("PM"),
        }
    }
}
impl TryFrom<u8> for AmPm {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AmPm::AM),
            1 => Ok(AmPm::PM),
            _ => Err(()),
        }
    }
}
impl From<AmPm> for u8 {
    fn from(value: AmPm) -> Self {
        match value {
            AmPm::AM => 0,
            AmPm::PM => 1,
        }
    }
}

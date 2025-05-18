use core::fmt;
use linearize::Linearize;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Linearize)]
pub enum SixHour {
    /// 24:00--06:00
    Night,
    /// 06:00--12:00
    Morning,
    /// 12:00--18:00
    Afternoon,
    /// 18:00--24:00
    Evening,
}

impl fmt::Display for SixHour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SixHour::Night => f.write_str("night"),
            SixHour::Morning => f.write_str("morning"),
            SixHour::Afternoon => f.write_str("afternoon"),
            SixHour::Evening => f.write_str("evening"),
        }
    }
}
impl TryFrom<u8> for SixHour {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SixHour::Night),
            1 => Ok(SixHour::Morning),
            2 => Ok(SixHour::Afternoon),
            3 => Ok(SixHour::Evening),
            _ => Err(()),
        }
    }
}
impl From<SixHour> for u8 {
    fn from(value: SixHour) -> Self {
        match value {
            SixHour::Night => 0,
            SixHour::Morning => 1,
            SixHour::Afternoon => 2,
            SixHour::Evening => 3,
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

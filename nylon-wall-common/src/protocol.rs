#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Protocol {
    Any = 0,
    TCP = 6,
    UDP = 17,
    ICMP = 1,
    ICMPv6 = 58,
}

impl Protocol {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Any),
            6 => Some(Self::TCP),
            17 => Some(Self::UDP),
            1 => Some(Self::ICMP),
            58 => Some(Self::ICMPv6),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[repr(C)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

impl PortRange {
    pub fn single(port: u16) -> Self {
        Self { start: port, end: port }
    }

    pub fn range(start: u16, end: u16) -> Self {
        Self { start, end }
    }

    pub fn contains(&self, port: u16) -> bool {
        port >= self.start && port <= self.end
    }
}

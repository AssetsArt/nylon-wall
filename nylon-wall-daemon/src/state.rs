//! Connection tracking state reader

use nylon_wall_common::conntrack::ConntrackInfo;

pub struct ConntrackReader;

impl ConntrackReader {
    pub fn new() -> Self {
        Self
    }

    /// Read current connections from eBPF conntrack map
    pub fn get_connections(&self) -> Vec<ConntrackInfo> {
        // TODO: Read from eBPF LRU HashMap
        Vec::new()
    }
}

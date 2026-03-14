use serde::{Deserialize, Serialize};

/// WebSocket event types broadcast to connected clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsEvent {
    #[serde(rename = "rule_created")]
    RuleCreated(serde_json::Value),
    #[serde(rename = "rule_updated")]
    RuleUpdated(serde_json::Value),
    #[serde(rename = "rule_deleted")]
    RuleDeleted { id: u32 },
    #[serde(rename = "rule_toggled")]
    RuleToggled(serde_json::Value),
    #[serde(rename = "nat_created")]
    NatCreated(serde_json::Value),
    #[serde(rename = "nat_updated")]
    NatUpdated(serde_json::Value),
    #[serde(rename = "nat_deleted")]
    NatDeleted { id: u32 },
    #[serde(rename = "zone_created")]
    ZoneCreated(serde_json::Value),
    #[serde(rename = "zone_updated")]
    ZoneUpdated(serde_json::Value),
    #[serde(rename = "zone_deleted")]
    ZoneDeleted { id: u32 },
    #[serde(rename = "policy_created")]
    PolicyCreated(serde_json::Value),
    #[serde(rename = "policy_updated")]
    PolicyUpdated(serde_json::Value),
    #[serde(rename = "policy_deleted")]
    PolicyDeleted { id: u32 },
    #[serde(rename = "route_created")]
    RouteCreated(serde_json::Value),
    #[serde(rename = "route_updated")]
    RouteUpdated(serde_json::Value),
    #[serde(rename = "route_deleted")]
    RouteDeleted { id: u32 },
    #[serde(rename = "log_event")]
    LogEvent(serde_json::Value),
    #[serde(rename = "config_restored")]
    ConfigRestored,

    // DHCP events
    #[serde(rename = "dhcp_pool_created")]
    DhcpPoolCreated(serde_json::Value),
    #[serde(rename = "dhcp_pool_updated")]
    DhcpPoolUpdated(serde_json::Value),
    #[serde(rename = "dhcp_pool_deleted")]
    DhcpPoolDeleted { id: u32 },
    #[serde(rename = "dhcp_lease_changed")]
    DhcpLeaseChanged(serde_json::Value),
    #[serde(rename = "dhcp_reservation_created")]
    DhcpReservationCreated(serde_json::Value),
    #[serde(rename = "dhcp_reservation_deleted")]
    DhcpReservationDeleted { id: u32 },
    #[serde(rename = "dhcp_client_status_changed")]
    DhcpClientStatusChanged(serde_json::Value),

    // Change management
    #[serde(rename = "changes_reverted")]
    ChangesReverted { count: usize },
}

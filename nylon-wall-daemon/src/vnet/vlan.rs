use nylon_wall_common::vnet::VlanConfig;

/// Apply a VLAN sub-interface to the system.
#[cfg(target_os = "linux")]
pub fn apply_vlan(config: &VlanConfig) -> Result<(), String> {
    let iface = config.iface_name();

    // Create VLAN sub-interface (ignore if already exists)
    let _ = std::process::Command::new("ip")
        .args([
            "link", "add", "link",
            &config.parent_interface,
            "name", &iface,
            "type", "vlan", "id",
            &config.vlan_id.to_string(),
        ])
        .output();

    // Set IP address if provided
    if let Some(ref ip) = config.ip_address {
        if !ip.is_empty() {
            let _ = std::process::Command::new("ip")
                .args(["addr", "flush", "dev", &iface])
                .output();
            let output = std::process::Command::new("ip")
                .args(["addr", "add", ip, "dev", &iface])
                .output()
                .map_err(|e| format!("ip addr add failed: {}", e))?;
            if !output.status.success() {
                tracing::warn!(
                    "ip addr add {}: {}",
                    iface,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }

    // Bring up
    let _ = std::process::Command::new("ip")
        .args(["link", "set", &iface, "up"])
        .output();

    tracing::info!("VLAN {} applied on {}", config.vlan_id, config.parent_interface);
    Ok(())
}

/// Remove a VLAN sub-interface.
#[cfg(target_os = "linux")]
pub fn remove_vlan(config: &VlanConfig) {
    let iface = config.iface_name();
    let _ = std::process::Command::new("ip")
        .args(["link", "delete", &iface])
        .output();
    tracing::info!("VLAN {} removed", iface);
}

#[cfg(not(target_os = "linux"))]
pub fn apply_vlan(_config: &VlanConfig) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn remove_vlan(_config: &VlanConfig) {}

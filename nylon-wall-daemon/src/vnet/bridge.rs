use nylon_wall_common::vnet::BridgeConfig;

/// Apply a bridge configuration to the system.
#[cfg(target_os = "linux")]
pub fn apply_bridge(config: &BridgeConfig) -> Result<(), String> {
    let name = &config.name;

    // Create bridge (ignore if already exists)
    let _ = std::process::Command::new("ip")
        .args(["link", "add", "name", name, "type", "bridge"])
        .output();

    // Set STP state
    let stp = if config.stp_enabled { "1" } else { "0" };
    let _ = std::process::Command::new("ip")
        .args(["link", "set", name, "type", "bridge", "stp_state", stp])
        .output();

    // Set IP address if provided
    if let Some(ref ip) = config.ip_address {
        if !ip.is_empty() {
            let _ = std::process::Command::new("ip")
                .args(["addr", "flush", "dev", name])
                .output();
            let output = std::process::Command::new("ip")
                .args(["addr", "add", ip, "dev", name])
                .output()
                .map_err(|e| format!("ip addr add failed: {}", e))?;
            if !output.status.success() {
                tracing::warn!(
                    "ip addr add {}: {}",
                    name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }

    // Add ports
    for port in &config.ports {
        let output = std::process::Command::new("ip")
            .args(["link", "set", port, "master", name])
            .output()
            .map_err(|e| format!("ip link set master failed: {}", e))?;
        if !output.status.success() {
            tracing::warn!(
                "Adding port {} to {}: {}",
                port,
                name,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    // Bring up
    let _ = std::process::Command::new("ip")
        .args(["link", "set", name, "up"])
        .output();

    tracing::info!("Bridge {} applied with {} ports", name, config.ports.len());
    Ok(())
}

/// Update bridge ports by diffing old vs new port lists.
#[cfg(target_os = "linux")]
pub fn update_bridge_ports(name: &str, old_ports: &[String], new_ports: &[String]) {
    // Remove ports no longer in the list
    for port in old_ports {
        if !new_ports.contains(port) {
            let _ = std::process::Command::new("ip")
                .args(["link", "set", port, "nomaster"])
                .output();
        }
    }
    // Add newly added ports
    for port in new_ports {
        if !old_ports.contains(port) {
            let _ = std::process::Command::new("ip")
                .args(["link", "set", port, "master", name])
                .output();
        }
    }
}

/// Remove a bridge.
#[cfg(target_os = "linux")]
pub fn remove_bridge(config: &BridgeConfig) {
    // Detach all ports first
    for port in &config.ports {
        let _ = std::process::Command::new("ip")
            .args(["link", "set", port, "nomaster"])
            .output();
    }
    let _ = std::process::Command::new("ip")
        .args(["link", "delete", &config.name])
        .output();
    tracing::info!("Bridge {} removed", config.name);
}

#[cfg(not(target_os = "linux"))]
pub fn apply_bridge(_config: &BridgeConfig) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn update_bridge_ports(_name: &str, _old_ports: &[String], _new_ports: &[String]) {}

#[cfg(not(target_os = "linux"))]
pub fn remove_bridge(_config: &BridgeConfig) {}

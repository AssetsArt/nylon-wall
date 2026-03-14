//! Route management — static routes and policy-based routing.
//! On Linux, manages kernel routing table entries via ip command.

/// Apply a static route to the kernel routing table.
#[cfg(target_os = "linux")]
pub async fn apply_route(route: &nylon_wall_common::route::Route) -> anyhow::Result<()> {
    if !route.enabled {
        return remove_route(route).await;
    }

    let mut args = vec![
        "route".to_string(),
        "replace".to_string(),
        route.destination.clone(),
    ];

    if let Some(ref gw) = route.gateway {
        args.push("via".to_string());
        args.push(gw.clone());
    }

    args.push("dev".to_string());
    args.push(route.interface.clone());

    if route.metric > 0 {
        args.push("metric".to_string());
        args.push(route.metric.to_string());
    }

    if route.table > 0 && route.table != 254 {
        args.push("table".to_string());
        args.push(route.table.to_string());
    }

    let output = tokio::process::Command::new("ip")
        .args(&args)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Failed to apply route {}: {}", route.destination, stderr);
    } else {
        tracing::info!("Applied route: {} via {} dev {}", route.destination,
            route.gateway.as_deref().unwrap_or("direct"), route.interface);
    }

    Ok(())
}

/// Remove a static route from the kernel routing table.
#[cfg(target_os = "linux")]
pub async fn remove_route(route: &nylon_wall_common::route::Route) -> anyhow::Result<()> {
    let mut args = vec![
        "route".to_string(),
        "del".to_string(),
        route.destination.clone(),
    ];

    if route.table > 0 && route.table != 254 {
        args.push("table".to_string());
        args.push(route.table.to_string());
    }

    let output = tokio::process::Command::new("ip")
        .args(&args)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!("Route delete (may not exist): {}", stderr);
    }

    Ok(())
}

/// Apply a policy route using ip rule + ip route.
#[cfg(target_os = "linux")]
pub async fn apply_policy_route(policy: &nylon_wall_common::route::PolicyRoute) -> anyhow::Result<()> {
    // Add ip rule for source-based routing
    let fwmark = policy.route_table;
    let mut rule_args = vec![
        "rule".to_string(),
        "add".to_string(),
    ];

    if let Some(ref src) = policy.src_ip {
        rule_args.push("from".to_string());
        rule_args.push(src.clone());
    }

    if let Some(ref dst) = policy.dst_ip {
        rule_args.push("to".to_string());
        rule_args.push(dst.clone());
    }

    rule_args.push("priority".to_string());
    rule_args.push(policy.priority.to_string());
    rule_args.push("table".to_string());
    rule_args.push(fwmark.to_string());

    let output = tokio::process::Command::new("ip")
        .args(&rule_args)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("File exists") {
            tracing::warn!("Failed to add policy rule: {}", stderr);
        }
    } else {
        tracing::info!("Applied policy route: table {} priority {}", fwmark, policy.priority);
    }

    Ok(())
}

/// Remove a policy route rule.
#[cfg(target_os = "linux")]
pub async fn remove_policy_route(policy: &nylon_wall_common::route::PolicyRoute) -> anyhow::Result<()> {
    let mut rule_args = vec![
        "rule".to_string(),
        "del".to_string(),
    ];

    if let Some(ref src) = policy.src_ip {
        rule_args.push("from".to_string());
        rule_args.push(src.clone());
    }

    rule_args.push("priority".to_string());
    rule_args.push(policy.priority.to_string());

    let output = tokio::process::Command::new("ip")
        .args(&rule_args)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!("Policy rule delete (may not exist): {}", stderr);
    }

    Ok(())
}

/// Sync all routes from database to kernel on startup.
#[cfg(target_os = "linux")]
pub async fn sync_routes(state: &crate::AppState) {
    use nylon_wall_common::route::{PolicyRoute, Route};

    // Apply static routes
    let routes = state
        .db
        .scan_prefix::<Route>("route:")
        .await
        .unwrap_or_default();
    for (_, route) in &routes {
        if let Err(e) = apply_route(route).await {
            tracing::warn!("Failed to sync route {}: {}", route.destination, e);
        }
    }

    // Apply policy routes
    let policies = state
        .db
        .scan_prefix::<PolicyRoute>("policy_route:")
        .await
        .unwrap_or_default();
    for (_, policy) in &policies {
        if let Err(e) = apply_policy_route(policy).await {
            tracing::warn!("Failed to sync policy route: {}", e);
        }
    }

    tracing::info!(
        "Synced {} static routes + {} policy routes to kernel",
        routes.len(),
        policies.len()
    );
}

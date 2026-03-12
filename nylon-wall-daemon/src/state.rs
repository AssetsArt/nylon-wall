//! Connection tracking and perf event processing

use nylon_wall_common::conntrack::ConntrackInfo;

/// Read conntrack entries — delegates to eBPF on Linux, returns empty on other platforms.
pub async fn get_connections(state: &crate::AppState) -> Vec<ConntrackInfo> {
    #[cfg(target_os = "linux")]
    {
        let mut ebpf_guard = state.ebpf.lock().await;
        if let Some(ref mut bpf) = *ebpf_guard {
            crate::ebpf_loader::read_conntrack(bpf)
        } else {
            Vec::new()
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = state;
        Vec::new()
    }
}

/// Background task: read perf events from eBPF EVENTS map and persist to SlateDB.
#[cfg(target_os = "linux")]
pub async fn perf_event_loop(state: std::sync::Arc<crate::AppState>) {
    use aya::maps::AsyncPerfEventArray;
    use aya::util::online_cpus;
    use bytes::BytesMut;
    use nylon_wall_common::log::{EbpfPacketEvent, PacketLog};

    let cpus = match online_cpus() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to get online CPUs: {}", e);
            return;
        }
    };

    // Take the EVENTS map out of the Ebpf handle
    let perf_array = {
        let mut ebpf_guard = state.ebpf.lock().await;
        let bpf = match ebpf_guard.as_mut() {
            Some(b) => b,
            None => return,
        };
        let map = match bpf.take_map("EVENTS") {
            Some(m) => m,
            None => {
                tracing::warn!("EVENTS perf map not found");
                return;
            }
        };
        match AsyncPerfEventArray::try_from(map) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Failed to create AsyncPerfEventArray: {}", e);
                return;
            }
        }
    };

    tracing::info!("Started perf event reader on {} CPUs", cpus.len());

    let mut tasks = Vec::new();

    for cpu_id in cpus {
        let mut buf = perf_array
            .open(cpu_id, None)
            .expect("Failed to open perf buffer");
        let state = std::sync::Arc::clone(&state);

        let task = tokio::spawn(async move {
            let mut buffers = (0..10)
                .map(|_| BytesMut::with_capacity(std::mem::size_of::<EbpfPacketEvent>() + 64))
                .collect::<Vec<_>>();

            loop {
                let events = match buf.read_events(&mut buffers).await {
                    Ok(events) => events,
                    Err(e) => {
                        tracing::warn!("Error reading perf events on CPU {}: {}", cpu_id, e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                for i in 0..events.read {
                    let buf = &buffers[i];
                    if buf.len() < std::mem::size_of::<EbpfPacketEvent>() {
                        continue;
                    }

                    let event: EbpfPacketEvent = unsafe {
                        std::ptr::read_unaligned(buf.as_ptr() as *const EbpfPacketEvent)
                    };

                    let src_ip = std::net::Ipv4Addr::from(u32::from_be(event.src_ip));
                    let dst_ip = std::net::Ipv4Addr::from(u32::from_be(event.dst_ip));

                    let protocol = match event.protocol {
                        6 => "TCP",
                        17 => "UDP",
                        1 => "ICMP",
                        _ => "Other",
                    };

                    let action = match event.action {
                        0 => "allow",
                        1 => "drop",
                        2 => "log",
                        3 => "rate_limit",
                        _ => "unknown",
                    };

                    let now = chrono::Utc::now().timestamp();
                    let log = PacketLog {
                        timestamp: now,
                        src_ip: src_ip.to_string(),
                        dst_ip: dst_ip.to_string(),
                        src_port: event.src_port,
                        dst_port: event.dst_port,
                        protocol: protocol.to_string(),
                        action: action.to_string(),
                        rule_id: event.rule_id,
                        interface: format!("if{}", event.ifindex),
                        bytes: event.bytes,
                    };

                    // Persist to SlateDB
                    let seq = event.timestamp % 1_000_000;
                    let key = format!("log:{}:{:06}", now, seq);
                    if let Err(e) = state.db.put(&key, &log).await {
                        tracing::warn!("Failed to store log: {}", e);
                    }
                    if let Err(e) = state.db.add_to_index("log:", &key).await {
                        tracing::warn!("Failed to update log index: {}", e);
                    }

                    // Broadcast to WebSocket subscribers
                    let _ = state.event_tx.send(
                        crate::events::WsEvent::LogEvent(
                            serde_json::to_value(&log).unwrap_or_default(),
                        ),
                    );
                }
            }
        });

        tasks.push(task);
    }

    // Wait for all CPU reader tasks
    futures_util::future::join_all(tasks).await;
}

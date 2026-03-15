use dioxus::prelude::*;
use futures_util::StreamExt;
use gloo_net::websocket::{futures::WebSocket, Message};
use serde::Deserialize;

use crate::api_client;

/// Per-category event generation counters.
/// Components watch their relevant counter(s) to trigger refetches on real-time events.
#[derive(Clone, Copy)]
pub struct WsEventBus {
    rules: Signal<u64>,
    nat: Signal<u64>,
    routes: Signal<u64>,
    zones: Signal<u64>,
    policies: Signal<u64>,
    dhcp: Signal<u64>,
    sni: Signal<u64>,
    ddns: Signal<u64>,
    logs: Signal<u64>,
    system: Signal<u64>,
    connected: Signal<bool>,
}

impl WsEventBus {
    pub fn rules(&self) -> u64 { (self.rules)() }
    pub fn nat(&self) -> u64 { (self.nat)() }
    pub fn routes(&self) -> u64 { (self.routes)() }
    pub fn zones(&self) -> u64 { (self.zones)() }
    pub fn policies(&self) -> u64 { (self.policies)() }
    pub fn dhcp(&self) -> u64 { (self.dhcp)() }
    pub fn sni(&self) -> u64 { (self.sni)() }
    pub fn ddns(&self) -> u64 { (self.ddns)() }
    pub fn logs(&self) -> u64 { (self.logs)() }
    pub fn system(&self) -> u64 { (self.system)() }
    pub fn connected(&self) -> bool { (self.connected)() }

    fn inc(sig: &mut Signal<u64>) {
        let val = *sig.peek();
        sig.set(val + 1);
    }
}

/// We only need the "type" field to determine which category changed.
#[derive(Deserialize)]
struct WsEventType {
    r#type: String,
}

fn increment_for_event(bus: &mut WsEventBus, event_type: &str) {
    match event_type {
        t if t.starts_with("rule_") => WsEventBus::inc(&mut bus.rules),
        t if t.starts_with("nat_") => WsEventBus::inc(&mut bus.nat),
        t if t.starts_with("route_") => WsEventBus::inc(&mut bus.routes),
        t if t.starts_with("zone_") => WsEventBus::inc(&mut bus.zones),
        t if t.starts_with("policy_") => WsEventBus::inc(&mut bus.policies),
        t if t.starts_with("dhcp_") => WsEventBus::inc(&mut bus.dhcp),
        t if t.starts_with("sni_") => WsEventBus::inc(&mut bus.sni),
        t if t.starts_with("ddns_") => WsEventBus::inc(&mut bus.ddns),
        "log_event" => WsEventBus::inc(&mut bus.logs),
        "config_restored" => {
            WsEventBus::inc(&mut bus.rules);
            WsEventBus::inc(&mut bus.nat);
            WsEventBus::inc(&mut bus.routes);
            WsEventBus::inc(&mut bus.zones);
            WsEventBus::inc(&mut bus.policies);
            WsEventBus::inc(&mut bus.dhcp);
            WsEventBus::inc(&mut bus.sni);
            WsEventBus::inc(&mut bus.ddns);
            WsEventBus::inc(&mut bus.system);
        }
        "changes_reverted" => WsEventBus::inc(&mut bus.system),
        _ => {}
    }
}

/// Initialize WebSocket connection and provide event bus as context.
/// Call once in Layout.
pub fn use_ws_provider() {
    let mut bus = WsEventBus {
        rules: use_signal(|| 0u64),
        nat: use_signal(|| 0u64),
        routes: use_signal(|| 0u64),
        zones: use_signal(|| 0u64),
        policies: use_signal(|| 0u64),
        dhcp: use_signal(|| 0u64),
        sni: use_signal(|| 0u64),
        ddns: use_signal(|| 0u64),
        logs: use_signal(|| 0u64),
        system: use_signal(|| 0u64),
        connected: use_signal(|| false),
    };
    use_context_provider(|| bus);

    use_future(move || async move {
        loop {
            let url = api_client::ws_url();

            match WebSocket::open(&url) {
                Ok(mut ws) => {
                    bus.connected.set(true);

                    while let Some(msg) = ws.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(evt) = serde_json::from_str::<WsEventType>(&text) {
                                    increment_for_event(&mut bus, &evt.r#type);
                                }
                            }
                            Err(_) => break,
                            _ => {}
                        }
                    }

                    bus.connected.set(false);
                }
                Err(_) => {}
            }

            // Reconnect after delay
            gloo_timers::future::TimeoutFuture::new(3_000).await;
        }
    });
}

/// Get the WebSocket event bus from context.
pub fn use_ws_events() -> WsEventBus {
    use_context::<WsEventBus>()
}

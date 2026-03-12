# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Nylon Wall is a Linux network firewall built in Rust using eBPF (via aya) for kernel-space packet processing and Dioxus for a web management UI. It is in early development — only the workspace root and a hello-world main.rs exist so far.

## Architecture

The system has four crates in a Cargo workspace:

- **nylon-wall-common** — Shared types between kernel and userspace (FirewallRule, NatEntry, Route, Zone, ConntrackEntry, etc.)
- **nylon-wall-ebpf** — eBPF programs (XDP ingress, TC egress, NAT) compiled to `bpfel-unknown-none` target
- **nylon-wall-daemon** — Userspace daemon: loads eBPF programs via aya, manages rules/state in SlateDB, exposes REST API via axum on port 9450
- **nylon-wall-ui** — Dioxus fullstack web UI on port 8080, communicates with daemon via HTTP

Data flows: UI → REST API (axum) → daemon → eBPF maps → kernel. Logs flow back via eBPF perf events → daemon → SlateDB → WebSocket → UI.

## Build Commands

```bash
# Install required tools
cargo install bpf-linker
cargo install dioxus-cli

# Build eBPF programs (requires nightly)
cargo build -p nylon-wall-ebpf --target bpfel-unknown-none -Z build-std=core

# Build daemon
cargo build -p nylon-wall-daemon --release

# Build UI
cd nylon-wall-ui && dx build --release

# Run daemon (requires root for eBPF)
sudo ./target/release/nylon-wall-daemon
```

## Build Requirements

- Linux kernel >= 5.15
- Rust nightly (for eBPF compilation)
- Root or CAP_BPF + CAP_NET_ADMIN privileges to run

## Key Tech Choices

- **eBPF framework**: aya (pure Rust, no C/libbpf dependency)
- **Storage**: SlateDB (embedded KV store on object storage) with prefix-based key namespacing (e.g., `rule:{id}`, `nat:{id}`, `log:{timestamp}:{seq}`)
- **Serialization**: serde_json for all stored values and API payloads
- **IPC**: Unix domain socket / gRPC (tonic) between daemon and UI
- **Logging**: tracing crate for structured logging
- **Rust edition**: 2024

## API

REST API base: `http://localhost:9450/api/v1` — resources include `/rules`, `/nat`, `/routes`, `/zones`, `/policies`, `/conntrack`, `/logs`, `/metrics`, `/system/*`. Real-time events via WebSocket at `/ws/events`.

## Configuration

Default config path: `/etc/nylon-wall/config.toml` — configures daemon listen address, SlateDB path, eBPF mode (driver/skb/hw), interfaces, and UI bind address.

## Implementation Status

The project is at the very beginning. See `checklist.md` for the phased implementation plan (Phase 1: Foundation → Phase 6: System & Hardening). See `spec.md` for the full specification including data structures, eBPF map design, API endpoints, and UI pages.

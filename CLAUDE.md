# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Nylon Wall is a Linux network firewall built in Rust using eBPF (via aya) for kernel-space packet processing and Dioxus for a web management UI.

## Architecture

The system has four crates in a Cargo workspace:

- **nylon-wall-common** — Shared types between kernel and userspace. Supports both `no_std` (eBPF) and `std` via feature flags. eBPF-facing structs use `#[repr(C)]`, userspace types gated behind `#[cfg(feature = "std")]`.
- **nylon-wall-ebpf** — eBPF programs (XDP ingress, TC egress, NAT) compiled to `bpfel-unknown-none` target. Excluded from workspace (separate build).
- **nylon-wall-daemon** — Userspace daemon: loads eBPF programs via aya, manages rules/state in SlateDB, exposes REST API via axum on port 9450.
- **nylon-wall-ui** — Dioxus 0.7 web UI, communicates with daemon via gloo-net HTTP client.

Data flows: UI → REST API (axum) → daemon → eBPF maps → kernel. Logs flow back via eBPF perf events → daemon → SlateDB → WebSocket → UI.

## Build Commands

```bash
# Check workspace (common + daemon)
cargo check

# Check UI (requires wasm target)
cargo check -p nylon-wall-ui --target wasm32-unknown-unknown

# Build daemon
cargo build -p nylon-wall-daemon --release

# Build/serve UI (requires dioxus-cli)
cd nylon-wall-ui && dx serve        # dev mode
cd nylon-wall-ui && dx build --release  # production

# Build eBPF (Linux only, requires nightly + bpf-linker)
cargo build -p nylon-wall-ebpf --target bpfel-unknown-none -Z build-std=core

# Run daemon (requires root for eBPF on Linux)
sudo ./target/release/nylon-wall-daemon
```

## Key Conventions

- **Naming**: `RuleAction` enum (not `Action`) to avoid collision with `dioxus::prelude::Action`
- **Storage keys**: SlateDB prefix-based namespacing (`rule:{id}`, `nat:{id}`, `zone:{id}`, `policy:{id}`, `route:{id}`, `log:{timestamp}:{seq}`)
- **eBPF structs**: Always `#[repr(C)]` with explicit padding, available in `no_std`
- **Userspace-only types**: Gated behind `#[cfg(feature = "std")]` with serde derives
- **Rust edition**: 2024

## Key Tech Choices

- **eBPF framework**: aya (pure Rust, no C/libbpf dependency)
- **Storage**: SlateDB 0.3 with index-key pattern for scan_prefix (stores key lists at `{prefix}__index`)
- **Web UI**: Dioxus 0.7 with router, gloo-net for HTTP, dark-theme CSS
- **Serialization**: serde_json for all stored values and API payloads
- **Logging**: tracing crate

## API

REST API base: `http://localhost:9450/api/v1` — CRUD for `/rules`, `/nat`, `/routes`, `/zones`, `/policies`, plus `/system/status`. Real-time events planned via WebSocket at `/ws/events`.

## Configuration

Default config: `/etc/nylon-wall/config.toml` — daemon listen address, SlateDB path, eBPF mode, interfaces, UI bind address.

## Implementation Status

Phase 1 (Foundation) complete. See `checklist.md` for remaining phases and `spec.md` for full specification.

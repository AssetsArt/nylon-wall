//! eBPF program loader - only functional on Linux

#[cfg(target_os = "linux")]
pub async fn load_and_attach() -> anyhow::Result<()> {
    use tracing::info;

    // TODO: Load eBPF bytecode
    // let mut bpf = aya::Ebpf::load(include_bytes_aligned!(
    //     "../../target/bpfel-unknown-none/release/nylon-wall-ebpf"
    // ))?;

    info!("eBPF loader initialized (placeholder)");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub async fn load_and_attach() -> anyhow::Result<()> {
    tracing::warn!("eBPF not available on this platform");
    Ok(())
}

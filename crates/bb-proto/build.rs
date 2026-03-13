use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(
        &[
            "proto/device.proto",
            "proto/heartbeat.proto",
            "proto/blocklist.proto",
            "proto/events.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}

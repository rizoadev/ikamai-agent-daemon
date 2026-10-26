//! Fuzz test: IPC frame parsing — ensure no panic on arbitrary input.
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 { return; }
    let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if len > 4 * 1024 * 1024 { return; }  // Skip huge frames
    if data.len() < 4 + len { return; }
    let payload = &data[4..4+len];
    // Should not panic
    let _ = serde_json::from_slice::<serde_json::Value>(payload);
});

//! Fuzz test: Agent manifest YAML parsing.
#![no_main]
use libfuzzer_sys::fuzz_target;
use iad_core::AgentManifest;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Should not panic
        let _ = serde_yaml::from_str::<AgentManifest>(s);
    }
});

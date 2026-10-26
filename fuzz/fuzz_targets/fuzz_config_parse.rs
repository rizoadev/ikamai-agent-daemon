//! Fuzz test: TOML config parsing.
#![no_main]
use libfuzzer_sys::fuzz_target;
use iad_core::config::DaemonConfig;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Should not panic
        let _ = toml::from_str::<DaemonConfig>(s);
    }
});

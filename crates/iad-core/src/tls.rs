//! mTLS configuration for gRPC (Phase 4.10).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// TLS configuration for gRPC server/client.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    /// Enable mTLS (require client certificate).
    pub enabled:      bool,
    /// Server certificate file (PEM).
    pub cert_file:    Option<PathBuf>,
    /// Server private key file (PEM).
    pub key_file:     Option<PathBuf>,
    /// CA certificate for verifying client certs (mTLS).
    pub ca_file:      Option<PathBuf>,
}

impl TlsConfig {
    pub fn is_valid(&self) -> bool {
        if !self.enabled { return true; }
        self.cert_file.as_ref().map(|p| p.exists()).unwrap_or(false)
            && self.key_file.as_ref().map(|p| p.exists()).unwrap_or(false)
    }

    /// Load cert + key as PEM bytes for tonic Identity.
    pub fn load_identity(&self) -> crate::Result<Option<(Vec<u8>, Vec<u8>)>> {
        if !self.enabled { return Ok(None); }
        let cert = self.cert_file.as_ref()
            .map(std::fs::read)
            .transpose()?
            .ok_or_else(|| crate::Error::Config("cert_file not set".into()))?;
        let key = self.key_file.as_ref()
            .map(std::fs::read)
            .transpose()?
            .ok_or_else(|| crate::Error::Config("key_file not set".into()))?;
        Ok(Some((cert, key)))
    }

    /// Load CA cert PEM bytes for client verification.
    pub fn load_ca(&self) -> crate::Result<Option<Vec<u8>>> {
        if !self.enabled { return Ok(None); }
        Ok(self.ca_file.as_ref().map(std::fs::read).transpose()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_is_valid() {
        let cfg = TlsConfig { enabled: false, ..Default::default() };
        assert!(cfg.is_valid());
    }

    #[test]
    fn enabled_without_certs_invalid() {
        let cfg = TlsConfig { enabled: true, ..Default::default() };
        assert!(!cfg.is_valid());
    }
}

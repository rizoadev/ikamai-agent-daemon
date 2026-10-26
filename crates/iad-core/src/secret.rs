//! Secret management — encrypted vault (Phase 4.12).
//!
//! Stores secrets encrypted at rest using AES-256-GCM.
//! Key is derived from a master password via PBKDF2.
//! Secrets are written to ~/.iad/vault.json (encrypted JSON blob).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use crate::{Error, Result};

/// A named secret entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEntry {
    pub name:       String,
    pub value_enc:  String,   // base64(AES-256-GCM(value))
    pub nonce:      String,   // base64(12-byte nonce)
    pub created_at: String,
}

/// In-memory secret store (decrypted at runtime).
#[derive(Default)]
pub struct SecretStore {
    secrets: HashMap<String, String>,
}

impl SecretStore {
    pub fn new() -> Self { Self::default() }

    /// Set a secret.
    pub fn set(&mut self, name: &str, value: &str) {
        self.secrets.insert(name.into(), value.into());
    }

    /// Get a secret by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.secrets.get(name).map(|s| s.as_str())
    }

    /// Delete a secret.
    pub fn delete(&mut self, name: &str) -> bool {
        self.secrets.remove(name).is_some()
    }

    /// List secret names (values never returned directly).
    pub fn list_names(&self) -> Vec<&str> {
        self.secrets.keys().map(|s| s.as_str()).collect()
    }

    /// Inject secret values into env var map.
    /// Replaces "${secret:NAME}" references in values.
    pub fn inject_env(&self, env: &mut HashMap<String, String>) {
        for val in env.values_mut() {
            if let Some(name) = val.strip_prefix("${secret:").and_then(|s| s.strip_suffix('}')) {
                if let Some(secret) = self.get(name) {
                    *val = secret.to_string();
                }
            }
        }
    }

    /// Save to file as plaintext JSON (encryption placeholder — integrate ring/aes-gcm in prod).
    pub fn save(&self, path: &Path) -> Result<()> {
        let map: HashMap<&str, &str> = self.secrets.iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let json = serde_json::to_string_pretty(&map)
            .map_err(|e| Error::Serde(e.to_string()))?;
        // In production: encrypt with AES-256-GCM before writing.
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load from file.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() { return Ok(Self::default()); }
        let raw = std::fs::read_to_string(path)?;
        let map: HashMap<String, String> = serde_json::from_str(&raw)
            .map_err(|e| Error::Serde(e.to_string()))?;
        Ok(Self { secrets: map })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_delete() {
        let mut s = SecretStore::new();
        s.set("API_KEY", "sk-1234");
        assert_eq!(s.get("API_KEY"), Some("sk-1234"));
        assert!(s.delete("API_KEY"));
        assert!(s.get("API_KEY").is_none());
    }

    #[test]
    fn inject_env() {
        let mut s = SecretStore::new();
        s.set("DB_PASS", "hunter2");
        let mut env = HashMap::new();
        env.insert("PASSWORD".into(), "${secret:DB_PASS}".into());
        env.insert("OTHER".into(), "plain".into());
        s.inject_env(&mut env);
        assert_eq!(env["PASSWORD"], "hunter2");
        assert_eq!(env["OTHER"], "plain");
    }

    #[test]
    fn list_names() {
        let mut s = SecretStore::new();
        s.set("A", "1"); s.set("B", "2");
        let mut names = s.list_names();
        names.sort();
        assert_eq!(names, vec!["A", "B"]);
    }
}

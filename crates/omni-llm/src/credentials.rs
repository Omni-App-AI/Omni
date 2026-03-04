use chrono::{DateTime, Utc};
use keyring::Entry;
use serde::{Deserialize, Serialize};

use crate::error::{LlmError, Result};

const SERVICE_NAME: &str = "omni";
const PROVIDER_PREFIX: &str = "provider.";

/// Credential types supported by the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Credential {
    ApiKey {
        key: String,
    },
    OAuth {
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    },
    AwsCredentials {
        access_key_id: String,
        secret_access_key: String,
        session_token: Option<String>,
        region: String,
    },
}

impl Credential {
    /// Check if an OAuth credential has expired.
    pub fn is_expired(&self) -> bool {
        match self {
            Credential::OAuth {
                expires_at: Some(exp),
                ..
            } => Utc::now() >= *exp,
            _ => false,
        }
    }

    /// Get the API key if this is an ApiKey credential.
    pub fn api_key(&self) -> Option<&str> {
        match self {
            Credential::ApiKey { key } => Some(key),
            _ => None,
        }
    }

    /// Get the bearer token (API key or OAuth access token).
    pub fn bearer_token(&self) -> Option<&str> {
        match self {
            Credential::ApiKey { key } => Some(key),
            Credential::OAuth { access_token, .. } => Some(access_token),
            _ => None,
        }
    }
}

/// Secure credential storage using the OS keychain.
pub struct CredentialVault {
    service_name: String,
}

impl CredentialVault {
    pub fn new() -> Self {
        Self {
            service_name: SERVICE_NAME.to_string(),
        }
    }

    /// Store a credential in the OS keychain.
    pub fn store(&self, provider_id: &str, credential: &Credential) -> Result<()> {
        let key = format!("{}{}", PROVIDER_PREFIX, provider_id);
        let entry = Entry::new(&self.service_name, &key)
            .map_err(|e| LlmError::Keyring(e.to_string()))?;
        let serialized =
            serde_json::to_string(credential).map_err(|e| LlmError::Auth(e.to_string()))?;
        entry
            .set_password(&serialized)
            .map_err(|e| LlmError::Keyring(e.to_string()))?;
        Ok(())
    }

    /// Retrieve a credential from the OS keychain.
    pub fn retrieve(&self, provider_id: &str) -> Result<Option<Credential>> {
        let key = format!("{}{}", PROVIDER_PREFIX, provider_id);
        let entry = Entry::new(&self.service_name, &key)
            .map_err(|e| LlmError::Keyring(e.to_string()))?;
        match entry.get_password() {
            Ok(serialized) => {
                let credential: Credential = serde_json::from_str(&serialized)
                    .map_err(|e| LlmError::Auth(e.to_string()))?;
                Ok(Some(credential))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(LlmError::Keyring(e.to_string())),
        }
    }

    /// Delete a credential from the OS keychain.
    pub fn delete(&self, provider_id: &str) -> Result<()> {
        let key = format!("{}{}", PROVIDER_PREFIX, provider_id);
        let entry = Entry::new(&self.service_name, &key)
            .map_err(|e| LlmError::Keyring(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already deleted
            Err(e) => Err(LlmError::Keyring(e.to_string())),
        }
    }

    /// Try to retrieve a credential, returning an error if not found.
    pub fn require(&self, provider_id: &str) -> Result<Credential> {
        self.retrieve(provider_id)?
            .ok_or_else(|| LlmError::CredentialNotFound(provider_id.to_string()))
    }
}

impl Default for CredentialVault {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_api_key() {
        let cred = Credential::ApiKey {
            key: "sk-test123".to_string(),
        };
        assert_eq!(cred.api_key(), Some("sk-test123"));
        assert_eq!(cred.bearer_token(), Some("sk-test123"));
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_credential_oauth_not_expired() {
        let cred = Credential::OAuth {
            access_token: "at-test".to_string(),
            refresh_token: Some("rt-test".to_string()),
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        };
        assert!(!cred.is_expired());
        assert_eq!(cred.bearer_token(), Some("at-test"));
        assert_eq!(cred.api_key(), None);
    }

    #[test]
    fn test_credential_oauth_expired() {
        let cred = Credential::OAuth {
            access_token: "at-expired".to_string(),
            refresh_token: None,
            expires_at: Some(Utc::now() - chrono::Duration::hours(1)),
        };
        assert!(cred.is_expired());
    }

    #[test]
    fn test_credential_oauth_no_expiry() {
        let cred = Credential::OAuth {
            access_token: "at-noexp".to_string(),
            refresh_token: None,
            expires_at: None,
        };
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_credential_aws() {
        let cred = Credential::AwsCredentials {
            access_key_id: "AKID".to_string(),
            secret_access_key: "SECRET".to_string(),
            session_token: None,
            region: "us-east-1".to_string(),
        };
        assert_eq!(cred.api_key(), None);
        assert_eq!(cred.bearer_token(), None);
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_credential_serialization_roundtrip() {
        let cred = Credential::ApiKey {
            key: "sk-abc".to_string(),
        };
        let json = serde_json::to_string(&cred).unwrap();
        let deserialized: Credential = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.api_key(), Some("sk-abc"));
    }

    #[test]
    fn test_credential_vault_new() {
        let vault = CredentialVault::new();
        assert_eq!(vault.service_name, "omni");
    }
}

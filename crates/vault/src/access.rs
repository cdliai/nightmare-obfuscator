//! Access control levels for vaults

/// Different access levels for vault content
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AccessLevel {
    /// View only - can decrypt but not modify
    View,
    /// Standard - full access to vault content
    Standard,
    /// Admin - can modify vault settings
    Admin,
    /// Owner - full control including destruction
    Owner,
}

/// Access token granted after successful authentication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccessToken {
    pub level: AccessLevel,
    pub granted_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub vault_name: String,
}

impl AccessToken {
    pub fn new(level: AccessLevel, vault_name: &str) -> Self {
        Self {
            level,
            granted_at: chrono::Utc::now(),
            expires_at: None,
            vault_name: vault_name.to_string(),
        }
    }

    pub fn with_expiry(mut self, seconds: i64) -> Self {
        self.expires_at = Some(chrono::Utc::now() + chrono::Duration::seconds(seconds));
        self
    }

    pub fn is_valid(&self) -> bool {
        if let Some(expiry) = self.expires_at {
            chrono::Utc::now() < expiry
        } else {
            true
        }
    }

    pub fn can_read(&self) -> bool {
        self.is_valid() && self.level as u8 >= AccessLevel::View as u8
    }

    pub fn can_write(&self) -> bool {
        self.is_valid() && self.level as u8 >= AccessLevel::Standard as u8
    }

    pub fn can_admin(&self) -> bool {
        self.is_valid() && self.level as u8 >= AccessLevel::Admin as u8
    }
}

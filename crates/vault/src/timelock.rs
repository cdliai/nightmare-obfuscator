//! Time-locked encryption using blockchain time oracles

use chrono::{DateTime, Duration, Utc};
use nightmare_core::{NightmareError, Result};

/// Timelock that prevents decryption before a specific time
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Timelock {
    unlock_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    duration_seconds: u64,
}

impl Timelock {
    pub fn new(duration_seconds: u64) -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            unlock_at: now + Duration::seconds(duration_seconds as i64),
            duration_seconds,
        }
    }

    pub fn from_timestamp(unlock_at: DateTime<Utc>) -> Self {
        let now = Utc::now();
        let duration = (unlock_at - now).num_seconds().max(0) as u64;

        Self {
            created_at: now,
            unlock_at,
            duration_seconds: duration,
        }
    }

    pub fn from_string(data: &str) -> Result<Self> {
        serde_json::from_str(data).map_err(|e| NightmareError::Crypto(e.to_string()))
    }

    pub fn is_unlocked(&self) -> bool {
        Utc::now() >= self.unlock_at
    }

    pub fn unlock_time(&self) -> DateTime<Utc> {
        self.unlock_at
    }

    pub fn seconds_remaining(&self) -> i64 {
        let remaining = self.unlock_at - Utc::now();
        remaining.num_seconds().max(0)
    }

    pub fn to_string(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| NightmareError::Crypto(e.to_string()))
    }
}

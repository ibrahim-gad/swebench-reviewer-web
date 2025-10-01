#[cfg(feature = "ssr")]
use std::collections::HashMap;
#[cfg(feature = "ssr")]
use std::sync::{Arc, RwLock};
#[cfg(feature = "ssr")]
use std::path::PathBuf;
#[cfg(feature = "ssr")]
use uuid::Uuid;
#[cfg(feature = "ssr")]
use crate::auth::GoogleTokens;
#[cfg(feature = "ssr")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use tokio::{fs, io::AsyncWriteExt};
#[cfg(feature = "ssr")]
use anyhow::{Result, anyhow};

#[cfg(feature = "ssr")]
pub type SessionId = String;

#[cfg(feature = "ssr")]
#[derive(Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub user_tokens: GoogleTokens,
    pub created_at: std::time::SystemTime,
}

#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, SessionData>>>,
    storage_path: PathBuf,
}

#[cfg(feature = "ssr")]
impl SessionManager {
    pub fn new() -> Self {
        let storage_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("sessions.json");
        
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage_path,
        }
    }

    pub async fn new_with_persistence() -> Result<Self> {
        let mut manager = Self::new();
        manager.load_sessions().await?;
        Ok(manager)
    }

    async fn load_sessions(&mut self) -> Result<()> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        match fs::read_to_string(&self.storage_path).await {
            Ok(content) => {
                let sessions: HashMap<SessionId, SessionData> = serde_json::from_str(&content)
                    .map_err(|e| anyhow!("Failed to parse sessions file: {}", e))?;
                
                if let Ok(mut current_sessions) = self.sessions.write() {
                    *current_sessions = sessions;
                }
                Ok(())
            }
            Err(_) => Ok(()) // File doesn't exist or can't be read, start fresh
        }
    }

    async fn save_sessions(&self) -> Result<()> {
        let sessions = if let Ok(sessions) = self.sessions.read() {
            sessions.clone()
        } else {
            return Err(anyhow!("Failed to read sessions for saving"));
        };

        let content = serde_json::to_string_pretty(&sessions)
            .map_err(|e| anyhow!("Failed to serialize sessions: {}", e))?;

        let temp_path = self.storage_path.with_extension("tmp");
        
        // Write to temporary file first, then rename for atomic operation
        let mut file = fs::File::create(&temp_path).await
            .map_err(|e| anyhow!("Failed to create temp file: {}", e))?;
        
        file.write_all(content.as_bytes()).await
            .map_err(|e| anyhow!("Failed to write sessions: {}", e))?;
        
        file.sync_all().await
            .map_err(|e| anyhow!("Failed to sync file: {}", e))?;
        
        fs::rename(&temp_path, &self.storage_path).await
            .map_err(|e| anyhow!("Failed to rename temp file: {}", e))?;

        Ok(())
    }

    pub fn create_session(&self, tokens: GoogleTokens) -> SessionId {
        let session_id = Uuid::new_v4().to_string();
        let session_data = SessionData {
            user_tokens: tokens,
            created_at: std::time::SystemTime::now(),
        };

        if let Ok(mut sessions) = self.sessions.write() {
            sessions.insert(session_id.clone(), session_data);
        }

        // Save to disk asynchronously (fire and forget)
        let manager_clone = self.clone();
        tokio::spawn(async move {
            let _ = manager_clone.save_sessions().await;
        });

        session_id
    }

    pub fn get_session(&self, session_id: &str) -> Option<SessionData> {
        if let Ok(sessions) = self.sessions.read() {
            sessions.get(session_id).cloned()
        } else {
            None
        }
    }

    pub fn update_session(&self, session_id: &str, tokens: GoogleTokens) -> bool {
        let result = if let Ok(mut sessions) = self.sessions.write() {
            if let Some(session_data) = sessions.get_mut(session_id) {
                session_data.user_tokens = tokens;
                true
            } else {
                false
            }
        } else {
            false
        };

        if result {
            // Save to disk asynchronously (fire and forget)
            let manager_clone = self.clone();
            tokio::spawn(async move {
                let _ = manager_clone.save_sessions().await;
            });
        }

        result
    }

    pub fn remove_session(&self, session_id: &str) -> bool {
        let result = if let Ok(mut sessions) = self.sessions.write() {
            sessions.remove(session_id).is_some()
        } else {
            false
        };

        if result {
            // Save to disk asynchronously (fire and forget)
            let manager_clone = self.clone();
            tokio::spawn(async move {
                let _ = manager_clone.save_sessions().await;
            });
        }

        result
    }

    pub fn cleanup_expired_sessions(&self) {
        let mut removed_any = false;
        if let Ok(mut sessions) = self.sessions.write() {
            let now = std::time::SystemTime::now();
            let max_age = std::time::Duration::from_secs(24 * 60 * 60); // 24 hours

            let original_len = sessions.len();
            sessions.retain(|_, session_data| {
                now.duration_since(session_data.created_at)
                    .map(|age| age < max_age)
                    .unwrap_or(false)
            });
            removed_any = sessions.len() != original_len;
        }

        if removed_any {
            // Save to disk asynchronously (fire and forget)
            let manager_clone = self.clone();
            tokio::spawn(async move {
                let _ = manager_clone.save_sessions().await;
            });
        }
    }
}

#[cfg(feature = "ssr")]
impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

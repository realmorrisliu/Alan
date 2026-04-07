use crate::token_data::{ChatgptTokenData, parse_jwt_expiration};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredChatgptAuth {
    pub tokens: ChatgptTokenData,
    pub account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token_expires_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_refresh_at: Option<DateTime<Utc>>,
}

impl StoredChatgptAuth {
    pub fn from_tokens(tokens: ChatgptTokenData) -> io::Result<Self> {
        let account_id = tokens.id_token.account_id.clone().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "ChatGPT id token did not include chatgpt_account_id",
            )
        })?;

        Ok(Self {
            email: tokens.id_token.email.clone(),
            plan_type: tokens.id_token.plan_type.clone(),
            user_id: tokens.id_token.user_id.clone(),
            access_token_expires_at: parse_jwt_expiration(&tokens.access_token).ok().flatten(),
            tokens,
            account_id,
            last_refresh_at: Some(Utc::now()),
        })
    }

    pub fn is_access_token_expired(&self, now: DateTime<Utc>) -> bool {
        self.access_token_expires_at.is_some_and(|exp| exp <= now)
    }

    pub fn should_refresh(&self, now: DateTime<Utc>) -> bool {
        self.access_token_expires_at
            .is_some_and(|exp| exp <= now + chrono::Duration::minutes(2))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AuthStore {
    #[serde(default)]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chatgpt: Option<StoredChatgptAuth>,
}

#[derive(Debug, Clone)]
pub struct AuthStorage {
    path: PathBuf,
}

impl AuthStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> io::Result<AuthStore> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => serde_json::from_str(&raw).map_err(io::Error::other),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(AuthStore {
                version: 1,
                chatgpt: None,
            }),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self, store: &AuthStore) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let raw = serde_json::to_vec_pretty(store).map_err(io::Error::other)?;
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).write(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let mut file = options.open(&self.path)?;
        file.write_all(&raw)?;
        file.write_all(b"\n")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&self.path, permissions)?;
        }

        Ok(())
    }

    pub fn clear_chatgpt(&self) -> io::Result<()> {
        let mut store = self.load()?;
        store.chatgpt = None;
        self.save(&store)
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthStorage, AuthStore, StoredChatgptAuth};
    use crate::token_data::{ChatgptIdTokenInfo, ChatgptTokenData};
    use base64::Engine;
    use serde_json::json;
    use tempfile::TempDir;

    fn build_jwt(payload: serde_json::Value) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"alg":"none","typ":"JWT"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        format!("{header}.{payload}.sig")
    }

    #[test]
    fn storage_round_trip() {
        let temp_dir = TempDir::new().expect("temp dir");
        let storage = AuthStorage::new(temp_dir.path().join("auth.json"));
        let id_token = build_jwt(json!({
            "email": "user@example.com",
            "https://api.openai.com/auth": {
                "chatgpt_plan_type": "pro",
                "chatgpt_user_id": "user_123",
                "chatgpt_account_id": "acct_123"
            }
        }));
        let access_token = build_jwt(json!({"exp": 4_102_444_800_i64}));
        let auth = StoredChatgptAuth::from_tokens(ChatgptTokenData {
            id_token: ChatgptIdTokenInfo {
                email: Some("user@example.com".to_string()),
                plan_type: Some("pro".to_string()),
                user_id: Some("user_123".to_string()),
                account_id: Some("acct_123".to_string()),
                raw_jwt: id_token,
            },
            access_token,
            refresh_token: "refresh".to_string(),
        })
        .expect("stored auth");
        storage
            .save(&AuthStore {
                version: 1,
                chatgpt: Some(auth.clone()),
            })
            .expect("save");

        let loaded = storage.load().expect("load");
        assert_eq!(loaded.chatgpt, Some(auth));
    }

    #[test]
    fn stored_auth_accepts_opaque_access_tokens() {
        let id_token = build_jwt(serde_json::json!({
            "email": "user@example.com",
            "https://api.openai.com/auth": {
                "chatgpt_account_id": "acct_123"
            }
        }));

        let auth = StoredChatgptAuth::from_tokens(ChatgptTokenData {
            id_token: ChatgptIdTokenInfo {
                email: Some("user@example.com".to_string()),
                plan_type: None,
                user_id: None,
                account_id: Some("acct_123".to_string()),
                raw_jwt: id_token,
            },
            access_token: "opaque-access-token".to_string(),
            refresh_token: "refresh".to_string(),
        })
        .expect("stored auth");

        assert_eq!(auth.access_token_expires_at, None);
    }
}

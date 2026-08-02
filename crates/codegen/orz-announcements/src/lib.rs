use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteAnnouncement {
    pub id: String,
    pub message: String,
    pub url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl RemoteAnnouncement {
    pub fn should_show(&self, _last_emitted: &[Self], _stored: Option<&[Self]>) -> bool {
        false
    }
}

//! Announcements stub — no real announcements.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteAnnouncement {
    pub id: String,
    pub title: String,
    pub body: String,
    pub url: Option<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub dismissible: bool,
}

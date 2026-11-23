use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatorProfile {
    pub id: String,
    pub name: String,
    pub service: String,
    pub indexed: String,
    pub updated: String,
    pub public_id: Option<String>,
    pub relation_id: Option<u64>,
    pub has_chats: Option<bool>,
    pub post_count: u64,
    pub dm_count: u64,
    pub share_count: u64,
    pub chat_count: u64,
}

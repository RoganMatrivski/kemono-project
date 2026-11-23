use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatorProfile {
    pub id: String,
    pub name: String,
    pub service: String,
    pub indexed: String,
    pub updated: String,
    #[serde(default)]
    pub public_id: Option<String>,
    #[serde(default)]
    pub relation_id: Option<u64>,
    #[serde(default)]
    pub has_chats: Option<bool>,
    #[serde(default)]
    pub post_count: u64,
    #[serde(default)]
    pub dm_count: u64,
    #[serde(default)]
    pub share_count: u64,
    #[serde(default)]
    pub chat_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_without_counts() {
        let j = r#"{"id":"1","name":"n","service":"patreon","indexed":"i","updated":"u"}"#;
        let p: CreatorProfile = serde_json::from_str(j).unwrap();
        assert_eq!(p.post_count, 0);
        assert!(p.public_id.is_none());
    }
}

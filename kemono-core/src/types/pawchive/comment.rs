use jiff::civil::DateTime;
use serde::{Deserialize, Serialize};

/// `GET /v1/{service}/user/{id}/post/{post_id}/comments` item. `published` has microseconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PawchiveComment {
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub commenter: String,
    #[serde(default)]
    pub commenter_name: String,
    #[serde(default)]
    pub content: String,
    pub published: DateTime,
    #[serde(default)]
    pub revisions: Vec<PawchiveCommentRevision>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PawchiveCommentRevision {
    pub id: u64,
    pub content: String,
    pub added: DateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comment() {
        let j = r#"{"id":"1","parent_id":null,"commenter":"2","commenter_name":"n","content":"c",
          "published":"2026-11-23T15:00:27.650000","revisions":[{"id":5,"content":"x","added":"2026-11-08T10:00:00"}]}"#;
        let c: PawchiveComment = serde_json::from_str(j).unwrap();
        assert_eq!(c.revisions.len(), 1);
        assert!(c.parent_id.is_none());
    }
}

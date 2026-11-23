use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostRoot {
    pub post: Post,
    #[serde(default)]
    pub attachments: Vec<Value>,
    #[serde(default)]
    pub previews: Vec<Preview>,
    #[serde(default)]
    pub videos: Vec<Value>,
    #[serde(default)]
    pub props: Props,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: String,
    pub user: String,
    pub service: String,
    pub title: String,
    pub content: String,
    pub embed: Embed,
    #[serde(rename = "shared_file")]
    pub shared_file: bool,
    pub added: String,
    pub published: String,
    #[serde(default)]
    pub edited: Option<String>,
    pub file: FileObject,
    pub attachments: Vec<FileObject>,
    #[serde(default)]
    pub poll: Value,
    #[serde(default)]
    pub captions: Value,
    #[serde(default)]
    pub tags: Value,
    /// Neighbouring post IDs; `null` at either end of a creator's list.
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub prev: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Embed {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    #[serde(rename = "type")]
    pub type_field: String,
    pub server: String,
    pub name: String,
    pub path: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Props {
    /// Documented as an integer, seen as null/other types; kept loose.
    #[serde(default)]
    pub flagged: Value,
    #[serde(default)]
    pub revisions: Vec<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostSummary {
    pub id: String,
    pub user: String,
    pub service: String,
    pub title: String,
    pub substring: String,
    pub published: String, // ISO 8601 datetime as string
    pub file: FileObject,
    pub attachments: Vec<FileObject>,
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct FileObject {
    pub name: String,
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_post_has_null_next_and_edited() {
        let j = r#"{"post":{"id":"1","user":"2","service":"fanbox","title":"t","content":"c",
          "embed":{},"shared_file":false,"added":"2021-03-30T18:00:05.973913",
          "published":"2021-01-24T17:54:38","edited":null,
          "file":{"name":"a.jpg","path":"/5c/98/x.jpg"},"attachments":[],
          "next":null,"prev":"1836649"},
          "attachments":[],"previews":[],"videos":[],"props":{"flagged":null,"revisions":[]}}"#;
        let r: PostRoot = serde_json::from_str(j).unwrap();
        assert_eq!(r.post.next, None);
        assert_eq!(r.post.prev.as_deref(), Some("1836649"));
        assert_eq!(r.post.edited, None);
    }

    #[test]
    fn minimal_root_still_parses() {
        let j = r#"{"post":{"id":"1","user":"2","service":"s","title":"t","content":"c",
          "embed":{},"shared_file":true,"added":"a","published":"p",
          "file":{"name":"n","path":"p"},"attachments":[]}}"#;
        assert!(serde_json::from_str::<PostRoot>(j).is_ok());
    }
}

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostRoot {
    pub post: Post,
    pub attachments: Vec<Value>,
    pub previews: Vec<Preview>,
    pub videos: Vec<Value>,
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
    pub edited: Value,
    pub file: FileObject,
    pub attachments: Vec<FileObject>,
    pub poll: Value,
    pub captions: Value,
    pub tags: Value,
    pub next: String,
    pub prev: Value,
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
    pub flagged: String,
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

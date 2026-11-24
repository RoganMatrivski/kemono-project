use jiff::civil::DateTime;
use serde::{Deserialize, Serialize};
use url::Url;

/// `GET /v1/search_hash/{sha256}`. `discord_posts` is null, not `[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PawchiveHashSearch {
    pub hash: String,
    #[serde(default)]
    pub posts: Vec<PawchiveHashPost>,
    #[serde(default)]
    pub discord_posts: Option<Vec<serde_json::Value>>,
}

/// Posts here carry a truncated `content` and a [`PawchiveHashFile`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PawchiveHashPost {
    pub id: String,
    pub user: String,
    pub service: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub published: Option<DateTime>,
    #[serde(default)]
    pub file: PawchiveHashFile,
    #[serde(default)]
    pub attachments: Vec<PawchiveHashFile>,
}

/// Unlike other file objects: numeric `node` and the classic path, no `server`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PawchiveHashFile {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub node: Option<u32>,
}

impl PawchiveHashFile {
    /// `https://n{node}.pawchive.pw/data{path}?f={name}` (hotlink-protected, see `PawchiveFile::url`).
    pub fn url(&self) -> Option<Url> {
        let node = self.node?;
        if self.path.is_empty() {
            return None;
        }
        let mut url = Url::parse(&format!("https://n{node}.pawchive.pw/data{}", self.path)).ok()?;
        if !self.name.is_empty() {
            url.query_pairs_mut().append_pair("f", &self.name);
        }
        Some(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_and_url() {
        let j = r#"{"hash":"44","posts":[{"id":"1","user":"2","service":"patreon","title":"t",
          "content":"c","published":"2026-11-23T12:00:27",
          "file":{"name":"a b.png","node":2,"path":"/44/ad/44.png"},"attachments":[]}],
          "discord_posts":null}"#;
        let r: PawchiveHashSearch = serde_json::from_str(j).unwrap();
        assert!(r.discord_posts.is_none());
        assert_eq!(
            r.posts[0].file.url().unwrap().as_str(),
            "https://n2.pawchive.pw/data/44/ad/44.png?f=a+b.png"
        );
    }
}

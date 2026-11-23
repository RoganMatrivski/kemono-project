use jiff::civil::DateTime;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A file or attachment. Newer files carry `server`; older ones use the
/// classic path with no server (favorite posts use `node` instead).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PawchiveFile {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub server: Option<String>,
    /// Numeric file node (`n{node}.pawchive.pw`); used by favorite posts instead of `server`.
    #[serde(default)]
    pub node: Option<u32>,
    /// Set on listing items whose full file is not archived (no `server`, so no URL).
    #[serde(default)]
    pub preview_only: Option<bool>,
}

impl PawchiveFile {
    /// `{server}/data{path}?f={name}`, or `None` if there is no server or path.
    ///
    /// Note: downloads are hotlink-protected; a direct request without a
    /// pawchive.pw `Referer` gets 403.
    pub fn url(&self) -> Option<Url> {
        let server = match (&self.server, self.node) {
            (Some(s), _) => s.clone(),
            (None, Some(n)) => format!("https://n{n}.pawchive.pw"),
            _ => return None,
        };
        if self.path.is_empty() {
            return None;
        }
        let mut url = Url::parse(&format!("{server}/data{}", self.path)).ok()?;
        if !self.name.is_empty() {
            url.query_pairs_mut().append_pair("f", &self.name);
        }
        Some(url)
    }
}

/// Full post shape: single post and a creator's post list.
/// `next`/`prev` only appear on the single-post endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PawchivePost {
    pub id: String,
    pub user: String,
    pub service: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub embed: Value,
    #[serde(default)]
    pub shared_file: bool,
    #[serde(default)]
    pub added: Option<DateTime>,
    #[serde(default)]
    pub published: Option<DateTime>,
    #[serde(default)]
    pub edited: Option<DateTime>,
    #[serde(default)]
    pub file: PawchiveFile,
    #[serde(default)]
    pub attachments: Vec<PawchiveFile>,
    #[serde(default)]
    pub poll: Value,
    #[serde(default)]
    pub captions: Value,
    /// Raw Postgres array literal (`{"a b",c}`) or null. See [`Self::parsed_tags`].
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub origin: Option<String>,
    #[serde(default)]
    pub preview_state: Option<String>,
    #[serde(default)]
    pub has_full: Option<bool>,
    #[serde(default)]
    pub detail_fetched: Option<bool>,
    #[serde(default)]
    pub next: Option<String>,
    #[serde(default)]
    pub prev: Option<String>,
}

impl PawchivePost {
    pub fn parsed_tags(&self) -> Vec<String> {
        self.tags.as_deref().map(parse_pg_array).unwrap_or_default()
    }
}

/// A favorite post: the full post plus `faved_seq`. Its `file` uses `node`, not `server`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PawchiveFavoritePost {
    #[serde(flatten)]
    pub post: PawchivePost,
    #[serde(default)]
    pub faved_seq: Option<u64>,
}

/// A post revision: the full post shape plus `revision_id`. Newest first.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PawchiveRevision {
    pub revision_id: u64,
    #[serde(flatten)]
    pub post: PawchivePost,
}

/// Listing shape from `GET /v1/posts` (has `substring` instead of `content`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PawchivePostSummary {
    pub id: String,
    pub user: String,
    pub service: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub substring: String,
    #[serde(default)]
    pub published: Option<DateTime>,
    #[serde(default)]
    pub file: PawchiveFile,
    #[serde(default)]
    pub attachments: Vec<PawchiveFile>,
    #[serde(default)]
    pub preview_state: Option<String>,
    #[serde(default)]
    pub has_full: Option<bool>,
    #[serde(default)]
    pub origin: Option<String>,
}

/// Parses a one-dimensional Postgres text array literal such as
/// `{"Anime Reactions",Steins;Gate}`. Quoted elements may use `\` escapes.
pub fn parse_pg_array(s: &str) -> Vec<String> {
    let s = s.trim();
    let Some(inner) = s.strip_prefix('{').and_then(|r| r.strip_suffix('}')) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut in_quotes, mut was_quoted) = (false, false);
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' if in_quotes => cur.extend(chars.next()),
            '"' => {
                in_quotes = !in_quotes;
                was_quoted = true;
            }
            ',' if !in_quotes => {
                out.push(std::mem::take(&mut cur));
                was_quoted = false;
            }
            _ => cur.push(c),
        }
    }
    if was_quoted || !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_literal() {
        assert_eq!(
            parse_pg_array(r#"{"Anime Reactions",Steins;Gate}"#),
            ["Anime Reactions", "Steins;Gate"]
        );
        assert_eq!(parse_pg_array(r#"{"a \"q\", b",c}"#), ["a \"q\", b", "c"]);
        assert!(parse_pg_array("{}").is_empty());
        assert!(parse_pg_array("garbage").is_empty());
    }

    #[test]
    fn full_post() {
        let j = r#"{"id":"1","user":"2","service":"patreon","title":"t","content":"<p>x</p>",
          "embed":{},"shared_file":false,"added":"2026-11-23T13:00:00",
          "published":"2026-11-23T12:00:27","edited":"2026-11-23T12:00:27",
          "file":{"name":"x y.png","path":"/244/ad/h.png","server":"https://n2.pawchive.pw"},
          "attachments":[],"poll":null,"captions":null,"tags":"{\"A b\",c}",
          "origin":"import","preview_state":"scraped","has_full":true,"detail_fetched":true,
          "next":"3","prev":null}"#;
        let p: PawchivePost = serde_json::from_str(j).unwrap();
        assert_eq!(p.parsed_tags(), ["A b", "c"]);
        assert_eq!(p.prev, None);
        assert_eq!(p.published.unwrap().to_string(), "2026-11-23T12:00:27");
        assert_eq!(
            p.file.url().unwrap().as_str(),
            "https://n2.pawchive.pw/data/244/ad/h.png?f=x+y.png"
        );
    }

    #[test]
    fn null_tags_and_micros() {
        let j = r#"{"id":"1","user":"2","service":"fanbox","tags":null,
          "published":"2026-11-05T14:01:01.452000"}"#;
        let p: PawchivePost = serde_json::from_str(j).unwrap();
        assert!(p.parsed_tags().is_empty());
        assert!(p.published.is_some());
    }

    #[test]
    fn revision_and_preview_only() {
        let j = r#"{"revision_id":7,"id":"1","user":"2","service":"patreon","title":"t","content":"c",
          "file":{"name":"a.png","path":"/ff/2c/x.png","preview_only":true},"next":null}"#;
        let r: PawchiveRevision = serde_json::from_str(j).unwrap();
        assert_eq!(r.revision_id, 7);
        assert_eq!(r.post.file.preview_only, Some(true));
        assert!(r.post.file.url().is_none());
    }
}

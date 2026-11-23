use jiff::{Timestamp, civil::DateTime};
use serde::{Deserialize, Serialize};

/// `GET /v1/{service}/user/{creator_id}/profile`
///
/// Timestamps are ISO strings without a timezone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PawchiveCreatorProfile {
    pub id: String,
    pub name: String,
    pub service: String,
    pub indexed: DateTime,
    pub updated: DateTime,
    #[serde(default)]
    pub public_id: Option<String>,
    #[serde(default)]
    pub relation_id: Option<u64>,
    #[serde(default)]
    pub ever_imported: bool,
    /// Only on `/links` items.
    #[serde(default)]
    pub kemono_favorited: Option<u64>,
    /// Only on `/links` items.
    #[serde(default)]
    pub reimport_pending: Option<bool>,
}

/// One item of `GET /v1/creators` (the whole site, ~16 MB).
/// `indexed`/`updated` are Unix seconds here, unlike the profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PawchiveCreatorListItem {
    pub id: String,
    pub name: String,
    pub service: String,
    #[serde(default)]
    pub public_id: Option<String>,
    #[serde(with = "unix_seconds")]
    pub indexed: Timestamp,
    #[serde(with = "unix_seconds")]
    pub updated: Timestamp,
    #[serde(default)]
    pub favorited: u64,
    #[serde(default)]
    pub ever_imported: bool,
}

/// `GET /v1/{service}/user/{id}/tags` item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PawchiveTagCount {
    pub tag: String,
    pub post_count: u64,
}

/// `GET /v1/fanbox/user/{id}/fancards` item. `ext` includes the dot; `price` is a string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PawchiveFancard {
    pub id: u64,
    pub user_id: String,
    pub file_id: u64,
    pub price: String,
    pub hash: String,
    pub mtime: DateTime,
    pub ctime: DateTime,
    pub mime: String,
    pub ext: String,
    pub added: DateTime,
    pub size: u64,
    #[serde(default)]
    pub ihash: Option<String>,
}

pub(crate) mod unix_seconds {
    use jiff::Timestamp;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(t: &Timestamp, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_i64(t.as_second())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Timestamp, D::Error> {
        Timestamp::from_second(i64::deserialize(d)?).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_item_unix_seconds() {
        let j = r#"{"id":"1","name":"B","public_id":"B","service":"patreon","indexed":1785423600,
          "updated":1795222800,"favorited":3877,"ever_imported":true}"#;
        let c: PawchiveCreatorListItem = serde_json::from_str(j).unwrap();
        assert_eq!(c.indexed.as_second(), 1785423600);
        assert_eq!(serde_json::to_value(&c).unwrap()["updated"], 1795222800);
    }

    #[test]
    fn fancard_and_links() {
        let j = r#"{"id":8649,"user_id":"37","file_id":8649,"price":"1000","hash":"ab",
          "mtime":"2026-09-16T20:38:17.209157","ctime":"2026-09-16T20:38:17.209157",
          "mime":"image/webp","ext":".webp","added":"2026-09-16T21:00:00","size":33406,"ihash":null}"#;
        let f: PawchiveFancard = serde_json::from_str(j).unwrap();
        assert_eq!(f.ext, ".webp");
        let l = r#"{"id":"1","name":"A","service":"patreon","indexed":"2026-07-27T08:00:00",
          "updated":"2026-11-07T02:00:00","public_id":"x","relation_id":622,"ever_imported":true,
          "kemono_favorited":0,"reimport_pending":false}"#;
        let p: PawchiveCreatorProfile = serde_json::from_str(l).unwrap();
        assert_eq!(p.relation_id, Some(622));
    }
}

use jiff::civil::DateTime;
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
}

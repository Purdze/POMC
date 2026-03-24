use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub id: String,
    pub icon: Option<String>,
    pub name: String,
    pub version: String,
    pub last_played: Option<NonZeroU64>,
    pub created_at: u64,
    pub directory: String,
    pub width: u32,
    pub height: u32,
}

use chrono::{DateTime, Utc};
use cloud_proto::{prost_types::Timestamp, proto};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DbUser {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub email: String,
    pub username: String,
    pub passhash: String,
    pub storage_quota: Option<u64>,
    pub storage_used: u64,
}

impl DbUser {
    pub fn to_proto(&self) -> proto::User {
        proto::User {
            id: self.id.to_string(),
            username: self.username.to_string(),
            storage_quota: self.storage_quota,
            storage_used: self.storage_used,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbFile {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub owner_id: ObjectId,
    pub bucket_id: ObjectId,
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub modified_at: DateTime<Utc>,
}

impl DbFile {
    pub fn to_proto(&self) -> proto::File {
        proto::File {
            id: self.id.to_string(),
            path: self.path.to_owned(),
            hash: self.hash.to_owned(),
            size: self.size,
            created_at: Some(Timestamp::default()),
            modified_at: Some(Timestamp::default()),
        }
    }
}

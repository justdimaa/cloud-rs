use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub url: String,
    pub access_token: String,
    pub user_id: String,
}

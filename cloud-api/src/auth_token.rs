use chrono::{Duration, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub fn create_access_token(user_id: ObjectId) -> Option<String> {
    let claims = Claims {
        sub: user_id.to_string(),
        exp: (Utc::now() + Duration::weeks(1)).timestamp() as usize,
    };

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret("secret".as_ref()),
    )
    .ok()
}

pub fn validate_access_token(
    token: String,
) -> Result<jsonwebtoken::TokenData<Claims>, jsonwebtoken::errors::Error> {
    jsonwebtoken::decode::<Claims>(
        &token,
        &jsonwebtoken::DecodingKey::from_secret("secret".as_ref()),
        &jsonwebtoken::Validation::default(),
    )
}

pub fn get_user_id_from_request<T>(request: &tonic::Request<T>) -> Result<ObjectId, tonic::Status> {
    match request.metadata().get("authorization") {
        Some(token) => match token.to_str() {
            Ok(token) => {
                let token = token.trim_start_matches("Baerer ");
                let data = validate_access_token(token.to_owned())
                    .map_err(|e| tonic::Status::unauthenticated(e.to_string()))?;
                ObjectId::parse_str(data.claims.sub)
                    .map_err(|e| tonic::Status::unauthenticated(e.to_string()))
            }
            Err(_) => Err(tonic::Status::unauthenticated(
                "auth token is not a valid string",
            )),
        },
        None => Err(tonic::Status::unauthenticated("auth token is missing")),
    }
}

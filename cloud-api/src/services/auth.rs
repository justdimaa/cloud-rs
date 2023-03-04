use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use cloud_proto::proto::{
    auth_service_server::AuthService, AuthLoginRequest, AuthLoginResponse, AuthRegisterRequest,
    AuthRegisterResponse,
};
use mongodb::bson::{doc, oid::ObjectId};
use tonic::{Request, Response, Status};

use crate::{auth_token, models::DbUser};

#[derive(Debug)]
pub struct MyAuthService {
    mongo: mongodb::Client,
}

impl MyAuthService {
    pub fn new(mongo: mongodb::Client) -> Self {
        Self { mongo }
    }
}

#[tonic::async_trait]
impl AuthService for MyAuthService {
    async fn register(
        &self,
        request: Request<AuthRegisterRequest>,
    ) -> Result<Response<AuthRegisterResponse>, Status> {
        let argon = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let passhash = argon.hash_password(request.get_ref().password.as_bytes(), &salt);

        let passhash = match passhash {
            Ok(p) => p.to_string(),
            Err(e) => {
                tracing::error!("failed to hash password: {:?}", e);
                return Err(Status::internal(e.to_string()));
            }
        };

        let db_user = DbUser {
            id: ObjectId::new(),
            email: request.get_ref().email.to_lowercase(),
            username: request.get_ref().username.to_owned(),
            passhash,
            max_storage: 1073741824, // 10gb
        };

        let db = self.mongo.database("cloud");
        let db_users = db.collection::<DbUser>("users");
        db_users
            .insert_one(&db_user, None)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let token = auth_token::create_access_token(db_user.id);

        Ok(Response::new(AuthRegisterResponse {
            access_token: token.unwrap(),
            user_id: db_user.id.to_string(),
        }))
    }

    async fn login(
        &self,
        request: Request<AuthLoginRequest>,
    ) -> Result<Response<AuthLoginResponse>, Status> {
        let db = self.mongo.database("cloud");
        let db_users = db.collection::<DbUser>("users");
        let db_user = db_users
            .find_one(
                doc! { "email": request.get_ref().email.to_lowercase().to_owned() },
                None,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or(Status::unauthenticated("invalid credentials"))?;

        let parsed_hash = PasswordHash::new(&db_user.passhash)
            .map_err(|e| Status::internal(e.to_string()))?;

        let verified = Argon2::default().verify_password(request.get_ref().password.as_bytes(), &parsed_hash);

        if verified.is_err() {
            return Err(Status::unauthenticated("invalid credentials"));
        }

        let token = auth_token::create_access_token(db_user.id);

        Ok(Response::new(AuthLoginResponse {
            access_token: token.unwrap(),
            user_id: db_user.id.to_string(),
        }))
    }
}

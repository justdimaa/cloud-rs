use cloud_proto::proto::{self, user_service_server::UserService};
use mongodb::bson::doc;
use tonic::{Request, Response, Status};

use crate::{auth_token, models::DbUser};

#[derive(Debug)]
pub struct MyUserService {
    mongo: mongodb::Client,
}

impl MyUserService {
    pub fn new(mongo: mongodb::Client) -> Self {
        Self { mongo }
    }
}

#[tonic::async_trait]
impl UserService for MyUserService {
    async fn get_self(&self, request: Request<()>) -> Result<Response<proto::User>, Status> {
        let user_id = auth_token::get_user_id_from_request(&request)?;

        let db = self.mongo.database("cloud");
        let db_users = db.collection::<DbUser>("users");

        let db_user = db_users
            .find_one(doc! { "_id": user_id }, None)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        match db_user {
            Some(u) => Ok(Response::new(u.to_proto())),
            None => return Err(Status::not_found("could not find user")),
        }
    }
}

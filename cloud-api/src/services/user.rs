use cloud_proto::proto::{self, user_service_server::UserService, UserGetRequest};
use tonic::{Request, Response, Status};

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
    async fn get(&self, request: Request<UserGetRequest>) -> Result<Response<proto::User>, Status> {
        tracing::debug!("Got a request from {:?}", request.remote_addr());
        todo!()
    }
}

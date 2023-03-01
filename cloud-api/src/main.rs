use std::error::Error;

use cloud_proto::proto::{
    auth_service_server::AuthServiceServer, file_service_server::FileServiceServer,
    user_service_server::UserServiceServer,
};
use mongodb::bson::doc;
use tonic::transport::Server;

use crate::services::{auth::MyAuthService, file::MyFileService, user::MyUserService};

mod auth_token;
mod models;
mod services;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = dotenvy::var("API_DATABASE_URL")?;
    let addr = dotenvy::var("API_URL")?.parse()?;

    tracing::info!("Connecting to database");

    let mongo = mongodb::Client::with_uri_str(&db_url).await?;

    mongo
        .database("cloud")
        .run_command(doc! {"ping": 1}, None)
        .await?;

    tracing::info!("Server listening on {}", addr);

    Server::builder()
        .add_service(AuthServiceServer::new(MyAuthService::new(mongo.clone())))
        .add_service(UserServiceServer::new(MyUserService::new(mongo.clone())))
        .add_service(FileServiceServer::new(MyFileService::new(mongo.clone())))
        .serve(addr)
        .await?;

    Ok(())
}

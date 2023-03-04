use cloud_proto::proto::{
    auth_service_server::AuthServiceServer, file_service_server::FileServiceServer,
    user_service_server::UserServiceServer,
};
use mongodb::bson::doc;
use tonic::transport::Server;

use crate::{
    config::Configuration,
    services::{auth::MyAuthService, file::MyFileService, user::MyUserService},
};

mod auth_token;
mod config;
mod models;
mod services;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Configuration::from_env()?;

    tracing::info!("Connecting to database");
    let mongo = mongodb::Client::with_uri_str(&config.database_url).await?;

    mongo
        .database("cloud")
        .run_command(doc! {"ping": 1}, None)
        .await?;

    tracing::info!("Server listening on {}", &config.server_endpoint);
    Server::builder()
        .add_service(AuthServiceServer::new(MyAuthService::new(
            config.clone(),
            mongo.clone(),
        )))
        .add_service(UserServiceServer::new(MyUserService::new(mongo.clone())))
        .add_service(FileServiceServer::new(MyFileService::new(mongo.clone())))
        .serve(config.server_endpoint.clone())
        .await?;

    Ok(())
}

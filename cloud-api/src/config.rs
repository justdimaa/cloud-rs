use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Configuration {
    pub database_url: String,
    pub server_endpoint: SocketAddr,
    pub user_storage_quota: u64,
}

impl Configuration {
    pub fn from_env() -> Result<Configuration, anyhow::Error> {
        let database_url = dotenvy::var("API_DATABASE_URL")?;
        let server_endpoint = dotenvy::var("API_ENDPOINT").map(|i| i.parse::<SocketAddr>())??;
        let user_storage_quota =
            dotenvy::var("API_USER_STORAGE_QUOTA").map(|i| i.parse::<u64>())??;

        Ok(Configuration {
            database_url,
            server_endpoint,
            user_storage_quota,
        })
    }
}

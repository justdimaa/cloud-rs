use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::fs;

const CONFIG_FILE_NAME: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub url: Option<String>,
    pub credentials: Option<Credentials>,
    pub sync_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

pub async fn read_conf() -> Result<Configuration, anyhow::Error> {
    let conf_path = Path::new(CONFIG_FILE_NAME);

    if conf_path.is_file() {
        let conf_str = fs::read_to_string(conf_path).await?;
        let conf = serde_json::from_str::<Configuration>(&conf_str)?;
        return Ok(conf);
    }

    let conf = Configuration {
        url: None,
        credentials: None,
        sync_dir: None,
    };
    write_conf(&conf).await?;
    Ok(conf)
}

pub async fn write_conf(conf: &Configuration) -> Result<(), anyhow::Error> {
    let conf_path = Path::new(CONFIG_FILE_NAME);
    let conf_json = serde_json::to_string_pretty(conf)?;
    fs::write(conf_path, conf_json).await?;
    Ok(())
}

pub async fn modify_conf<F>(f: F) -> Result<(), anyhow::Error>
where
    F: FnOnce(&mut Configuration),
{
    let mut conf = read_conf().await?;
    f(&mut conf);
    write_conf(&conf).await?;
    Ok(())
}

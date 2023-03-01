use std::path::Path;

use cloud_proto::proto;
use sqlx::{ConnectOptions, Pool, Sqlite};

const DB_FILE_NAME: &str = ".sync.db";

#[derive(Debug)]
pub struct DatabaseService {
    pool: Pool<Sqlite>,
}

impl DatabaseService {
    pub async fn init<P>(sync_dir: P) -> Result<DatabaseService, anyhow::Error>
    where
        P: AsRef<Path>,
    {
        let db_path = sync_dir.as_ref().join(DB_FILE_NAME);
        let mut options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        options.log_statements(tracing::log::LevelFilter::Trace);

        tracing::info!("connecting to sqlite database \"{}\"", db_path.display());
        let pool = sqlx::SqlitePool::connect_with(options).await?;
        let mut db = pool.acquire().await?;

        sqlx::query!(
            "CREATE TABLE IF NOT EXISTS files (
                id TEXT NOT NULL,
                path TEXT NOT NULL,
                hash TEXT NOT NULL,
                CONSTRAINT files_PK PRIMARY KEY (id))"
        )
        .execute(&mut db)
        .await?;

        Ok(DatabaseService { pool })
    }

    pub async fn test(&self) -> Result<(), sqlx::Error> {
        let mut db = self.pool.acquire().await?;

        sqlx::query!(
            "SELECT *
            FROM files"
        )
        .fetch_optional(&mut db)
        .await?;

        Ok(())
    }

    pub async fn add_file(&self, file: &proto::File) -> Result<(), sqlx::Error> {
        let mut db = self.pool.acquire().await?;

        sqlx::query!(
            "INSERT INTO files (id, path, hash)
            VALUES (?1, ?2, ?3)",
            file.id,
            file.path,
            file.hash
        )
        .execute(&mut db)
        .await?;

        Ok(())
    }

    pub async fn replace_file_hash(&self, file: &proto::File) -> Result<(), sqlx::Error> {
        let mut db = self.pool.acquire().await?;

        sqlx::query!(
            "UPDATE files
            SET hash = ?1
            WHERE id = ?2",
            file.hash,
            file.id,
        )
        .execute(&mut db)
        .await?;

        Ok(())
    }

    pub async fn delete_file_by_id(&self, id: String) -> Result<(), sqlx::Error> {
        let mut db = self.pool.acquire().await?;

        sqlx::query!(
            "DELETE FROM files
            WHERE id = ?1",
            id
        )
        .execute(&mut db)
        .await?;

        Ok(())
    }

    pub async fn find_file_by_path<P>(
        &self,
        relative_path: P,
    ) -> Result<Option<DbFile>, sqlx::Error>
    where
        P: AsRef<Path>,
    {
        let mut db = self.pool.acquire().await?;
        let relative_path_str = relative_path.as_ref().to_string_lossy().to_string();

        sqlx::query_as!(
            DbFile,
            "SELECT *
            FROM files
            WHERE path = ?1",
            relative_path_str
        )
        .fetch_optional(&mut db)
        .await
    }

    pub async fn update_file_hash_by_path<P>(
        &self,
        relative_path: P,
        hash: String,
    ) -> Result<bool, sqlx::Error>
    where
        P: AsRef<Path>,
    {
        let mut db = self.pool.acquire().await?;
        let relative_path_str = relative_path.as_ref().to_string_lossy().to_string();

        sqlx::query!(
            "UPDATE files
                SET hash = ?1
                WHERE path = ?2",
            hash,
            relative_path_str
        )
        .execute(&mut db)
        .await
        .map(|x| x.rows_affected() != 0)
    }
}

#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct DbFile {
    pub id: String,
    pub path: String,
    pub hash: String,
}

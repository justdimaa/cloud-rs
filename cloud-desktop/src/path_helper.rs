use std::path::{Path, PathBuf};

use futures::StreamExt;
use tokio::fs;
use tokio_util::io::ReaderStream;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FilePath {
    sync_dir: PathBuf,
    absolute_path: PathBuf,
    relative_path: PathBuf,
}

impl FilePath {
    pub fn from_abs<P, Q>(sync_dir: P, absolute_path: Q) -> Self
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        Self {
            sync_dir: sync_dir.as_ref().to_path_buf(),
            absolute_path: absolute_path.as_ref().to_path_buf(),
            relative_path: abs_to_rel_path(sync_dir, absolute_path),
        }
    }

    pub fn from_rel<P, Q>(sync_dir: P, relative_path: Q) -> Self
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        Self {
            sync_dir: sync_dir.as_ref().to_path_buf(),
            absolute_path: rel_to_abs_path(sync_dir, &relative_path),
            relative_path: relative_path.as_ref().to_path_buf(),
        }
    }

    pub fn get_sync_dir(&self) -> &PathBuf {
        &self.sync_dir
    }

    pub fn get_abs(&self) -> &PathBuf {
        &self.absolute_path
    }

    pub fn get_rel(&self) -> &PathBuf {
        &self.relative_path
    }

    pub fn to_abs_str(&self) -> String {
        self.absolute_path.to_string_lossy().to_string()
    }

    pub fn to_rel_str(&self) -> String {
        self.relative_path.to_string_lossy().to_string()
    }
}

pub fn abs_to_rel_path<P, Q>(sync_dir: P, absolute_path: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    Path::new("/").join(absolute_path.as_ref().strip_prefix(&sync_dir).unwrap())
}

pub fn rel_to_abs_path<P, Q>(sync_dir: P, relative_path: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let mut absolute_path = sync_dir.as_ref().to_path_buf();
    absolute_path.push(
        relative_path
            .as_ref()
            .to_string_lossy()
            .trim_start_matches("/"),
    );
    absolute_path
}

pub fn extract_file_name(path_str: String) -> String {
    path_str.split("/").last().unwrap().to_string()
}

pub async fn read_file_meta(path: &PathBuf) -> Result<(String, u64), anyhow::Error> {
    let file = fs::File::open(path).await?;
    let mut file_stream = ReaderStream::new(file);

    let mut hasher = blake3::Hasher::new();
    let mut size = 0;

    while let Some(bytes) = file_stream.next().await {
        let bytes = bytes?;

        hasher.update(&bytes);
        size += bytes.len();
    }

    let hash = hasher.finalize().to_string();
    Ok((hash, size as u64))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::path_helper;

    #[test]
    fn abs_to_rel() {
        let sync_dir = Path::new("/home/test/cloud").to_path_buf();
        assert_eq!("/home/test/cloud", sync_dir.to_string_lossy());

        assert_eq!(
            "/test.txt",
            path_helper::abs_to_rel_path(&sync_dir, "/home/test/cloud/test.txt").to_string_lossy()
        );

        assert_eq!(
            "/path/test.txt",
            path_helper::abs_to_rel_path(&sync_dir, "/home/test/cloud/path/test.txt")
                .to_string_lossy()
        );
    }

    #[test]
    fn rel_to_abs() {
        let sync_dir = Path::new("/home/test/cloud").to_path_buf();

        assert_eq!(
            "/home/test/cloud/test.txt",
            path_helper::rel_to_abs_path(&sync_dir, "/test.txt".to_owned()).to_string_lossy()
        );

        assert_eq!(
            "/home/test/cloud/path/test.txt",
            path_helper::rel_to_abs_path(&sync_dir, "/path/test.txt".to_owned()).to_string_lossy()
        );
    }
}

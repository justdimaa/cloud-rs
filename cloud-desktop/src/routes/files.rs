use std::{cmp::Ordering, collections::BTreeMap, path::Path, sync::Arc};

use anyhow::anyhow;
use cloud_proto::proto;
use dioxus::prelude::*;
use fermi::UseAtomRef;
use futures::StreamExt;
use tokio::{fs, sync::Mutex};
use walkdir::WalkDir;

use crate::{
    components::{FileElement, FileElementProps, FileStatus},
    global_state,
    path_helper::{self, FilePath},
    services::{
        api_service::{FileApiService, UserApiService},
        database_service::{DatabaseService, DbFile},
    },
};

#[derive(Debug, Clone)]
pub enum HandleFileCommand {
    Refresh,
    Skip(FilePath),
    KeepLocal(FilePromptKeep),
    KeepRemote(FilePromptKeep),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilePromptKeep {
    pub file_path: FilePath,
    pub local_meta: (String, u64),
    pub sql_file: Option<DbFile>,
    pub remote_file: proto::File,
}

pub fn Files(cx: Scope) -> Element {
    let sync_dir = fermi::use_read(cx, global_state::SYNC_DIR)
        .as_ref()
        .unwrap();
    let storage_space = use_state(cx, || "".to_string());
    let db_service = fermi::use_atom_state(cx, global_state::DATABASE_SERVICE);
    let user_service = fermi::use_atom_state(cx, global_state::USER_API_SERVICE);
    let file_service = fermi::use_atom_state(cx, global_state::FILE_API_SERVICE);
    let files = fermi::use_atom_ref(cx, global_state::FILES);
    let coroutine_handle = use_coroutine(cx, |rx: UnboundedReceiver<HandleFileCommand>| {
        handle_file_coroutine(
            rx,
            db_service.get().as_ref().unwrap().clone(),
            user_service.get().as_ref().unwrap().clone(),
            file_service.get().as_ref().unwrap().clone(),
            files.clone(),
            sync_dir.clone(),
            storage_space.clone(),
        )
    });

    let files = files.read();
    let files = files.values();
    let mut files = files.collect::<Vec<_>>();

    files.sort_by(|a, b| match (&a.status, &b.status) {
        (FileStatus::WaitingUser(_), _) => Ordering::Less,
        (_, FileStatus::WaitingUser(_)) => Ordering::Greater,
        (_, _) => Ordering::Equal,
    });

    cx.render(rsx! {
        div {
            class: "w-full h-full p-4 bg-white sm:p-8 dark:bg-gray-800",
            div {
                class: "flex items-center justify-between mb-4",
                div {
                    class: "flex-1 min-w-0",
                    h5 {
                        class: "text-xl font-medium text-gray-900 dark:text-white",
                        "Files"
                    },
                    p {
                        class: "text-sm text-gray-500 truncate dark:text-gray-400",
                        "{storage_space}"
                    }
                }
                button {
                    onclick: |_| {
                        coroutine_handle.send(
                            HandleFileCommand::Refresh
                        )
                    },
                    class: "text-sm font-medium text-blue-600 hover:underline dark:text-blue-500",
                    "Refresh"
                }
            }
            div {
                class: "flow-root",
                ul {
                    role: "list",
                    class: "divide-y divide-gray-200 dark:divide-gray-700",
                    files.into_iter().map(|v|
                        rsx! {
                            FileElement {
                                status: v.status.clone(),
                                path: v.path.clone(),
                                size: v.size,
                            }
                        }
                    )
                }
            }
        }
    })
}

async fn handle_file_coroutine<P>(
    mut rx: UnboundedReceiver<HandleFileCommand>,
    db_service: Arc<DatabaseService>,
    user_service: Arc<Mutex<UserApiService>>,
    file_service: Arc<Mutex<FileApiService>>,
    files: UseAtomRef<BTreeMap<String, FileElementProps>>,
    sync_dir: P,
    storage_space: UseState<String>,
) where
    P: AsRef<Path>,
{
    while let Some(cmd) = rx.next().await {
        match cmd {
            HandleFileCommand::Refresh => {
                let mut user_service = user_service.lock().await;
                let mut file_service = file_service.lock().await;
                on_refresh(
                    &db_service,
                    &mut user_service,
                    &mut file_service,
                    &files,
                    &sync_dir,
                    &storage_space,
                )
                .await;
            }
            HandleFileCommand::Skip(path) => {
                let mut files = files.write();
                let props = files.get_mut(&path.to_rel_str()).unwrap();
                props.status = FileStatus::Success;
            }
            HandleFileCommand::KeepLocal(keep) => {
                let mut file_service = file_service.lock().await;

                {
                    let mut files = files.write();
                    let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                    props.size = keep.local_meta.1;
                    props.status = FileStatus::WaitingQueue;
                }

                if let Some(sql_file) = &keep.sql_file {
                    if let Err(e) = db_service.delete_file_by_id(sql_file.id.to_owned()).await {
                        tracing::error!("failed to delete sql entry {:?}", e);
                        let mut files = files.write();
                        let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                        props.status = FileStatus::Failed;
                        return;
                    }
                }

                let uploaded_file =
                    upload_file(&mut file_service, &keep.file_path, &keep.local_meta).await;

                match uploaded_file {
                    Ok(uploaded_file) => {
                        if let Err(e) = db_service.add_file(&uploaded_file).await {
                            tracing::error!("failed to add sql entry {:?}", e);
                            let mut files = files.write();
                            let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                            props.status = FileStatus::Failed;
                            return;
                        }
                    }
                    Err(e) => {
                        tracing::error!("failed to upload file {:?}", e);
                        let mut files = files.write();
                        let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                        props.status = FileStatus::Failed;
                        return;
                    }
                }

                let mut files = files.write();
                let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                props.status = FileStatus::Success;
            }
            HandleFileCommand::KeepRemote(keep) => {
                let mut file_service = file_service.lock().await;

                {
                    let mut files = files.write();
                    let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                    props.size = keep.remote_file.size;
                    props.status = FileStatus::WaitingQueue;
                }

                if let Some(sql_file) = keep.sql_file {
                    if let Err(e) = db_service.delete_file_by_id(sql_file.id.to_owned()).await {
                        tracing::error!("failed to delete sql entry {:?}", e);
                        let mut files = files.write();
                        let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                        props.status = FileStatus::Failed;
                        return;
                    }
                }

                if let Err(e) = fs::remove_file(keep.file_path.get_abs()).await {
                    tracing::error!("failed to remove file {:?}", e);
                    let mut files = files.write();
                    let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                    props.status = FileStatus::Failed;
                    return;
                }

                if let Err(e) = download_file(
                    &mut file_service,
                    keep.file_path.get_sync_dir(),
                    &keep.remote_file,
                )
                .await
                {
                    tracing::error!("failed to download file {:?}", e);
                    let mut files = files.write();
                    let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                    props.status = FileStatus::Failed;
                    return;
                }

                if let Err(e) = db_service.add_file(&keep.remote_file).await {
                    tracing::error!("failed to add sql entry {:?}", e);
                    let mut files = files.write();
                    let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                    props.status = FileStatus::Failed;
                    return;
                }

                let mut files = files.write();
                let props = files.get_mut(&keep.file_path.to_rel_str()).unwrap();
                props.status = FileStatus::Success;
            }
        }
    }
}

async fn on_refresh<P>(
    db_service: &DatabaseService,
    user_service: &mut UserApiService,
    file_service: &mut FileApiService,
    files: &UseAtomRef<BTreeMap<String, FileElementProps>>,
    sync_dir: P,
    storage_space: &UseState<String>,
) where
    P: AsRef<Path>,
{
    files.write().clear();

    sync_local_files(db_service, file_service, files, &sync_dir)
        .await
        .ok();
    sync_api_files(db_service, file_service, files, &sync_dir)
        .await
        .ok();

    match user_service.get_self().await {
        Ok(u) => {
            let cur = byte_unit::Byte::from_bytes(u.storage_used.into()).get_appropriate_unit(true);

            let max = match u.storage_quota {
                Some(storage_quota) => byte_unit::Byte::from_bytes(storage_quota.into())
                    .get_appropriate_unit(true)
                    .to_string(),
                None => "∞".to_string(),
            };

            storage_space.set(format!("{} / {} used", cur, max))
        }
        Err(e) => {
            storage_space.set(e.to_string());
        }
    }
}

async fn sync_local_files<P>(
    db_service: &DatabaseService,
    file_service: &mut FileApiService,
    files: &UseAtomRef<BTreeMap<String, FileElementProps>>,
    sync_dir: P,
) -> Result<(), anyhow::Error>
where
    P: AsRef<Path>,
{
    let walk_dir = WalkDir::new(&sync_dir);

    for entry in walk_dir.into_iter() {
        if let Err(e) = &entry {
            tracing::error!("failed to read entry {}", e);
        }

        let entry = entry.unwrap();
        let entry_path = entry.path();

        if !entry_path.is_file() {
            continue;
        }

        let metadata = entry.metadata();

        if let Err(e) = &metadata {
            tracing::error!("failed to retrieve entry metadata {}", e);
        }

        let size = metadata.unwrap().len();
        let file_path = FilePath::from_abs(&sync_dir, entry_path);

        process_path(db_service, file_service, files, file_path, size).await;
    }

    Ok(())
}

async fn sync_api_files<P>(
    db_service: &DatabaseService,
    file_service: &mut FileApiService,
    files: &UseAtomRef<BTreeMap<String, FileElementProps>>,
    sync_dir: P,
) -> Result<(), anyhow::Error>
where
    P: AsRef<Path>,
{
    let mut get_resp = file_service.get_client().get_all(()).await?.into_inner();

    while let Some(Ok(api_file)) = get_resp.next().await {
        let file_path = FilePath::from_rel(&sync_dir, &api_file.path);
        process_path(db_service, file_service, files, file_path, api_file.size).await;
    }

    Ok(())
}
async fn process_path(
    db_service: &DatabaseService,
    file_service: &mut FileApiService,
    files: &UseAtomRef<BTreeMap<String, FileElementProps>>,
    file_path: FilePath,
    size: u64,
) {
    if files.read().contains_key(&file_path.to_rel_str()) {
        return;
    }

    let file_name = file_path.get_rel().file_name().unwrap().to_string_lossy();

    if file_name == ".sync.db" {
        return;
    }

    if file_name.starts_with(".~download~") {
        return;
    }

    files.write().insert(
        file_path.to_rel_str(),
        FileElementProps {
            status: FileStatus::WaitingQueue,
            path: file_path.clone(),
            size,
        },
    );

    let status = sync_file(db_service, file_service, &file_path).await;

    match status {
        Ok(status) => {
            let mut files = files.write();
            let props = files.get_mut(&file_path.to_rel_str()).unwrap();
            props.status = status;
        }
        Err(e) => {
            tracing::error!("failed to sync file {:?}", e);
            let mut files = files.write();
            let props = files.get_mut(&file_path.to_rel_str()).unwrap();
            props.status = FileStatus::Failed;
        }
    }
}

/// compares the local file hash to the local database hash to the api hash
/// and replaces the older file with newer file
///
/// file existence:
///
/// | action                | filesystem | local database | server api |
/// |-----------------------|------------|----------------|------------|
/// | do nothing            |     0      |        0       |      0     |
/// | download              |     0      |        0       |      1     |
/// | delete sql            |     0      |        1       |      0     |
/// | table011              |     0      |        1       |      1     |
/// | upload                |     1      |        0       |      0     |
/// | table101              |     1      |        0       |      1     |
/// | table110              |     1      |        1       |      0     |
/// | table111              |     1      |        1       |      1     |
///
/// file hash table011:
///
/// | action                | local database | server api |
/// |-----------------------|----------------|------------|
/// | delete sql, api       |        a       |      a     |
/// | delete sql, download  |        a       |      b     |
///
/// file hash table101:
///
/// | action                | filesystem | server api |
/// |-----------------------|------------|------------|
/// | add sql               |     a      |      a     |
/// | ask user              |     a      |      b     |
///
/// file hash table110:
///
/// | action                | filesystem | local database |
/// |-----------------------|------------|----------------|
/// | delete fs, sql        |     a      |        a       |
/// | upload, update sql    |     a      |        b       |
///
/// file hash table111:
///
/// | action                | filesystem | local database | server api |
/// |-----------------------|------------|----------------|------------|
/// | do nothing            |     a      |        a       |      a     |
/// | download              |     a      |        a       |      b     |
/// | upload                |     a      |        b       |      b     |
/// | ask user              |     a      |        b       |      c     |
async fn sync_file(
    db_service: &DatabaseService,
    file_service: &mut FileApiService,
    file_path: &FilePath,
) -> Result<FileStatus, anyhow::Error> {
    tracing::info!("syncing file {:?}", file_path.get_abs());

    let sql_file = db_service.find_file_by_path(&file_path.get_rel()).await?;

    let remote_file = file_service
        .get_client()
        .find(proto::FindFileRequest {
            path: file_path.to_rel_str(),
        })
        .await
        .map(|f| Some(f.into_inner()));

    let remote_file = match remote_file {
        Ok(x) => Ok(x),
        Err(e) => match e.code() {
            tonic::Code::NotFound => Ok(None),
            _ => Err(e),
        },
    }?;

    let local_meta = match file_path.get_abs().exists() {
        true => Some(path_helper::read_file_meta(file_path.get_abs()).await?),
        false => None,
    };

    match (local_meta, sql_file, remote_file) {
        (None, None, None) => {
            // do nothing
            return Ok(FileStatus::Success);
        }
        (None, None, Some(remote_file)) => {
            tracing::debug!("downloading api file");

            // download file
            download_file(file_service, file_path.get_sync_dir(), &remote_file).await?;
            db_service.add_file(&remote_file).await?;
            return Ok(FileStatus::Added);
        }
        (None, Some(sql_file), None) => {
            tracing::debug!("deleting sql entry");

            // delete sql entry
            db_service.delete_file_by_id(sql_file.id).await?;
            return Ok(FileStatus::Success);
        }
        (None, Some(sql_file), Some(remote_file)) => {
            if sql_file.hash == remote_file.hash {
                tracing::debug!("deleting api file");

                // local file deleted
                file_service.delete_file(sql_file.id.to_owned()).await?;
                db_service.delete_file_by_id(sql_file.id).await?;
                return Ok(FileStatus::Deleted);
            } else {
                tracing::debug!("replacing local file with api file");

                // old local file deleted, new file on server
                // download new file
                db_service.delete_file_by_id(sql_file.id).await?;
                download_file(file_service, file_path.get_sync_dir(), &remote_file).await?;
                db_service.add_file(&remote_file).await?;
                return Ok(FileStatus::Success);
            }
        }
        (Some(local_meta), None, None) => {
            tracing::debug!("uploading local file");

            // upload file and add to local db
            let remote_file = upload_file(file_service, file_path, &local_meta).await?;
            db_service.add_file(&remote_file).await?;
            return Ok(FileStatus::Added);
        }
        (Some(local_meta), None, Some(remote_file)) => {
            if local_meta.0 == remote_file.hash {
                tracing::debug!("adding sql entry");
                db_service.add_file(&remote_file).await?;
                return Ok(FileStatus::Success);
            } else {
                // local file changed while it already existed on the server but has not been synced
                // ask user whether to keep local file, keep api file, or rename local file
                return Ok(FileStatus::WaitingUser(FilePromptKeep {
                    file_path: file_path.clone(),
                    local_meta,
                    sql_file: None,
                    remote_file,
                }));
            }
        }
        (Some(local_meta), Some(sql_file), None) => {
            if local_meta.0 == sql_file.hash {
                tracing::debug!("deleting local file");

                // server file deleted
                fs::remove_file(file_path.get_abs()).await?;
                db_service.delete_file_by_id(sql_file.id).await?;
                return Ok(FileStatus::Deleted);
            } else {
                tracing::debug!("uploading local file");

                // local file changed
                db_service.delete_file_by_id(sql_file.id).await?;
                let remote_file = upload_file(file_service, file_path, &local_meta).await?;
                db_service.add_file(&remote_file).await?;
                return Ok(FileStatus::Success);
            }
        }
        (Some(local_meta), Some(sql_file), Some(remote_file)) => {
            if local_meta.0 == sql_file.hash && local_meta.0 == remote_file.hash {
                // do nothing
                tracing::debug!("already synced");
                return Ok(FileStatus::Success);
            } else if local_meta.0 == sql_file.hash {
                tracing::debug!("replacing local file with api file");
                // download
                download_file(file_service, file_path.get_sync_dir(), &remote_file).await?;
                db_service.replace_file_hash(&remote_file).await?;
                return Ok(FileStatus::Success);
            } else if sql_file.hash == remote_file.hash {
                tracing::debug!("replacing api file with local file");

                // local file modified, upload new file
                db_service.delete_file_by_id(sql_file.id).await?;
                let remote_file = upload_file(file_service, file_path, &local_meta).await?;
                db_service.add_file(&remote_file).await?;
                return Ok(FileStatus::Success);
            } else {
                // local file and api file have been changed since the last sync
                // ask user whether to keep local file, keep api file, or rename local file
                return Ok(FileStatus::WaitingUser(FilePromptKeep {
                    file_path: file_path.clone(),
                    local_meta,
                    sql_file: Some(sql_file),
                    remote_file,
                }));
            }
        }
    }
}

async fn upload_file(
    file_service: &mut FileApiService,
    file_path: &FilePath,
    file_meta: &(String, u64),
) -> Result<proto::File, anyhow::Error> {
    let fs_file = fs::File::open(file_path.get_abs()).await?;
    let api_file = {
        file_service
            .upload_file(
                fs_file,
                proto::UploadInfo {
                    path: file_path.to_rel_str(),
                    hash: file_meta.0.to_owned(),
                    size: file_meta.1,
                },
            )
            .await?
    };
    Ok(api_file)
}

async fn download_file<P>(
    file_service: &mut FileApiService,
    sync_dir: P,
    api_file: &proto::File,
) -> Result<(), anyhow::Error>
where
    P: AsRef<Path>,
{
    file_service.download_file(&sync_dir, api_file).await?;

    let absolute_path = path_helper::rel_to_abs_path(&sync_dir, api_file.path.to_owned());
    let file_meta = path_helper::read_file_meta(&absolute_path).await;

    if let Err(e) = file_meta {
        fs::remove_file(absolute_path).await?;
        return Err(e);
    }

    let file_meta = file_meta.unwrap();

    if api_file.hash != file_meta.0 {
        fs::remove_file(absolute_path).await?;
        return Err(anyhow!("downloaded file hash does not match api file hash"));
    }

    Ok(())
}

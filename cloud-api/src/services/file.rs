use std::{path::Path, pin::Pin};

use chrono::Utc;
use cloud_proto::proto::{
    self, file_service_server::FileService, upload_file_request::Upload, DeleteFileRequest,
    DownloadFileRequest, DownloadFileResponse, FindFileRequest, GetFileRequest, UploadFileRequest,
};
use futures_util::{AsyncWriteExt, StreamExt};
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::{GridFsUploadOptions, UpdateOptions},
};
use tokio_util::{compat::FuturesAsyncReadCompatExt, io::ReaderStream};
use tonic::{codegen::futures_core::Stream, Request, Response, Status, Streaming};

use crate::{
    auth_token,
    models::{DbFile, DbUser},
};

#[derive(Debug)]
pub struct MyFileService {
    mongo: mongodb::Client,
}

impl MyFileService {
    pub fn new(mongo: mongodb::Client) -> Self {
        Self { mongo }
    }
}

#[tonic::async_trait]
impl FileService for MyFileService {
    type DownloadStream = Pin<Box<dyn Stream<Item = Result<DownloadFileResponse, Status>> + Send>>;
    type GetAllStream = Pin<Box<dyn Stream<Item = Result<proto::File, Status>> + Send>>;

    async fn upload(
        &self,
        request: Request<Streaming<UploadFileRequest>>,
    ) -> Result<Response<proto::File>, Status> {
        let user_id = auth_token::get_user_id_from_request(&request)?;

        let db = self.mongo.database("cloud");
        let db_users = db.collection::<DbUser>("users");
        let db_files = db.collection::<DbFile>("files");

        let db_user = db_users
            .find_one(doc! { "_id": user_id }, None)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let db_user = match db_user {
            Some(u) => u,
            None => return Err(Status::failed_precondition("could not find user")),
        };

        let bucket = db.gridfs_bucket(None);

        let mut client_stream = request.into_inner();
        let mut client_file_info = None;

        let mut bucket_stream = None;

        let mut hasher = blake3::Hasher::new();
        let mut size = 0;

        let mut db_file = None;

        while let Some(msg) = client_stream.message().await? {
            let upload = msg.upload.unwrap();

            match upload {
                Upload::Info(info) => {
                    if client_file_info.is_some() {
                        return Err(Status::invalid_argument("file meta already sent"));
                    }

                    if let Some(storage_quota) = db_user.storage_quota {
                        if db_user.storage_used + info.size > storage_quota {
                            return Err(Status::resource_exhausted("user storage quota exceeded"));
                        }
                    }

                    let path = Path::new(&info.path);

                    if !path.is_absolute() {
                        return Err(Status::invalid_argument("path is not absolute"));
                    }

                    match path.file_name() {
                        Some(file_name) => {
                            if file_name == ".sync.db" {
                                return Err(Status::invalid_argument(
                                    "file name can not be .sync.db",
                                ));
                            }

                            if file_name.to_string_lossy().starts_with(".~download~") {
                                return Err(Status::invalid_argument(
                                    "file name can not start with .~download~",
                                ));
                            }
                        }
                        None => {
                            return Err(Status::invalid_argument("no file name specified"));
                        }
                    }

                    db_file = db_files
                        .find_one(
                            Some(doc! {
                                "owner_id": user_id.to_owned(),
                                "path": info.path.to_owned(),
                            }),
                            None,
                        )
                        .await
                        .map_err(|e| Status::internal(e.to_string()))?;

                    bucket_stream = Some(bucket.open_upload_stream(
                        &info.path,
                        Some(GridFsUploadOptions::builder().metadata(None).build()),
                    ));
                    client_file_info = Some(info);
                }
                Upload::Chunk(bytes) => {
                    let info = match &client_file_info {
                        Some(i) => i,
                        None => {
                            return Err(Status::invalid_argument(
                                "file metadata must be sent before the byte stream",
                            ))
                        }
                    };

                    size += bytes.len();

                    if size as u64 > info.size {
                        return Err(Status::aborted(
                            "uploaded file size exceeds the announced file size",
                        ));
                    }

                    hasher.update(&bytes);

                    bucket_stream
                        .as_mut()
                        .unwrap()
                        .write_all(&bytes)
                        .await
                        .map_err(|_e| Status::internal("bucket stream"))?;
                }
            }
        }

        if bucket_stream.is_none() {
            return Err(Status::invalid_argument("no data received"));
        }

        let hash = hasher.finalize().to_string();

        let client_file_info = client_file_info.unwrap();
        let mut bucket_stream = bucket_stream.unwrap();
        bucket_stream.close().await?;

        if hash != client_file_info.hash {
            return Err(Status::data_loss(format!(
                "hash(server: {}, client: {}) do not match",
                hash, client_file_info.hash
            )));
        }

        if size != client_file_info.size as usize {
            return Err(Status::data_loss(format!(
                "size(server: {}, client: {}) do not match",
                size, client_file_info.size
            )));
        }

        let bucket_id = bucket_stream.id().as_object_id().unwrap();

        match db_file.as_mut() {
            Some(mut db_file) => {
                bucket
                    .delete(db_file.bucket_id.into())
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;

                db_file.bucket_id = bucket_id;
                db_file.hash = hash;
                db_file.size = size as u64;
                db_file.modified_at = Utc::now();

                db_files
                    .replace_one(
                        doc! {
                            "_id": db_file.id,
                        },
                        db_file,
                        None,
                    )
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;
            }
            None => {
                let new_db_file = DbFile {
                    id: ObjectId::new(),
                    owner_id: user_id,
                    bucket_id,
                    path: client_file_info.path.to_owned(),
                    hash: client_file_info.hash.to_owned(),
                    size: client_file_info.size,
                    modified_at: Utc::now(),
                };

                db_files
                    .insert_one(&new_db_file, None)
                    .await
                    .map_err(|e| Status::internal(e.to_string()))?;

                db_file = Some(new_db_file);
            }
        }

        db_users
            .update_one(
                doc! { "_id": user_id },
                doc! { "$inc": { "storage_used": client_file_info.size as i64 } },
                UpdateOptions::builder().upsert(Some(true)).build(),
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        tracing::debug!(
            "uploaded file {} with hash {}",
            client_file_info.path,
            client_file_info.hash
        );

        Ok(Response::new(db_file.unwrap().to_proto()))
    }

    async fn download(
        &self,
        request: Request<DownloadFileRequest>,
    ) -> Result<Response<Self::DownloadStream>, Status> {
        let user_id = auth_token::get_user_id_from_request(&request)?;

        let file_id = ObjectId::parse_str(request.get_ref().id.to_owned())
            .map_err(|_| Status::invalid_argument("invalid id"))?;

        let db = self.mongo.database("cloud");
        let db_files = db.collection::<DbFile>("files");

        let db_file = db_files
            .find_one(Some(doc! { "_id": file_id, "owner_id": user_id }), None)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or(Status::not_found("file not found"))?;

        let bucket = db.gridfs_bucket(None);

        let bucket_stream = bucket
            .open_download_stream(db_file.bucket_id.into())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let stream = ReaderStream::new(bucket_stream.compat()).map(|f| match f {
            Ok(f) => Ok(DownloadFileResponse { chunk: f.to_vec() }),
            Err(e) => Err(Status::internal(e.to_string())),
        });

        return Ok(Response::new(Box::pin(stream)));
    }

    async fn get(&self, request: Request<GetFileRequest>) -> Result<Response<proto::File>, Status> {
        let user_id = auth_token::get_user_id_from_request(&request)?;

        let file_id = ObjectId::parse_str(request.get_ref().id.to_owned())
            .map_err(|_| Status::invalid_argument("invalid id"))?;

        let db = self.mongo.database("cloud");
        let db_files = db.collection::<DbFile>("files");

        let db_file = db_files
            .find_one(Some(doc! { "_id": file_id, "owner_id": user_id }), None)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or(Status::not_found("file not found"))?;

        Ok(Response::new(db_file.to_proto()))
    }

    async fn find(
        &self,
        request: Request<FindFileRequest>,
    ) -> Result<Response<proto::File>, Status> {
        let user_id = auth_token::get_user_id_from_request(&request)?;

        let db = self.mongo.database("cloud");
        let db_files = db.collection::<DbFile>("files");

        let db_file = db_files
            .find_one(
                Some(doc! { "owner_id": user_id, "path": request.get_ref().path.to_owned() }),
                None,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        match db_file {
            Some(db_file) => Ok(Response::new(db_file.to_proto())),
            None => Err(Status::not_found("file not found")),
        }
    }

    async fn get_all(&self, request: Request<()>) -> Result<Response<Self::GetAllStream>, Status> {
        let user_id = auth_token::get_user_id_from_request(&request)?;

        let db = self.mongo.database("cloud");
        let db_files = db.collection::<DbFile>("files");

        let cursor = db_files
            .find(doc! { "owner_id": user_id }, None)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map(|f| match f {
                Ok(f) => Ok(f.to_proto()),
                Err(e) => Err(Status::internal(e.to_string())),
            });

        Ok(Response::new(Box::pin(cursor)))
    }

    async fn delete(&self, request: Request<DeleteFileRequest>) -> Result<Response<()>, Status> {
        let user_id = auth_token::get_user_id_from_request(&request)?;

        let file_id = ObjectId::parse_str(request.get_ref().id.to_owned())
            .map_err(|_| Status::invalid_argument("invalid id"))?;

        let db = self.mongo.database("cloud");
        let db_users = db.collection::<DbUser>("users");
        let db_files = db.collection::<DbFile>("files");

        let db_file = db_files
            .find_one(doc! { "_id": file_id, "owner_id": user_id }, None)
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or(Status::not_found("file not found"))?;

        let bucket = db.gridfs_bucket(None);

        bucket
            .delete(db_file.bucket_id.into())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        db_files
            .delete_one(doc! { "_id": file_id, "owner_id": user_id }, None)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        db_users
            .update_one(
                doc! { "_id": user_id },
                doc! { "$inc": { "storage_used": -(db_file.size as i64) } },
                UpdateOptions::builder().upsert(Some(true)).build(),
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(()))
    }
}

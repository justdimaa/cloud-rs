use std::path::Path;

use cloud_proto::proto::{
    self, auth_service_client::AuthServiceClient, file_service_client::FileServiceClient,
};
use futures::StreamExt;
use tokio::{fs, io::AsyncWriteExt};
use tokio_util::io::ReaderStream;
use tonic::{codegen::InterceptedService, service::Interceptor, transport::Channel};

use crate::path_helper;

pub struct AuthInterceptor {
    pub access_token: String,
}

impl AuthInterceptor {
    fn new(access_token: String) -> Self {
        Self { access_token }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        let authorization = format!("Baerer {}", self.access_token);
        let authorization: tonic::metadata::MetadataValue<_> = authorization.parse().unwrap();

        request
            .metadata_mut()
            .insert("authorization", authorization);
        Ok(request)
    }
}

pub struct AuthApiService {
    client: AuthServiceClient<Channel>,
}

impl AuthApiService {
    pub fn new(channel: Channel) -> Result<Self, tonic::transport::Error> {
        let client = AuthServiceClient::new(channel);
        Ok(Self { client })
    }

    pub fn get_client(&mut self) -> &mut AuthServiceClient<Channel> {
        &mut self.client
    }
}

pub struct FileApiService {
    client: FileServiceClient<InterceptedService<Channel, AuthInterceptor>>,
}

impl FileApiService {
    pub fn new(channel: Channel, access_token: String) -> FileApiService {
        FileApiService {
            client: FileServiceClient::with_interceptor(
                channel,
                AuthInterceptor::new(access_token),
            ),
        }
    }

    pub fn get_client(
        &mut self,
    ) -> &mut FileServiceClient<InterceptedService<Channel, AuthInterceptor>> {
        &mut self.client
    }

    pub async fn delete_file(&mut self, file_id: String) -> Result<(), anyhow::Error> {
        self.client
            .delete(proto::DeleteFileRequest { id: file_id })
            .await?;

        Ok(())
    }

    pub async fn download_file<P>(
        &mut self,
        sync_dir: P,
        api_file: &proto::File,
    ) -> Result<(), anyhow::Error>
    where
        P: AsRef<Path>,
    {
        let mut download_res = self
            .client
            .download(proto::DownloadFileRequest {
                id: api_file.id.to_owned(),
            })
            .await?
            .into_inner();

        let absolute_path_download =
            path_helper::rel_to_abs_path(&sync_dir, api_file.path.to_owned()).with_file_name(
                format!(
                    ".~download~{}",
                    path_helper::extract_file_name(api_file.path.to_owned())
                ),
            );
        let absolute_path = path_helper::rel_to_abs_path(&sync_dir, api_file.path.to_owned());

        if let Some(parent) = absolute_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut fs_file = fs::File::create(&absolute_path_download).await?;
        // let mut downloaded_size = 0;

        while let Some(Ok(api_data)) = download_res.next().await {
            // downloaded_size += api_data.chunk.len() as u64;
            fs_file.write_all(&api_data.chunk).await?;
        }

        fs_file.shutdown().await?;
        fs::rename(absolute_path_download, absolute_path).await?;
        Ok(())
    }

    pub async fn upload_file(
        &mut self,
        fs_file: fs::File,
        info: proto::UploadInfo,
    ) -> Result<proto::File, anyhow::Error> {
        // let mut uploaded_size = 0;
        let mut file_stream = ReaderStream::new(fs_file);

        let upload_stream = async_stream::stream! {
            yield proto::UploadFileRequest {
                 upload: Some(proto::upload_file_request::Upload::Info(info)),
             };

            while let Some(f) = file_stream.next().await {
                let f = f.unwrap();
                // uploaded_size += f.len() as u64;
                // change_info_fn(Some(uploaded_size), components::FileStatus::Uploading);

                yield proto::UploadFileRequest {
                    upload: Some(proto::upload_file_request::Upload::Chunk(f.to_vec())),
                }
            }
        };

        let upload_response = self.client.upload(upload_stream).await?;
        Ok(upload_response.into_inner())
    }
}

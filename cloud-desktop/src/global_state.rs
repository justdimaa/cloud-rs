use std::{collections::BTreeMap, sync::Arc};

use fermi::{Atom, AtomRef};
use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::{
    components::FileElementProps,
    services::{api_service::FileApiService, database_service::DatabaseService},
};

pub static SYNC_DIR: Atom<Option<String>> = |_| None;
pub static API_CHANNEL: Atom<Option<Channel>> = |_| None;

pub static DATABASE_SERVICE: Atom<Option<Arc<DatabaseService>>> = |_| None;
pub static FILE_API_SERVICE: Atom<Option<Arc<Mutex<FileApiService>>>> = |_| None;

pub static FILES: AtomRef<BTreeMap<String, FileElementProps>> = |_| BTreeMap::new();

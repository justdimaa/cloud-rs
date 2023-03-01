use std::{path::Path, sync::Arc};

use cloud_proto::proto;
use dioxus::prelude::*;
use dioxus_router::{use_router, RouterContext};
use fermi::{use_atom_state, AtomState};
use futures::executor::block_on;
use tokio::sync::Mutex;
use tonic::transport::{Channel, Uri};

use crate::{
    config, global_state,
    services::{
        api_service::{AuthApiService, FileApiService},
        database_service::DatabaseService,
    },
};

#[derive(Clone, Copy)]
struct SetupData<'a> {
    router: &'a RouterContext,
    step: &'a UseState<SetupStep>,

    is_loading: &'a UseState<bool>,
    error_status: &'a UseState<String>,

    url_field: &'a UseState<String>,
    email_field: &'a UseState<String>,
    password_field: &'a UseState<String>,
    sync_dir_field: &'a UseState<String>,

    sync_dir_set: &'a AtomState<Option<String>>,
    database_service: &'a AtomState<Option<Arc<DatabaseService>>>,
    file_api_service: &'a AtomState<Option<Arc<Mutex<FileApiService>>>>,
    api_channel: &'a AtomState<Option<Channel>>,
}

#[derive(Debug)]
enum SetupStep {
    Url,
    Login,
    Register,
    SyncDir,
}

pub fn Setup(cx: Scope) -> Element {
    let conf = block_on(config::read_conf()).unwrap();

    let data = SetupData {
        router: use_router(cx),
        is_loading: use_state(cx, || false),
        step: use_state(cx, || SetupStep::Url),
        error_status: use_state(cx, || "".to_owned()),
        url_field: use_state(cx, || conf.url.clone().unwrap_or_default()),
        email_field: use_state(cx, || {
            conf.credentials
                .as_ref()
                .map_or(String::new(), |c| c.email.to_owned())
        }),
        password_field: use_state(cx, || {
            conf.credentials
                .as_ref()
                .map_or(String::new(), |c| c.password.to_owned())
        }),
        sync_dir_field: use_state(cx, || conf.sync_dir.clone().unwrap_or_default()),
        // access_token: use_atom_state(cx, global_state::ACCESS_TOKEN),
        sync_dir_set: use_atom_state(cx, global_state::SYNC_DIR),
        database_service: use_atom_state(cx, global_state::DATABASE_SERVICE),
        file_api_service: use_atom_state(cx, global_state::FILE_API_SERVICE),
        api_channel: use_atom_state(cx, global_state::API_CHANNEL),
    };

    cx.render(rsx! {
        div {
            class: "flex flex-row h-screen justify-center items-center",
            div {
                class: "w-full h-full p-4 bg-white sm:p-6 md:p-8 dark:bg-gray-800",
                match data.step.get() {
                    SetupStep::Url => rsx! {
                        form {
                            onsubmit: move |e| { on_submit_url(cx, data, e) },
                            class: "space-y-6",
                            h5 {
                                class: "text-xl font-bold leading-none text-gray-900",
                                "Sign in"
                            },
                            div {
                                label {
                                    class: "block mb-2 text-sm font-medium text-gray-900 dark:text-white",
                                    "Server url"
                                }
                                input {
                                    class: "bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-600 dark:border-gray-500 dark:placeholder-gray-400 dark:text-white",
                                    required: true,
                                    name: "url",
                                    disabled: "{data.is_loading}",
                                    value: "{data.url_field}"
                                }
                            }
                            div {
                                div {
                                    class: "text-red-800 text-sm pb-3",
                                    "{data.error_status}"
                                }
                                button {
                                    r#type: "submit",
                                    class: "w-full text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800",
                                    disabled: "{data.is_loading}",
                                    "Continue",
                                }
                            }
                        }
                    },
                    SetupStep::Login => rsx! {
                        form {
                            onsubmit: move |e| { on_submit_login(cx, data, e) },
                            class: "space-y-6",
                            div {
                                class: "flex items-center justify-between mb-4",
                                h5 {
                                    class: "text-xl font-bold leading-none text-gray-900",
                                    "Sign in"
                                }
                                button {
                                    onclick: |_| { data.step.set(SetupStep::Url) },
                                    class: "text-xl text-gray-500",
                                    disabled: "{data.is_loading}",
                                    i { class: "fa-solid fa-arrow-left mr-3" }
                                    "Back to url"
                                }
                            }
                            div {
                                label {
                                    class: "block mb-2 text-sm font-medium text-gray-900",
                                    "Email"
                                }
                                input {
                                    class: "bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-600 dark:border-gray-500 dark:placeholder-gray-400 dark:text-white",
                                    required: true,
                                    name: "email",
                                    disabled: "{data.is_loading}",
                                    value: "{data.email_field}"
                                }
                            }
                            div {
                                label {
                                    class: "block mb-2 text-sm font-medium text-gray-900 dark:text-white",
                                    "Password"
                                }
                                input {
                                    class: "bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-600 dark:border-gray-500 dark:placeholder-gray-400 dark:text-white",
                                    required: true,
                                    r#type: "password",
                                    name: "password",
                                    disabled: "{data.is_loading}",
                                    value: "{data.password_field}"
                                }
                            }
                            div {
                                div {
                                    class: "text-red-800 text-sm pb-3",
                                    "{data.error_status}",
                                }
                                button {
                                    r#type: "submit",
                                    class: "w-full text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800",
                                    disabled: "{data.is_loading}",
                                    "Log In",
                                }
                            }
                            div {
                                class: "text-sm font-medium text-gray-500 dark:text-gray-300",
                                "Not registered?"
                                button {
                                    onclick: |_| { data.step.set(SetupStep::Register) },
                                    class: "text-blue-700 hover:underline dark:text-blue-500 pl-2",
                                    disabled: "{data.is_loading}",
                                    "Register"
                                }
                            }
                        }
                    },
                    SetupStep::Register => rsx! {
                        form {
                            onsubmit: move |e| { on_submit_register(cx, data, e) },
                            class: "space-y-6",
                            div {
                                class: "flex items-center justify-between mb-4",
                                h5 {
                                    class: "text-xl font-bold leading-none text-gray-900",
                                    "Create an account"
                                }
                                button {
                                    onclick: |_| { data.step.set(SetupStep::Url) },
                                    class: "text-xl text-gray-500",
                                    disabled: "{data.is_loading}",
                                    i { class: "fa-solid fa-arrow-left mr-3" }
                                    "Back to url"
                                }
                            }
                            div {
                                label {
                                    class: "block mb-2 text-sm font-medium text-gray-900",
                                    "Email"
                                }
                                input {
                                    class: "bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-600 dark:border-gray-500 dark:placeholder-gray-400 dark:text-white",
                                    required: true,
                                    name: "email",
                                    disabled: "{data.is_loading}",
                                }
                            }
                            div {
                                label {
                                    class: "block mb-2 text-sm font-medium text-gray-900",
                                    "Username"
                                }
                                input {
                                    class: "bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-600 dark:border-gray-500 dark:placeholder-gray-400 dark:text-white",
                                    required: true,
                                    name: "username",
                                    disabled: "{data.is_loading}",
                                }
                            }
                            div {
                                label {
                                    class: "block mb-2 text-sm font-medium text-gray-900 dark:text-white",
                                    "Password"
                                }
                                input {
                                    class: "bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-600 dark:border-gray-500 dark:placeholder-gray-400 dark:text-white",
                                    required: true,
                                    r#type: "password",
                                    name: "password",
                                    disabled: "{data.is_loading}",
                                }
                            }
                            div {
                                div {
                                    class: "text-red-800 text-sm pb-3",
                                    "{data.error_status}",
                                }
                                button {
                                    r#type: "submit",
                                    class: "w-full text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800",
                                    disabled: "{data.is_loading}",
                                    "Register",
                                }
                            }
                            div {
                                class: "text-sm font-medium text-gray-500 dark:text-gray-300",
                                button {
                                    onclick: |_| { data.step.set(SetupStep::Login) },
                                    class: "text-blue-700 hover:underline dark:text-blue-500 pl-2",
                                    disabled: "{data.is_loading}",
                                    "Already have an account?"
                                }
                            }
                        }
                    },
                    SetupStep::SyncDir => rsx! {
                        form {
                            onsubmit: move |e| { on_submit_dir(cx, data, e) },
                            class: "space-y-6",
                            div {
                                class: "flex items-center justify-between mb-4",
                                h5 {
                                    class: "text-xl font-bold leading-none text-gray-900",
                                    "Select the directory"
                                }
                                button {
                                    onclick: |_| { data.step.set(SetupStep::Login) },
                                    class: "text-xl text-gray-500",
                                    disabled: "{data.is_loading}",
                                    i { class: "fa-solid fa-arrow-left mr-3" }
                                    "Back to login"
                                }
                            }
                            div {
                                label {
                                    class: "block mb-2 text-sm font-medium text-gray-900",
                                    "Sync Directory"
                                }
                                input {
                                    class: "bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-600 dark:border-gray-500 dark:placeholder-gray-400 dark:text-white",
                                    required: true,
                                    name: "sync_dir",
                                    disabled: "{data.is_loading}",
                                    value: "{data.sync_dir_field}"
                                }
                            }
                            div {
                                div {
                                    class: "text-red-800 text-sm pb-3",
                                    "{data.error_status}",
                                }
                                button {
                                    r#type: "submit",
                                    class: "w-full text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800",
                                    disabled: "{data.is_loading}",
                                    "Continue",
                                }
                            }
                        }
                    },
                }
            }
        }
    })
}

fn on_submit_url(cx: Scope, data: SetupData, event: Event<FormData>) {
    data.is_loading.set(true);
    data.error_status.set("".to_owned());

    let url = event.values.get("url").unwrap().to_owned().parse::<Uri>();

    if let Err(_) = url {
        data.error_status.set("Invalid url syntax".to_owned());
        data.is_loading.set(false);
        return;
    }

    let url = url.unwrap();

    let error_status = data.error_status.clone();
    let is_loading = data.is_loading.clone();
    let step = data.step.clone();
    let api_channel = data.api_channel.clone();

    cx.spawn({
        async move {
            let conf_res = config::modify_conf(|c| {
                c.url = Some(url.to_string());
            })
            .await;

            if let Err(e) = conf_res {
                tracing::error!("failed to modify config {:?}", e);
                error_status.set("Failed to modify the config".to_owned());
                is_loading.set(false);
                return;
            }

            let channel = Channel::builder(url.clone()).connect().await;

            if let Err(e) = channel {
                tracing::error!("failed to connect to server {:?}", e);
                error_status.set("Failed to connect to the server".to_owned());
                is_loading.set(false);
                return;
            }

            let channel = channel.unwrap();
            tracing::info!("Connected to server {:?}", url);

            step.set(SetupStep::Login);
            api_channel.set(Some(channel));
            is_loading.set(false);
        }
    });
}

fn on_submit_login(cx: Scope, data: SetupData, event: Event<FormData>) {
    data.is_loading.set(true);
    data.error_status.set("".to_owned());

    let email = event.values.get("email").unwrap().to_lowercase();
    let password = event.values.get("password").unwrap().to_owned();

    let channel = data.api_channel.as_ref().unwrap().clone();
    let auth_client = AuthApiService::new(channel.clone());

    let error_status = data.error_status.clone();
    let is_loading = data.is_loading.clone();
    let step = data.step.clone();
    let file_api_service = data.file_api_service.clone();

    if let Ok(mut auth_client) = auth_client {
        cx.spawn({
            async move {
                let login_res = auth_client
                    .get_client()
                    .login(proto::AuthLoginRequest {
                        email: email.to_owned(),
                        password: password.to_owned(),
                    })
                    .await
                    .map(|r| r.into_inner());

                match login_res {
                    Ok(login_res) => {
                        let conf_res = config::modify_conf(|c| {
                            c.credentials = Some(config::Credentials { email, password });
                        })
                        .await;

                        if let Err(e) = conf_res {
                            tracing::error!("failed to modify config {:?}", e);
                        }

                        file_api_service.set(Some(Arc::new(Mutex::new(FileApiService::new(
                            channel.clone(),
                            login_res.access_token,
                        )))));

                        step.set(SetupStep::SyncDir);
                    }
                    Err(e) => {
                        error_status.set(e.message().to_owned());
                    }
                }

                is_loading.set(false);
            }
        });
    }
}

fn on_submit_register(cx: Scope, data: SetupData, event: Event<FormData>) {
    data.is_loading.set(true);
    data.error_status.set("".to_owned());

    let email = event.values.get("email").unwrap().to_lowercase();
    let username = event.values.get("username").unwrap().to_owned();
    let password = event.values.get("password").unwrap().to_owned();

    let channel = data.api_channel.as_ref().unwrap().clone();
    let auth_client = AuthApiService::new(channel.clone());

    let error_status = data.error_status.clone();
    let is_loading = data.is_loading.clone();
    let step = data.step.clone();
    let file_api_service = data.file_api_service.clone();

    if let Ok(mut auth_client) = auth_client {
        cx.spawn({
            async move {
                let register_res = auth_client
                    .get_client()
                    .register(proto::AuthRegisterRequest {
                        email: email.to_owned(),
                        username: username.to_owned(),
                        password: password.to_owned(),
                    })
                    .await
                    .map(|r| r.into_inner());

                match register_res {
                    Ok(register_res) => {
                        let conf_res = config::modify_conf(|c| {
                            c.credentials = Some(config::Credentials { email, password });
                        })
                        .await;

                        if let Err(e) = conf_res {
                            tracing::error!("failed to modify config {:?}", e);
                        }

                        file_api_service.set(Some(Arc::new(Mutex::new(FileApiService::new(
                            channel.clone(),
                            register_res.access_token,
                        )))));
                        step.set(SetupStep::SyncDir);
                    }
                    Err(e) => {
                        error_status.set(e.message().to_owned());
                    }
                }

                is_loading.set(false);
            }
        });
    }
}

fn on_submit_dir(cx: Scope, data: SetupData, event: Event<FormData>) {
    data.is_loading.set(true);
    data.error_status.set("".to_owned());

    let dir = event.values.get("sync_dir").unwrap().to_owned();
    let dir_path = Path::new(&dir).to_path_buf();

    if !dir_path.is_absolute() {
        data.error_status.set("The path is not absolute".to_owned());
        data.is_loading.set(false);
        return;
    }

    if !dir_path.exists() {
        data.error_status.set("The path does not exist".to_owned());
        data.is_loading.set(false);
        return;
    }

    if !dir_path.is_dir() {
        data.error_status
            .set("The path is not a directory".to_owned());
        data.is_loading.set(false);
        return;
    }

    let error_status = data.error_status.clone();
    let is_loading = data.is_loading.clone();
    let sync_dir_set = data.sync_dir_set.clone();
    let database_service = data.database_service.clone();
    let router = data.router.clone();

    cx.spawn({
        async move {
            let conf_res =
                config::modify_conf(|c| c.sync_dir = Some(dir_path.to_string_lossy().to_string()))
                    .await;

            if let Err(e) = conf_res {
                tracing::error!("failed to modify config {:?}", e);
                error_status.set("Could not modify the config file".to_owned());
                is_loading.set(false);
                return;
            }

            sync_dir_set.set(Some(dir_path.to_string_lossy().to_string()));

            let db_service = DatabaseService::init(dir_path).await;

            if let Err(e) = &db_service {
                tracing::error!("failed to connect to local db {:?}", e);
            }

            let db_service = db_service.unwrap();
            database_service.set(Some(Arc::new(db_service)));

            router.navigate_to("/files");
        }
    });
}

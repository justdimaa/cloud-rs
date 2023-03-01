use dioxus::prelude::*;

use crate::{
    path_helper::FilePath,
    routes::files::{FilePromptKeep, HandleFileCommand},
};

#[derive(Debug, Clone, PartialEq, Props)]
pub struct FileElementProps {
    pub status: FileStatus,
    pub path: FilePath,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    WaitingQueue,
    WaitingUser(FilePromptKeep),
    Success,
    Failed,
    Added,
    Deleted,
}

pub fn FileElement(cx: Scope<FileElementProps>) -> Element {
    let icon = cx.props.status.to_icon();
    let name = cx
        .props
        .path
        .get_rel()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let dir = cx
        .props
        .path
        .to_rel_str()
        .trim_end_matches(name.as_str())
        .to_owned();
    let size = byte_unit::Byte::from_bytes(cx.props.size.into()).get_appropriate_unit(true);
    let prompt_action = use_coroutine_handle::<HandleFileCommand>(cx).unwrap();

    cx.render(rsx! {
        li {
            class: "py-3 sm:py-4",
            div {
                class: "flex items-center space-x-4",
                div {
                    class: "flex-shrink-0 text-gray-500 text-2xl",
                    i { class: "{icon}" }
                }
                div {
                    class: "flex-1 min-w-0",
                    p {
                        class: "text-sm font-medium text-gray-900 truncate dark:text-white",
                        "{name}"
                    }
                    p {
                        class: "text-sm text-gray-500 truncate dark:text-gray-400",
                        "{dir}"
                    }
                }
                div {
                    class: "inline-flex items-center text-base font-semibold text-gray-900 dark:text-white",
                    "{size}"
                }
            }
            if let FileStatus::WaitingUser(keep) = &cx.props.status {
                rsx! {
                    div {
                        class: "flex items-center space-x-4 py-3 sm:py-4",
                        button {
                            onclick: |_| prompt_action.send(HandleFileCommand::Skip(cx.props.path.clone())),
                            class: "w-full text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800",
                            "Skip",
                        }
                        button {
                            onclick: |_| prompt_action.send(HandleFileCommand::KeepLocal(keep.clone())),
                            class: "w-full text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800",
                            "Keep local",
                        }
                        button {
                            onclick: |_| prompt_action.send(HandleFileCommand::KeepRemote(keep.clone())),
                            class: "w-full text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800",
                            "Keep remote",
                        }
                    }
                }
            }
        }
    })
}

impl FileStatus {
    pub fn to_icon(&self) -> String {
        match *self {
            FileStatus::WaitingQueue => "fa-solid fa-spinner fa-spin",
            FileStatus::WaitingUser(_) => "fa-solid fa-file-circle-question",
            FileStatus::Success => "fa-solid fa-file-circle-check",
            FileStatus::Failed => "fa-solid fa-file-circle-xmark",
            FileStatus::Added => "fa-solid fa-file-circle-plus",
            FileStatus::Deleted => "fa-solid fa-file-circle-minus",
        }
        .to_owned()
    }
}

#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_router::Router;

pub mod components;
pub mod config;
pub mod context;
pub mod global_state;
pub mod path_helper;
pub mod routes;
pub mod services;

fn main() {
    tracing_subscriber::fmt::init();
    dioxus_desktop::launch_cfg(
        App,
        dioxus_desktop::Config::new().with_custom_index(include_str!("../index.html").into()),
    );
}

fn App(cx: Scope) -> Element {
    fermi::use_init_atom_root(&cx);

    cx.render(rsx! {
        Router {
            routes::create_routes(cx)
        }
    })
}

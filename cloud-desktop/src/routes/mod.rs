use dioxus::prelude::*;
use dioxus_router::Route;

pub mod files;
pub mod home;
pub mod setup;

pub fn create_routes(cx: Scope) -> Element {
    cx.render(rsx! {
        Route { to: "/", home::Home {}}
        Route { to: "/setup", rsx! { setup::Setup {} }}
        Route { to: "/files", rsx! { files::Files {} }}
    })
}

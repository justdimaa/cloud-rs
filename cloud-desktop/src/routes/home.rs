use dioxus::prelude::*;
use dioxus_router::Redirect;

pub fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        Redirect { to: "/setup" }
    })
}

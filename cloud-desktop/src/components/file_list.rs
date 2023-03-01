use dioxus::prelude::*;

pub fn FileList(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            class: "w-full max-w-md p-4 bg-white border border-gray-200 rounded-lg shadow sm:p-8 dark:bg-gray-800 dark:border-gray-700",
            div {
                class: "flex items-center justify-between mb-4",
                h5 {
                    class: "text-xl font-bold leading-none text-gray-900 dark:text-white",
                    "Latest Customers"
                }
                a {
                    class: "text-sm font-medium text-blue-600 hover:underline dark:text-blue-500",
                    href: "#",
                    "View all"
                }
            }
            div {
                class: "flow-root",
                ul {
                    role: "list",
                    class: "divide-y divide-gray-200 dark:divide-gray-700"
                }
            }
        }
    })
}

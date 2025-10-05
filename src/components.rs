use leptos::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlElement};

#[component]
pub fn ThemeToggle() -> impl IntoView {
    // Create signal that defaults to light mode on server
    let is_dark = RwSignal::new(false);

    // Client-side initialization - only runs in browser
    #[cfg(feature = "hydrate")]
    {
        let is_dark = is_dark.clone();
        spawn_local(async move {
            // Check local storage first
            if let Some(win) = window() {
                if let Ok(Some(local_storage)) = win.local_storage() {
                    if let Ok(Some(value)) = local_storage.get_item("theme") {
                        let dark = value == "dark";
                        is_dark.set(dark);
                        // Apply theme class immediately
                        if let Some(document) = win.document() {
                            if let Some(html_el) = document
                                .get_elements_by_tag_name("html")
                                .item(0)
                                .and_then(|el| el.dyn_into::<HtmlElement>().ok())
                            {
                                if dark {
                                    let _ = html_el.class_list().add_1("dark");
                                } else {
                                    let _ = html_el.class_list().remove_1("dark");
                                }
                            }
                        }
                        return;
                    }
                }
            }

            // Default to light mode if no local storage value (simpler than match_media for now)
            is_dark.set(false);
            if let Some(document) = window()
                .and_then(|w| w.document())
            {
                if let Some(html_el) = document
                    .get_elements_by_tag_name("html")
                    .item(0)
                    .and_then(|el| el.dyn_into::<HtmlElement>().ok())
                {
                    let _ = html_el.class_list().remove_1("dark");
                }
            }
        });
    }

    // Toggle action - only runs on client
    let toggle_theme = Action::new(move |_: &()| async move {
        #[cfg(feature = "hydrate")]
        {
            let new_dark = !is_dark.get();
            is_dark.set(new_dark);
            let new_theme = if new_dark { "dark" } else { "light" };
            
            // Update local storage
            if let Some(win) = window() {
                if let Ok(Some(local_storage)) = win.local_storage() {
                    let _ = local_storage.set_item("theme", new_theme);
                }
            }
            
            // Update html class
            if let Some(document) = window()
                .and_then(|w| w.document())
            {
                if let Some(html_el) = document
                    .get_elements_by_tag_name("html")
                    .item(0)
                    .and_then(|el| el.dyn_into::<HtmlElement>().ok())
                {
                    if new_dark {
                        let _ = html_el.class_list().add_1("dark");
                    } else {
                        let _ = html_el.class_list().remove_1("dark");
                    }
                }
            }
        }
    });

    view! {
        <button
            on:click=move |_| { toggle_theme.dispatch(()); }
            class="p-2 rounded-lg text-gray-400 hover:text-gray-600 dark:text-gray-300 dark:hover:text-gray-100 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-900"
            aria-label="Toggle dark mode"
        >
            <Show
                fallback=move || view! { 
                    <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"></path>
                    </svg> 
                }
                when=move || is_dark.get()
            >
                <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"></path>
                </svg>
            </Show>
        </button>
    }
}

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ProgrammingLanguage {
    Rust,
    JavaScript,
    Python,
    Java,
    Go,
    Cpp,
    Ruby,
    CSharp,
}

impl ProgrammingLanguage {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProgrammingLanguage::Rust => "Rust",
            ProgrammingLanguage::JavaScript => "JS/TS",
            ProgrammingLanguage::Python => "Python",
            ProgrammingLanguage::Java => "Java",
            ProgrammingLanguage::Go => "Go",
            ProgrammingLanguage::Cpp => "C/C++",
            ProgrammingLanguage::Ruby => "Ruby",
            ProgrammingLanguage::CSharp => "C#",
        }
    }

    pub fn icon_path(&self) -> &'static str {
        match self {
            ProgrammingLanguage::Rust => "/icons/rust.png",
            ProgrammingLanguage::JavaScript => "/icons/javascript.png",
            ProgrammingLanguage::Python => "/icons/python.png",
            ProgrammingLanguage::Java => "/icons/java.png",
            ProgrammingLanguage::Go => "/icons/go.png",
            ProgrammingLanguage::Cpp => "/icons/cpp.png",
            ProgrammingLanguage::Ruby => "/icons/ruby.png",
            ProgrammingLanguage::CSharp => "/icons/csharp.png",
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            ProgrammingLanguage::Rust => "text-blue-800 dark:text-blue-300",
            ProgrammingLanguage::JavaScript => "text-blue-800 dark:text-blue-300",
            ProgrammingLanguage::Python => "text-blue-800 dark:text-blue-300",
            ProgrammingLanguage::Java => "text-blue-800 dark:text-blue-300",
            ProgrammingLanguage::Go => "text-blue-800 dark:text-blue-300",
            ProgrammingLanguage::Cpp => "text-blue-800 dark:text-blue-300",
            ProgrammingLanguage::Ruby => "text-blue-800 dark:text-blue-300",
            ProgrammingLanguage::CSharp => "text-blue-800 dark:text-blue-300",
        }
    }

    pub fn bg_color_class(&self) -> &'static str {
        match self {
            ProgrammingLanguage::Rust => "bg-blue-100 dark:bg-blue-900/20",
            ProgrammingLanguage::JavaScript => "bg-blue-100 dark:bg-blue-900/20",
            ProgrammingLanguage::Python => "bg-blue-100 dark:bg-blue-900/20",
            ProgrammingLanguage::Java => "bg-blue-100 dark:bg-blue-900/20",
            ProgrammingLanguage::Go => "bg-blue-100 dark:bg-blue-900/20",
            ProgrammingLanguage::Cpp => "bg-blue-100 dark:bg-blue-900/20",
            ProgrammingLanguage::Ruby => "bg-blue-100 dark:bg-blue-900/20",
            ProgrammingLanguage::CSharp => "bg-blue-100 dark:bg-blue-900/20",
        }
    }

    pub fn selected_bg_class(&self) -> &'static str {
        match self {
            ProgrammingLanguage::Rust => "bg-blue-600 dark:bg-blue-700",
            ProgrammingLanguage::JavaScript => "bg-blue-600 dark:bg-blue-700",
            ProgrammingLanguage::Python => "bg-blue-600 dark:bg-blue-700",
            ProgrammingLanguage::Java => "bg-blue-600 dark:bg-blue-700",
            ProgrammingLanguage::Go => "bg-blue-600 dark:bg-blue-700",
            ProgrammingLanguage::Cpp => "bg-blue-600 dark:bg-blue-700",
            ProgrammingLanguage::Ruby => "bg-blue-600 dark:bg-blue-700",
            ProgrammingLanguage::CSharp => "bg-blue-600 dark:bg-blue-700",
        }
    }

    pub fn all() -> Vec<ProgrammingLanguage> {
        vec![
            ProgrammingLanguage::Rust,
            ProgrammingLanguage::JavaScript,
            ProgrammingLanguage::Python,
            ProgrammingLanguage::Java,
            ProgrammingLanguage::Go,
            ProgrammingLanguage::Cpp,
            ProgrammingLanguage::Ruby,
            ProgrammingLanguage::CSharp,
        ]
    }
}

impl Default for ProgrammingLanguage {
    fn default() -> Self {
        ProgrammingLanguage::Rust
    }
}

#[component]
pub fn LanguageSelector(
    selected_language: RwSignal<ProgrammingLanguage>,
    disabled: RwSignal<bool>,
) -> impl IntoView {
    // Load from localStorage on mount
    let load_from_storage = move || {
        #[cfg(feature = "hydrate")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(local_storage)) = window.local_storage() {
                    if let Ok(Some(stored)) = local_storage.get_item("selected_programming_language") {
                        if let Ok(lang) = serde_json::from_str::<ProgrammingLanguage>(&stored) {
                            selected_language.set(lang);
                        }
                    }
                }
            }
        }
    };

    // Save to localStorage when language changes
    let save_to_storage = move |_lang: ProgrammingLanguage| {
        #[cfg(feature = "hydrate")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(local_storage)) = window.local_storage() {
                    if let Ok(json) = serde_json::to_string(&_lang) {
                        let _ = local_storage.set_item("selected_programming_language", &json);
                    }
                }
            }
        }
    };

    // Load from storage on mount
    Effect::new(move |_| {
        load_from_storage();
    });

    // Save to storage when language changes
    Effect::new(move |_| {
        let lang = selected_language.get();
        save_to_storage(lang);
    });

    let select_language = move |lang: ProgrammingLanguage| {
        if !disabled.get() {
            selected_language.set(lang);
        }
    };

    view! {
        <div class="flex flex-nowrap justify-center gap-1 p-2 bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-x-auto">
            {move || {
                ProgrammingLanguage::all().into_iter().map(|language| {
                    let language_for_click = language.clone();
                    let language_for_icon = language.clone();
                    let language_for_text = language.clone();
                    let language_for_selection = language.clone();
                    let language_for_class = language.clone();
                    let is_selected = move || selected_language.get() == language_for_selection;
                    
                    view! {
                        <button
                            type="button"
                            on:click=move |_| select_language(language_for_click.clone())
                            disabled=move || disabled.get()
                            class=move || {
                                let base = "flex items-center gap-1.5 px-2.5 py-1.5 rounded-md font-medium text-xs transition-all duration-200 hover:scale-105 focus:outline-none focus:ring-2 focus:ring-offset-1 disabled:opacity-50 disabled:cursor-not-allowed disabled:transform-none whitespace-nowrap";
                                let selected = if is_selected() {
                                    format!("{} text-white shadow-md border-2 border-white/30", language_for_class.selected_bg_class())
                                } else {
                                    format!("{} {} hover:shadow-sm border border-transparent hover:border-gray-300 dark:hover:border-gray-600", language_for_class.bg_color_class(), language_for_class.color_class())
                                };
                                format!("{} {}", base, selected)
                            }
                        >
                            <img 
                                src=language_for_icon.icon_path()
                                alt=language_for_icon.as_str()
                                class="flex-shrink-0 w-6 h-6"
                            />
                            <span class="whitespace-nowrap text-xs">
                                {language_for_text.as_str()}
                            </span>
                        </button>
                    }
                }).collect::<Vec<_>>()
            }}
        </div>
    }
}

use leptos::prelude::*;
use leptos::task::spawn_local;
use super::types::{FileContents, FileContent, ProcessingResult, LoadedFileTypes};

#[server]
pub async fn handle_get_file_contents(file_type: String, file_paths: Vec<String>) -> Result<String, ServerFnError> {
    use crate::api::file_operations::{get_file_contents};
    Ok(get_file_contents(file_type, file_paths).unwrap())
}

pub fn load_file_contents(
    result: RwSignal<Option<ProcessingResult>>,
    file_contents: RwSignal<FileContents>,
    loading_files: RwSignal<bool>,
    loaded_file_types: RwSignal<LoadedFileTypes>,
) {
    if result.get().is_none() {
        return;
    }
    
    let result_data = result.get().unwrap();
    if result_data.file_paths.is_empty() {
        return;
    }
    
    // Get current loaded types to determine what needs loading
    let current_loaded = loaded_file_types.get();
    let file_types = vec!["base", "before", "after", "agent", "main_json", "report"];
    let to_load: Vec<&str> = file_types.iter()
        .filter(|&&ft| !current_loaded.is_loaded(ft))
        .copied()
        .collect();
    
    if to_load.is_empty() {
        loading_files.set(false);
        return;
    }
    
    loading_files.set(true);
    
    spawn_local(async move {
        let mut contents = file_contents.get();
        let mut loaded_types = loaded_file_types.get();
        
        for &file_type in &to_load {
            let _file_type = file_type;
            let content = handle_get_file_contents(file_type.to_string(), result_data.file_paths.clone()).await;
            if let Ok(content) = content {
                let is_json_type = matches!(file_type, "main_json" | "report")
                    || file_type.contains("json");
                let file_content = FileContent {
                    content,
                    file_type: if is_json_type { "json" } else { "text" }.to_string(),
                };
                
                match file_type {
                    "base" => contents.base = Some(file_content),
                    "before" => contents.before = Some(file_content),
                    "after" => contents.after = Some(file_content),
                    "agent" => contents.agent = Some(file_content),
                    "main_json" => contents.main_json = Some(file_content),
                    "report" => contents.report = Some(file_content),
                    _ => {}
                }
                
                loaded_types.set_loaded(file_type);
            }
        }
        
        // Update the signals
        file_contents.set(contents);
        loaded_file_types.set(loaded_types);
        
        // Set loading to false after attempting to load all
        loading_files.set(false);
    });
}

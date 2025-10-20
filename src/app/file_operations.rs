use leptos::prelude::*;
use leptos::task::spawn_local;
use super::types::{FileContents, FileContent, ProcessingResult, LoadedFileTypes};

#[server]
pub async fn handle_get_file_contents(file_type: String, file_paths: Vec<String>) -> Result<String, ServerFnError> {
    use crate::api::file_operations::{get_file_contents};
    get_file_contents(file_type, file_paths)
        .map_err(|e| ServerFnError::ServerError(e))
}

pub fn load_file_contents(
    result: RwSignal<Option<ProcessingResult>>,
    file_contents: RwSignal<FileContents>,
    loading_files: RwSignal<bool>,
    loaded_file_types: RwSignal<LoadedFileTypes>,
    only_load_types: Option<Vec<String>>,
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
    let to_load: Vec<String> = only_load_types.unwrap_or(vec!["base", "before", "after", "agent", "main_json", "report"].into_iter().map(|s| s.to_string()).collect()).iter()
        .filter(|ft| !current_loaded.is_loaded(&ft))
        .map(|s| s.to_string())
        .collect();
    if to_load.is_empty() {
        loading_files.set(false);
        return;
    }
    
    loading_files.set(true);
    
    spawn_local(async move {
        let mut contents = file_contents.get();
        let mut loaded_types = loaded_file_types.get();
        
        for file_type in &to_load {
            let content = handle_get_file_contents(file_type.clone(), result_data.file_paths.clone()).await;
            match content {
                Ok(content) => {
                    // Check if this is a "not found" message for optional files
                    let is_optional = matches!(file_type.as_str(), "agent" | "report");
                    let is_not_found = content.starts_with("No ") && content.contains("file found");
                    
                    if is_optional && is_not_found {
                        // For optional files that are not found, don't create FileContent
                        // Just mark as loaded so we don't keep trying
                        loaded_types.set_loaded(file_type.as_str());
                        continue;
                    }
                    
                    let is_json_type = matches!(file_type.as_str(), "main_json" | "report")
                        || file_type.contains("json");
                    let file_content = FileContent {
                        content,
                        file_type: if is_json_type { "json" } else { "text" }.to_string(),
                    };
                    
                    match file_type.as_str() {
                        "base" => contents.base = Some(file_content),
                        "before" => contents.before = Some(file_content),
                        "after" => contents.after = Some(file_content),
                        "agent" => contents.agent = Some(file_content),
                        "main_json" => contents.main_json = Some(file_content),
                        "report" => contents.report = Some(file_content),
                        _ => {}
                    }
                    
                    loaded_types.set_loaded(file_type.as_str());
                }
                Err(e) => {
                    // Handle error - mark as loaded to prevent infinite retry
                    eprintln!("Failed to load {}: {:?}", file_type, e);
                    loaded_types.set_loaded(file_type.as_str());
                    // For required files, we could optionally store an error message
                }
            }
        }
        
        // Update the signals
        file_contents.set(contents);
        loaded_file_types.set(loaded_types);
        
        // Set loading to false after attempting to load all
        loading_files.set(false);
    });
}

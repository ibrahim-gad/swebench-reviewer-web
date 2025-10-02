use leptos::prelude::*;
use leptos::task::spawn_local;
use super::types::{FileContents, FileContent, ProcessingResult};

pub fn load_file_contents(
    result: RwSignal<Option<ProcessingResult>>,
    file_contents: RwSignal<FileContents>,
    loading_files: RwSignal<bool>,
) {
    if result.get().is_none() {
        return;
    }
    
    let result_data = result.get().unwrap();
    if result_data.file_paths.is_empty() {
        return;
    }
    
    loading_files.set(true);
    
    spawn_local(async move {
        let mut contents = FileContents::default();
        
        // Load each file type
        let _file_types = vec!["base", "before", "after", "agent", "main_json", "report"];
        
        for _file_type in _file_types {
            #[cfg(feature = "hydrate")]
            if let Ok(response) = gloo_net::http::Request::post("/api/get_file_content")
                .json(&serde_json::json!({
                    "file_type": _file_type,
                    "file_paths": result_data.file_paths
                }))
                .unwrap()
                .send()
                .await
            {
                if response.ok() {
                    if let Ok(content) = response.text().await {
                        let is_json_type = matches!(_file_type, "main_json" | "report")
                            || _file_type.contains("json");
                        let file_content = FileContent {
                            content,
                            file_type: if is_json_type { "json" } else { "text" }.to_string(),
                        };
                        
                        match _file_type {
                            "base" => contents.base = Some(file_content),
                            "before" => contents.before = Some(file_content),
                            "after" => contents.after = Some(file_content),
                            "agent" => contents.agent = Some(file_content),
                            "main_json" => contents.main_json = Some(file_content),
                            "report" => contents.report = Some(file_content),
                            _ => {}
                        }
                    }
                }
            }
        }
        
        file_contents.set(contents);
        loading_files.set(false);
    });
}

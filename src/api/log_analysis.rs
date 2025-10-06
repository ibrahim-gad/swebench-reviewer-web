use crate::app::types::{LogAnalysisResult, LogSearchResults, SearchResult};


pub fn search_logs(file_paths: Vec<String>, test_name: String) -> Result<LogSearchResults, String> {
    let base_log = file_paths.iter().find(|path| path.to_lowercase().contains("base.log"));
    let before_log = file_paths.iter().find(|path| path.to_lowercase().contains("before.log"));
    let after_log = file_paths.iter().find(|path| path.to_lowercase().contains("after.log"));
    
    let base_results = if let Some(path) = base_log {
        search_in_log_file(path, &test_name)?
    } else {
        Vec::new()
    };
    
    let before_results = if let Some(path) = before_log {
        search_in_log_file(path, &test_name)?
    } else {
        Vec::new()
    };
    
    let after_results = if let Some(path) = after_log {
        search_in_log_file(path, &test_name)?
    } else {
        Vec::new()
    };
    
    Ok(LogSearchResults {
        base_results,
        before_results,
        after_results,
    })
}

fn search_in_log_file(file_path: &str, test_name: &str) -> Result<Vec<SearchResult>, String> {
    use std::fs;
    
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read log file {}: {}", file_path, e))?;
    
    let lines: Vec<&str> = content.lines().collect();
    let mut results = Vec::new();
    
    let search_terms = get_search_terms(test_name);
    
    for (line_number, line) in lines.iter().enumerate() {
        let mut found_match = false;
        
        for search_term in &search_terms {
            if line.contains(search_term) {
                found_match = true;
                break;
            }
        }
        
        if found_match {
            let context_before: Vec<String> = lines.iter()
                .skip(line_number.saturating_sub(5))
                .take(5.min(line_number))
                .map(|s| s.to_string())
                .collect();
            
            let context_after: Vec<String> = lines.iter()
                .skip(line_number + 1)
                .take(5)
                .map(|s| s.to_string())
                .collect();
            
            results.push(SearchResult {
                line_number: line_number + 1,
                line_content: line.to_string(),
                context_before,
                context_after,
            });
        }
    }
    
    Ok(results)
}

fn get_search_terms(test_name: &str) -> Vec<String> {
    let mut search_terms = vec![test_name.to_string()];
    
    if let Some(last_part) = test_name.split(" - ").last() {
        if last_part != test_name {
            search_terms.push(last_part.to_string());
        }
    }
    
    search_terms.dedup();
    search_terms
}


pub fn analyze_logs(
    file_paths: Vec<String>,
) -> Result<LogAnalysisResult, String> {
    use crate::api::log_parser::LogParser;
    use std::fs;
    
    // Find main.json to get test lists
    let main_json_path = file_paths.iter()
        .find(|path| path.to_lowercase().contains("main.json") || path.to_lowercase().contains("main/"));
    
    let (fail_to_pass_tests, pass_to_pass_tests, language) = if let Some(path) = main_json_path {
        match fs::read_to_string(path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(main_json) => {
                        let fail_to_pass: Vec<String> = main_json.get("fail_to_pass")
                            .and_then(|v| v.as_array())
                            .unwrap_or(&vec![])
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect();
                        
                        let pass_to_pass: Vec<String> = main_json.get("pass_to_pass")
                            .and_then(|v| v.as_array())
                            .unwrap_or(&vec![])
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect();
                        let language = main_json.get("language")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string().to_lowercase())
                            .unwrap_or(String::from("rust"));
                        (fail_to_pass, pass_to_pass, language)
                    },
                    Err(_) => (vec![], vec![], String::from("rust")),
                }
            },
            Err(_) => (vec![], vec![], String::from("rust")),
        }
    } else {
        (vec![], vec![], String::from("rust"))
    };
    
    let log_checker = LogParser::new();
    log_checker.analyze_logs(&file_paths, &language, &fail_to_pass_tests, &pass_to_pass_tests)
}


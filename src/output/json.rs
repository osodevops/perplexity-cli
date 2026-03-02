use crate::api::types::{ChatCompletionResponse, SearchResponse, StreamResult, Usage};

const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Render full API response as JSON, with cli_version injected.
pub fn render_response(response: &ChatCompletionResponse) {
    match serde_json::to_value(response) {
        Ok(mut val) => {
            if let Some(obj) = val.as_object_mut() {
                obj.insert(
                    "cli_version".to_string(),
                    serde_json::Value::String(CLI_VERSION.to_string()),
                );
            }
            match serde_json::to_string_pretty(&val) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("Failed to serialize response: {e}"),
            }
        }
        Err(e) => eprintln!("Failed to serialize response: {e}"),
    }
}

/// Render stream result as JSON.
pub fn render_stream_result(result: &StreamResult) {
    let mut output = serde_json::json!({
        "cli_version": CLI_VERSION,
        "content": result.content,
        "model": result.model,
        "citations": result.citations,
        "search_results": result.search_results,
        "images": result.images,
        "related_questions": result.related_questions,
        "usage": result.usage.as_ref().map(usage_to_json),
    });
    if let Some(ref thinking) = result.thinking_content {
        output["thinking_content"] = serde_json::json!(thinking);
    }
    match serde_json::to_string_pretty(&output) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Failed to serialize response: {e}"),
    }
}

/// Render search response as JSON, with cli_version injected.
pub fn render_search_response(response: &SearchResponse) {
    match serde_json::to_value(response) {
        Ok(mut val) => {
            if let Some(obj) = val.as_object_mut() {
                obj.insert(
                    "cli_version".to_string(),
                    serde_json::Value::String(CLI_VERSION.to_string()),
                );
            }
            match serde_json::to_string_pretty(&val) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("Failed to serialize search response: {e}"),
            }
        }
        Err(e) => eprintln!("Failed to serialize search response: {e}"),
    }
}

fn usage_to_json(usage: &Usage) -> serde_json::Value {
    serde_json::to_value(usage).unwrap_or(serde_json::Value::Null)
}

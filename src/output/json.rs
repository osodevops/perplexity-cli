use crate::api::types::{ChatCompletionResponse, SearchResponse, StreamResult, Usage};

/// Render full API response as JSON.
pub fn render_response(response: &ChatCompletionResponse) {
    match serde_json::to_string_pretty(response) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Failed to serialize response: {e}"),
    }
}

/// Render stream result as JSON.
pub fn render_stream_result(result: &StreamResult) {
    let mut output = serde_json::json!({
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

/// Render search response as JSON.
pub fn render_search_response(response: &SearchResponse) {
    match serde_json::to_string_pretty(response) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Failed to serialize search response: {e}"),
    }
}

fn usage_to_json(usage: &Usage) -> serde_json::Value {
    serde_json::to_value(usage).unwrap_or(serde_json::Value::Null)
}

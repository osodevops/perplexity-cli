use serde::{Deserialize, Serialize};

// ── Request types ──

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_recency_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_after_date_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_before_date_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated_after_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated_before_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_related_questions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_search_classifier: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<JsonSchemaWrapper>,
}

#[derive(Debug, Serialize)]
pub struct JsonSchemaWrapper {
    pub schema: serde_json::Value,
}

// ── Non-streaming response ──

#[derive(Debug, Deserialize, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub created: u64,
    pub choices: Vec<ResponseChoice>,
    pub usage: Option<Usage>,
    pub citations: Option<Vec<String>>,
    pub search_results: Option<Vec<SearchResult>>,
    pub images: Option<Vec<ImageResult>>,
    pub related_questions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseChoice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

// ── Streaming chunk ──

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // fields present for serde deserialization of API responses
pub struct ChatCompletionChunk {
    pub id: String,
    pub model: String,
    pub created: u64,
    pub choices: Vec<ChunkChoice>,
    pub usage: Option<Usage>,
    pub citations: Option<Vec<String>>,
    pub search_results: Option<Vec<SearchResult>>,
    pub images: Option<Vec<ImageResult>>,
    pub related_questions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // fields present for serde deserialization of API responses
pub struct ChunkChoice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // fields present for serde deserialization of API responses
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
}

// ── Shared types ──

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost: Option<Cost>,
    pub search_context_size: Option<String>,
    pub citation_tokens: Option<u32>,
    pub num_search_queries: Option<u32>,
    pub reasoning_tokens: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cost {
    pub input_tokens_cost: Option<f64>,
    pub output_tokens_cost: Option<f64>,
    pub total_cost: Option<f64>,
    pub reasoning_tokens_cost: Option<f64>,
    pub request_cost: Option<f64>,
    pub citation_tokens_cost: Option<f64>,
    pub search_queries_cost: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub date: Option<String>,
    pub last_updated: Option<String>,
    pub snippet: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageResult {
    pub image_url: String,
    pub origin_url: Option<String>,
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Collected result from a streaming session.
#[derive(Debug)]
pub struct StreamResult {
    pub content: String,
    pub thinking_content: Option<String>,
    pub usage: Option<Usage>,
    pub citations: Option<Vec<String>>,
    pub search_results: Option<Vec<SearchResult>>,
    pub images: Option<Vec<ImageResult>>,
    pub related_questions: Option<Vec<String>>,
    pub model: String,
}

// ── Async Research API types ──

/// POST /async/chat/completions request body (same shape as ChatCompletionRequest).
#[derive(Debug, Serialize)]
pub struct AsyncResearchSubmitRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_recency_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_after_date_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_before_date_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<String>,
}

/// POST /async/chat/completions response (job submission).
#[derive(Debug, Deserialize, Serialize)]
pub struct AsyncResearchSubmitResponse {
    pub id: String,
    #[serde(default)]
    pub status: String,
}

/// GET /async/chat/completions/{id} response (job status/result).
#[derive(Debug, Deserialize, Serialize)]
pub struct AsyncResearchStatusResponse {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub response: Option<ChatCompletionResponse>,
}

/// GET /async/chat/completions response (list all jobs).
#[derive(Debug, Deserialize, Serialize)]
pub struct AsyncResearchListResponse {
    #[serde(default)]
    pub items: Vec<AsyncResearchListItem>,
}

/// A single item in the research job list.
#[derive(Debug, Deserialize, Serialize)]
pub struct AsyncResearchListItem {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub created_at: Option<u64>,
}

// ── Agent API types ──

/// POST /responses request body for agent/third-party models.
#[derive(Debug, Serialize)]
pub struct AgentRequest {
    pub model: String,
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AgentTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// A tool specification for the agent API.
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentTool {
    pub r#type: String,
}

/// POST /responses response body.
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentResponse {
    pub id: String,
    pub model: String,
    #[serde(default)]
    pub output: Vec<AgentOutputItem>,
    pub usage: Option<AgentUsage>,
}

/// A single output item from the agent response.
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentOutputItem {
    pub r#type: String,
    #[serde(default)]
    pub content: Option<Vec<AgentContentItem>>,
    pub text: Option<String>,
}

/// Content item within an agent output item.
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentContentItem {
    pub r#type: String,
    pub text: Option<String>,
    #[serde(default)]
    pub annotations: Option<Vec<AgentAnnotation>>,
}

/// An annotation (citation) from the agent response.
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentAnnotation {
    pub r#type: String,
    pub url: Option<String>,
    pub title: Option<String>,
}

/// Usage data from the agent API (field names differ from chat completions).
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

// ── Search API types ──

/// POST /search request body.
#[derive(Debug, Serialize)]
pub struct SearchRequest {
    pub query: SearchQuery,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_recency_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_after_date_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_before_date_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<String>,
}

/// Supports single query (string) or multi-query (array of strings).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SearchQuery {
    Single(String),
    Multi(Vec<String>),
}

/// POST /search response body.
#[derive(Debug, Deserialize, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub id: String,
    pub server_time: Option<String>,
}

/// API error response body.
#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    pub error: Option<ApiErrorDetail>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // fields present for serde deserialization of API responses
pub struct ApiErrorDetail {
    pub message: Option<String>,
    pub r#type: Option<String>,
    pub code: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = ChatCompletionRequest {
            model: "sonar-pro".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: None,
            temperature: Some(0.7),
            top_p: None,
            stream: Some(true),
            search_domain_filter: None,
            search_recency_filter: None,
            search_after_date_filter: None,
            search_before_date_filter: None,
            last_updated_after_filter: None,
            last_updated_before_filter: None,
            return_images: None,
            return_related_questions: None,
            search_mode: None,
            search_context_size: None,
            reasoning_effort: None,
            response_format: None,
            disable_search: None,
            enable_search_classifier: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "sonar-pro");
        assert!(json["temperature"].as_f64().unwrap() - 0.7 < 0.001);
        assert!(json.get("max_tokens").is_none());
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "model": "sonar-pro",
            "created": 1234567890,
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15,
                "cost": {
                    "input_tokens_cost": 0.00003,
                    "output_tokens_cost": 0.000075,
                    "total_cost": 0.005105,
                    "request_cost": 0.005
                }
            },
            "citations": ["https://example.com"]
        }"#;
        let resp: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "chatcmpl-123");
        assert_eq!(resp.choices[0].message.content, "Hello!");
        assert_eq!(resp.citations.as_ref().unwrap().len(), 1);
        let cost = resp.usage.unwrap().cost.unwrap();
        assert_eq!(cost.request_cost, Some(0.005));
    }

    #[test]
    fn test_search_request_serialization() {
        let req = SearchRequest {
            query: SearchQuery::Single("Rust async".to_string()),
            max_results: Some(10),
            max_tokens_per_page: None,
            country: None,
            search_domain_filter: None,
            search_recency_filter: None,
            search_after_date_filter: None,
            search_before_date_filter: None,
            search_mode: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["query"], "Rust async");
        assert_eq!(json["max_results"], 10);
        assert!(json.get("max_tokens_per_page").is_none());
    }

    #[test]
    fn test_search_request_multi_query() {
        let req = SearchRequest {
            query: SearchQuery::Multi(vec!["query one".to_string(), "query two".to_string()]),
            max_results: None,
            max_tokens_per_page: None,
            country: None,
            search_domain_filter: None,
            search_recency_filter: None,
            search_after_date_filter: None,
            search_before_date_filter: None,
            search_mode: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        let queries = json["query"].as_array().unwrap();
        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0], "query one");
    }

    #[test]
    fn test_search_response_deserialization() {
        let json = r#"{
            "id": "search-123",
            "server_time": "2024-01-15T10:30:00Z",
            "results": [{
                "title": "Rust Programming",
                "url": "https://www.rust-lang.org",
                "date": "2024-01-01",
                "snippet": "A language empowering everyone",
                "source": "web"
            }]
        }"#;
        let resp: SearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "search-123");
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0].title, "Rust Programming");
    }

    #[test]
    fn test_chunk_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "model": "sonar-pro",
            "created": 1234567890,
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }]
        }"#;
        let chunk: ChatCompletionChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
    }
}

use assert_cmd::Command;
use predicates::prelude::*;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn pplx_cmd() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("pplx").unwrap()
}

// ── CLI help and basics ──

#[test]
fn test_no_args_shows_help() {
    pplx_cmd()
        .assert()
        .success()
        .stdout(predicate::str::contains("A powerful Perplexity API client"));
}

#[test]
fn test_help_flag() {
    pplx_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("ask"))
        .stdout(predicate::str::contains("completions"));
}

#[test]
fn test_version_flag() {
    pplx_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("pplx"));
}

#[test]
fn test_completions_zsh() {
    pplx_cmd()
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_pplx"));
}

#[test]
fn test_completions_bash() {
    pplx_cmd()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pplx"));
}

// ── Missing API key errors ──

#[test]
fn test_ask_without_api_key() {
    pplx_cmd()
        .args(["ask", "--config", "/dev/null", "test query"])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key not found"));
}

// ── Mock API tests ──

#[tokio::test]
async fn test_api_client_non_streaming() {
    let mock_server = MockServer::start().await;

    let response_body = include_str!("fixtures/chat_response.json");
    let response: serde_json::Value = serde_json::from_str(response_body).unwrap();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer test-key-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let request = pplx::api::types::ChatCompletionRequest {
        model: "sonar-pro".to_string(),
        messages: vec![pplx::api::types::Message {
            role: "user".to_string(),
            content: "What is Rust?".to_string(),
        }],
        max_tokens: None,
        temperature: None,
        top_p: None,
        stream: Some(false),
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

    let resp = client.chat_completion(&request).await.unwrap();
    assert_eq!(resp.model, "sonar-pro");
    assert_eq!(
        resp.choices[0].message.content,
        "Rust is a systems programming language focused on safety, speed, and concurrency."
    );
    assert_eq!(resp.citations.as_ref().unwrap().len(), 2);
}

#[tokio::test]
async fn test_api_client_auth_error() {
    let mock_server = MockServer::start().await;

    let error_body = include_str!("fixtures/error_401.json");
    let error: serde_json::Value = serde_json::from_str(error_body).unwrap();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("bad-key", &mock_server.uri()).unwrap();

    let request = pplx::api::types::ChatCompletionRequest {
        model: "sonar-pro".to_string(),
        messages: vec![pplx::api::types::Message {
            role: "user".to_string(),
            content: "test".to_string(),
        }],
        max_tokens: None,
        temperature: None,
        top_p: None,
        stream: Some(false),
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

    let err = client.chat_completion(&request).await.unwrap_err();
    assert!(err.to_string().contains("Invalid API key"));
}

#[tokio::test]
async fn test_api_client_streaming() {
    let mock_server = MockServer::start().await;

    let sse_body = include_str!("fixtures/chat_stream.txt");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let request = pplx::api::types::ChatCompletionRequest {
        model: "sonar-pro".to_string(),
        messages: vec![pplx::api::types::Message {
            role: "user".to_string(),
            content: "What is Rust?".to_string(),
        }],
        max_tokens: None,
        temperature: None,
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

    let mut tokens = Vec::new();
    let result = client
        .chat_completion_stream(
            &request,
            |token| {
                tokens.push(token.to_string());
            },
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.content, "Rust is a systems programming language.");
    assert_eq!(result.model, "sonar-pro");
    assert!(result.citations.is_some());
    assert!(result.usage.is_some());
    assert!(tokens.len() >= 5);
}

// ── Search command tests ──

#[test]
fn test_search_without_api_key() {
    pplx_cmd()
        .args(["search", "--config", "/dev/null", "test query"])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key not found"));
}

#[tokio::test]
async fn test_api_search_basic() {
    let mock_server = MockServer::start().await;

    let response_body = include_str!("fixtures/search_response.json");
    let response: serde_json::Value = serde_json::from_str(response_body).unwrap();

    Mock::given(method("POST"))
        .and(path("/search"))
        .and(header("authorization", "Bearer test-key-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let request = pplx::api::types::SearchRequest {
        query: pplx::api::types::SearchQuery::Single("Rust programming".to_string()),
        max_results: Some(10),
        max_tokens_per_page: None,
        country: None,
        search_domain_filter: None,
        search_recency_filter: None,
        search_after_date_filter: None,
        search_before_date_filter: None,
        search_mode: None,
    };

    let resp = client.search(&request).await.unwrap();
    assert_eq!(resp.id, "search-abc123");
    assert_eq!(resp.results.len(), 3);
    assert_eq!(resp.results[0].title, "Rust Programming Language");
    assert_eq!(resp.results[0].url, "https://www.rust-lang.org");
}

#[tokio::test]
async fn test_api_search_multi_query() {
    let mock_server = MockServer::start().await;

    let response_body = include_str!("fixtures/search_response.json");
    let response: serde_json::Value = serde_json::from_str(response_body).unwrap();

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let request = pplx::api::types::SearchRequest {
        query: pplx::api::types::SearchQuery::Multi(vec![
            "Rust async".to_string(),
            "Rust concurrency".to_string(),
        ]),
        max_results: None,
        max_tokens_per_page: None,
        country: None,
        search_domain_filter: None,
        search_recency_filter: None,
        search_after_date_filter: None,
        search_before_date_filter: None,
        search_mode: None,
    };

    let resp = client.search(&request).await.unwrap();
    assert_eq!(resp.results.len(), 3);
}

#[tokio::test]
async fn test_api_search_auth_error() {
    let mock_server = MockServer::start().await;

    let error_body = include_str!("fixtures/error_401.json");
    let error: serde_json::Value = serde_json::from_str(error_body).unwrap();

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("bad-key", &mock_server.uri()).unwrap();

    let request = pplx::api::types::SearchRequest {
        query: pplx::api::types::SearchQuery::Single("test".to_string()),
        max_results: None,
        max_tokens_per_page: None,
        country: None,
        search_domain_filter: None,
        search_recency_filter: None,
        search_after_date_filter: None,
        search_before_date_filter: None,
        search_mode: None,
    };

    let err = client.search(&request).await.unwrap_err();
    assert!(err.to_string().contains("Invalid API key"));
}

#[test]
fn test_json_schema_flag_parsed() {
    // Verify --json-schema flag is accepted by the CLI parser
    pplx_cmd()
        .args([
            "ask",
            "--config",
            "/dev/null",
            "--json-schema",
            r#"{"type":"object"}"#,
            "test query",
        ])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key not found"));
}

#[test]
fn test_save_flag_parsed() {
    // Verify --save flag is accepted by the CLI parser
    pplx_cmd()
        .args([
            "ask",
            "--config",
            "/dev/null",
            "--save",
            "/tmp/test.md",
            "test query",
        ])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key not found"));
}

// ── Stdin pipe test ──

#[test]
fn test_stdin_pipe_without_api_key() {
    pplx_cmd()
        .args(["ask", "--config", "/dev/null"])
        .write_stdin("What is Rust?")
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key not found"));
}

// ── Phase 3: Think block streaming test ──

#[tokio::test]
async fn test_streaming_with_think_blocks() {
    let mock_server = MockServer::start().await;

    let sse_body = include_str!("fixtures/chat_stream_reasoning.txt");

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let request = pplx::api::types::ChatCompletionRequest {
        model: "sonar-reasoning-pro".to_string(),
        messages: vec![pplx::api::types::Message {
            role: "user".to_string(),
            content: "test".to_string(),
        }],
        max_tokens: None,
        temperature: None,
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

    let mut think_tokens = Vec::new();
    let mut normal_tokens = Vec::new();

    let mut think_handler = |token: &str| {
        think_tokens.push(token.to_string());
    };

    let result = client
        .chat_completion_stream(
            &request,
            |token| {
                normal_tokens.push(token.to_string());
            },
            Some(&mut think_handler as &mut dyn FnMut(&str)),
        )
        .await
        .unwrap();

    // Content should be clean (no think tags)
    assert_eq!(result.content, "The answer is 42.");
    // Thinking content should be captured
    assert_eq!(
        result.thinking_content,
        Some("Let me reason about this carefully.".to_string())
    );
    assert_eq!(result.model, "sonar-reasoning-pro");
    assert!(result.citations.is_some());
    assert!(result.usage.is_some());

    // Normal tokens should not contain think block content
    let normal_text: String = normal_tokens.concat();
    assert_eq!(normal_text, "The answer is 42.");

    // Think tokens should contain the reasoning
    let think_text: String = think_tokens.concat();
    assert_eq!(think_text, "Let me reason about this carefully.");
}

// ── Phase 3: Research API tests ──

#[tokio::test]
async fn test_research_submit() {
    let mock_server = MockServer::start().await;

    let response_body = include_str!("fixtures/research_submit_response.json");
    let response: serde_json::Value = serde_json::from_str(response_body).unwrap();

    Mock::given(method("POST"))
        .and(path("/async/chat/completions"))
        .and(header("authorization", "Bearer test-key-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let request = pplx::api::types::AsyncResearchSubmitRequest {
        model: "sonar-deep-research".to_string(),
        messages: vec![pplx::api::types::Message {
            role: "user".to_string(),
            content: "Analyze the Rust ecosystem".to_string(),
        }],
        max_tokens: None,
        temperature: None,
        search_domain_filter: None,
        search_recency_filter: None,
        search_after_date_filter: None,
        search_before_date_filter: None,
        search_mode: None,
        search_context_size: None,
    };

    let resp = client.research_submit(&request).await.unwrap();
    assert_eq!(resp.id, "research-job-abc123");
    assert_eq!(resp.status, "pending");
}

#[tokio::test]
async fn test_research_status() {
    let mock_server = MockServer::start().await;

    let response_body = include_str!("fixtures/research_status_complete.json");
    let response: serde_json::Value = serde_json::from_str(response_body).unwrap();

    Mock::given(method("GET"))
        .and(path("/async/chat/completions/research-job-abc123"))
        .and(header("authorization", "Bearer test-key-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let resp = client.research_status("research-job-abc123").await.unwrap();
    assert_eq!(resp.id, "research-job-abc123");
    assert_eq!(resp.status, "completed");
    assert!(resp.response.is_some());
    let inner = resp.response.unwrap();
    assert_eq!(inner.model, "sonar-deep-research");
    assert_eq!(
        inner.choices[0].message.content,
        "Here is the comprehensive research analysis of the Rust ecosystem."
    );
}

#[tokio::test]
async fn test_research_list() {
    let mock_server = MockServer::start().await;

    let response_body = include_str!("fixtures/research_list_response.json");
    let response: serde_json::Value = serde_json::from_str(response_body).unwrap();

    Mock::given(method("GET"))
        .and(path("/async/chat/completions"))
        .and(header("authorization", "Bearer test-key-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let resp = client.research_list().await.unwrap();
    assert_eq!(resp.items.len(), 2);
    assert_eq!(resp.items[0].id, "research-job-abc123");
    assert_eq!(resp.items[0].status, "completed");
    assert_eq!(resp.items[1].id, "research-job-def456");
    assert_eq!(resp.items[1].status, "pending");
}

#[test]
fn test_research_without_api_key() {
    pplx_cmd()
        .args(["research", "--config", "/dev/null", "test query"])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key not found"));
}

// ── Phase 3: Agent API tests ──

#[tokio::test]
async fn test_agent_basic() {
    let mock_server = MockServer::start().await;

    let response_body = include_str!("fixtures/agent_response.json");
    let response: serde_json::Value = serde_json::from_str(response_body).unwrap();

    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(header("authorization", "Bearer test-key-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let request = pplx::api::types::AgentRequest {
        model: "openai/gpt-4o".to_string(),
        input: "Latest AI news".to_string(),
        tools: None,
        instructions: None,
        max_tokens: None,
        temperature: None,
        stream: Some(false),
    };

    let resp = client.agent_completion(&request).await.unwrap();
    assert_eq!(resp.id, "resp-agent-123");
    assert_eq!(resp.model, "openai/gpt-4o");
    assert!(!resp.output.is_empty());

    // Extract text using the output helper
    let text = pplx::output::extract_agent_text(&resp);
    assert_eq!(text, "Here is the latest AI news summary.");

    // Extract citations
    let citations = pplx::output::extract_agent_citations(&resp);
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0], "https://example.com/ai-news");

    // Check usage
    let usage = resp.usage.unwrap();
    assert_eq!(usage.input_tokens, 50);
    assert_eq!(usage.output_tokens, 30);
    assert_eq!(usage.total_tokens, 80);
}

#[tokio::test]
async fn test_agent_with_tools() {
    let mock_server = MockServer::start().await;

    let response_body = include_str!("fixtures/agent_response.json");
    let response: serde_json::Value = serde_json::from_str(response_body).unwrap();

    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("test-key-123", &mock_server.uri()).unwrap();

    let request = pplx::api::types::AgentRequest {
        model: "openai/gpt-4o".to_string(),
        input: "Search for AI news".to_string(),
        tools: Some(vec![
            pplx::api::types::AgentTool {
                r#type: "web_search".to_string(),
            },
            pplx::api::types::AgentTool {
                r#type: "fetch_url".to_string(),
            },
        ]),
        instructions: Some("Be concise".to_string()),
        max_tokens: None,
        temperature: None,
        stream: Some(false),
    };

    // Verify request serialization includes tools
    let json = serde_json::to_value(&request).unwrap();
    let tools = json["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0]["type"], "web_search");
    assert_eq!(tools[1]["type"], "fetch_url");

    let resp = client.agent_completion(&request).await.unwrap();
    assert_eq!(resp.id, "resp-agent-123");
}

#[tokio::test]
async fn test_agent_auth_error() {
    let mock_server = MockServer::start().await;

    let error_body = include_str!("fixtures/error_401.json");
    let error: serde_json::Value = serde_json::from_str(error_body).unwrap();

    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&error))
        .mount(&mock_server)
        .await;

    let client =
        pplx::api::client::ApiClient::with_base_url("bad-key", &mock_server.uri()).unwrap();

    let request = pplx::api::types::AgentRequest {
        model: "openai/gpt-4o".to_string(),
        input: "test".to_string(),
        tools: None,
        instructions: None,
        max_tokens: None,
        temperature: None,
        stream: Some(false),
    };

    let err = client.agent_completion(&request).await.unwrap_err();
    assert!(err.to_string().contains("Invalid API key"));
}

#[test]
fn test_agent_without_api_key() {
    pplx_cmd()
        .args(["agent", "--config", "/dev/null", "test query"])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key not found"));
}

// ── Phase 3: Reasoning flag test ──

#[test]
fn test_reasoning_flag_parsed() {
    pplx_cmd()
        .args(["ask", "--config", "/dev/null", "--reasoning", "test query"])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key not found"));
}

// ── Phase 3: Interactive exit test ──

#[test]
fn test_interactive_exits_on_eof() {
    pplx_cmd()
        .args(["interactive"])
        .env("PERPLEXITY_API_KEY", "test-key-123")
        .write_stdin("")
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success();
}

// ── Phase 4: Validation tests ──

#[test]
fn test_invalid_temperature() {
    pplx_cmd()
        .args(["ask", "--temperature", "3.0", "test"])
        .env("PERPLEXITY_API_KEY", "test-key-123")
        .assert()
        .failure()
        .stderr(predicate::str::contains("temperature"));
}

#[test]
fn test_invalid_top_p() {
    pplx_cmd()
        .args(["ask", "--top-p", "1.5", "test"])
        .env("PERPLEXITY_API_KEY", "test-key-123")
        .assert()
        .failure()
        .stderr(predicate::str::contains("top_p"));
}

#[test]
fn test_version_contains_hash() {
    pplx_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.3.0"))
        .stdout(predicate::str::contains("("));
}

#[test]
fn test_config_set_invalid_key() {
    pplx_cmd()
        .args(["config", "set", "nonexistent", "value"])
        .env("PERPLEXITY_API_KEY", "test-key-123")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown config key"));
}

#[test]
fn test_config_set_invalid_output() {
    pplx_cmd()
        .args(["config", "set", "output", "xml"])
        .env("PERPLEXITY_API_KEY", "test-key-123")
        .assert()
        .failure()
        .stderr(predicate::str::contains("output"));
}

// ── Agent-friendliness tests ──

#[test]
fn test_auth_error_exit_code_3() {
    pplx_cmd()
        .args(["ask", "--config", "/dev/null", "test"])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .code(3);
}

#[test]
fn test_validation_error_exit_code_2() {
    pplx_cmd()
        .args(["ask", "--temperature", "3.0", "test"])
        .env("PERPLEXITY_API_KEY", "test-key-123")
        .assert()
        .code(2);
}

#[test]
fn test_json_error_on_auth_failure() {
    let output = pplx_cmd()
        .args(["ask", "--config", "/dev/null", "-o", "json", "test"])
        .env_remove("PERPLEXITY_API_KEY")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["error"]["code"], "auth_failed");
    assert_eq!(json["error"]["exit_code"], 3);
}

#[test]
fn test_json_error_on_validation() {
    let output = pplx_cmd()
        .args(["ask", "-o", "json", "--temperature", "5.0", "test"])
        .env("PERPLEXITY_API_KEY", "test-key-123")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["error"]["code"], "validation_error");
    assert_eq!(json["error"]["exit_code"], 2);
}

#[test]
fn test_no_json_error_in_plain_mode() {
    let output = pplx_cmd()
        .args(["ask", "--config", "/dev/null", "-o", "plain", "test"])
        .env_remove("PERPLEXITY_API_KEY")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // In plain mode, no JSON error should appear on stdout
    assert!(stdout.is_empty());
}

#[test]
fn test_no_spinner_flag_accepted() {
    pplx_cmd()
        .args(["ask", "--config", "/dev/null", "--no-spinner", "test"])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure();
}

#[test]
fn test_quiet_flag_accepted() {
    pplx_cmd()
        .args(["ask", "--config", "/dev/null", "-q", "test"])
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure();
}

#[test]
fn test_describe_outputs_valid_json() {
    let output = pplx_cmd().args(["describe"]).output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["commands"].is_object());
    assert!(json["exit_codes"].is_object());
    assert!(json["env_vars"].is_object());
    assert_eq!(json["name"], "pplx");
}

#[test]
fn test_subcommand_help_has_examples() {
    pplx_cmd()
        .args(["ask", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Examples:"));
}

#[test]
fn test_research_dry_run() {
    let output = pplx_cmd()
        .args(["research", "--dry-run", "test query"])
        .env("PERPLEXITY_API_KEY", "test-key-123")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["model"], "sonar-deep-research");
    assert!(json["messages"].is_array());
}

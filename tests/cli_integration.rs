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
        .args(["ask", "test query"])
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(sse_body, "text/event-stream"),
        )
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
        .chat_completion_stream(&request, |token| {
            tokens.push(token.to_string());
        })
        .await
        .unwrap();

    assert_eq!(result.content, "Rust is a systems programming language.");
    assert_eq!(result.model, "sonar-pro");
    assert!(result.citations.is_some());
    assert!(result.usage.is_some());
    assert!(tokens.len() >= 5);
}

// ── Stdin pipe test ──

#[test]
fn test_stdin_pipe_without_api_key() {
    pplx_cmd()
        .arg("ask")
        .write_stdin("What is Rust?")
        .env_remove("PERPLEXITY_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("API key not found"));
}

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::error::PplxError;

const DEFAULT_BASE_URL: &str = "https://api.perplexity.ai";
const DEFAULT_TIMEOUT_SECS: u64 = 60;
const MAX_RETRIES: u32 = 3;
const RETRY_BASE_MS: u64 = 500;

pub struct ApiClient {
    pub(crate) client: reqwest::Client,
    pub(crate) base_url: String,
    #[allow(dead_code)]
    pub(crate) api_key: String,
}

impl ApiClient {
    pub fn new(api_key: &str) -> Result<Self, PplxError> {
        Self::with_base_url(api_key, DEFAULT_BASE_URL)
    }

    pub fn with_base_url(api_key: &str, base_url: &str) -> Result<Self, PplxError> {
        if api_key.is_empty() {
            return Err(PplxError::Auth(
                "API key not found. Set PERPLEXITY_API_KEY or add it to config.toml".to_string(),
            ));
        }

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(|e| {
                PplxError::Auth(format!("Invalid API key format: {e}"))
            })?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        })
    }

    pub fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

/// Execute a closure with exponential backoff retry on retryable errors.
pub async fn with_retry<F, Fut, T>(f: F) -> Result<T, PplxError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, PplxError>>,
{
    let mut last_err = None;
    for attempt in 0..MAX_RETRIES {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                let should_retry = matches!(&e, PplxError::RateLimited { .. })
                    || matches!(&e, PplxError::Api { status, .. } if *status >= 500);

                if !should_retry || attempt == MAX_RETRIES - 1 {
                    return Err(e);
                }

                let delay = match &e {
                    PplxError::RateLimited {
                        retry_after_secs: Some(secs),
                    } => Duration::from_secs(*secs),
                    _ => Duration::from_millis(RETRY_BASE_MS * 2u64.pow(attempt)),
                };

                tracing::debug!("Retry attempt {} after {:?}: {}", attempt + 1, delay, e);
                tokio::time::sleep(delay).await;
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap())
}

use crate::error::PplxError;

use super::client::ApiClient;
use super::types::{ApiErrorResponse, ChatCompletionRequest, ChatCompletionResponse};

impl ApiClient {
    /// Send a non-streaming chat completion request.
    pub async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, PplxError> {
        let url = self.endpoint("/chat/completions");

        let response = self.client.post(&url).json(request).send().await?;

        let status = response.status().as_u16();

        if status == 401 {
            return Err(PplxError::Auth(
                "Invalid API key. Check your key at https://www.perplexity.ai/settings/api"
                    .to_string(),
            ));
        }

        if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            return Err(PplxError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if status >= 400 {
            let body = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiErrorResponse>(&body)
                .ok()
                .and_then(|e| e.error)
                .and_then(|e| e.message)
                .unwrap_or(body);
            return Err(PplxError::Api { status, message });
        }

        let resp = response.json::<ChatCompletionResponse>().await?;
        Ok(resp)
    }
}

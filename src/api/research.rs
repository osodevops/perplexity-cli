use crate::error::PplxError;

use super::client::ApiClient;
use super::types::{
    ApiErrorResponse, AsyncResearchListResponse, AsyncResearchStatusResponse,
    AsyncResearchSubmitRequest, AsyncResearchSubmitResponse,
};

impl ApiClient {
    /// Submit an async research job via POST /async/chat/completions.
    pub async fn research_submit(
        &self,
        request: &AsyncResearchSubmitRequest,
    ) -> Result<AsyncResearchSubmitResponse, PplxError> {
        let url = self.endpoint("/async/chat/completions");
        let response = self.client.post(&url).json(request).send().await?;
        handle_error_status(&response.status(), response.headers())?;
        let status = response.status().as_u16();
        if status >= 400 {
            let body = response.text().await.unwrap_or_default();
            let message = parse_error_message(&body);
            return Err(PplxError::Api { status, message });
        }
        let resp = response.json::<AsyncResearchSubmitResponse>().await?;
        Ok(resp)
    }

    /// Get the status/result of a research job via GET /async/chat/completions/{id}.
    pub async fn research_status(
        &self,
        id: &str,
    ) -> Result<AsyncResearchStatusResponse, PplxError> {
        let url = self.endpoint(&format!("/async/chat/completions/{id}"));
        let response = self.client.get(&url).send().await?;
        handle_error_status(&response.status(), response.headers())?;
        let status = response.status().as_u16();
        if status >= 400 {
            let body = response.text().await.unwrap_or_default();
            let message = parse_error_message(&body);
            return Err(PplxError::Api { status, message });
        }
        let resp = response.json::<AsyncResearchStatusResponse>().await?;
        Ok(resp)
    }

    /// List all research jobs via GET /async/chat/completions.
    pub async fn research_list(&self) -> Result<AsyncResearchListResponse, PplxError> {
        let url = self.endpoint("/async/chat/completions");
        let response = self.client.get(&url).send().await?;
        handle_error_status(&response.status(), response.headers())?;
        let status = response.status().as_u16();
        if status >= 400 {
            let body = response.text().await.unwrap_or_default();
            let message = parse_error_message(&body);
            return Err(PplxError::Api { status, message });
        }
        let resp = response.json::<AsyncResearchListResponse>().await?;
        Ok(resp)
    }
}

fn handle_error_status(
    status: &reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
) -> Result<(), PplxError> {
    let code = status.as_u16();
    if code == 401 {
        return Err(PplxError::Auth(
            "Invalid API key. Check your key at https://www.perplexity.ai/settings/api".to_string(),
        ));
    }
    if code == 429 {
        let retry_after = headers
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        return Err(PplxError::RateLimited {
            retry_after_secs: retry_after,
        });
    }
    Ok(())
}

fn parse_error_message(body: &str) -> String {
    serde_json::from_str::<ApiErrorResponse>(body)
        .ok()
        .and_then(|e| e.error)
        .and_then(|e| e.message)
        .unwrap_or_else(|| body.to_string())
}

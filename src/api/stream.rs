use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};

use crate::error::PplxError;

use super::client::ApiClient;
use super::types::{ChatCompletionChunk, ChatCompletionRequest, StreamResult};

impl ApiClient {
    /// Send a streaming chat completion request.
    /// Calls `on_token` for each content delta as it arrives.
    pub async fn chat_completion_stream<F>(
        &self,
        request: &ChatCompletionRequest,
        mut on_token: F,
    ) -> Result<StreamResult, PplxError>
    where
        F: FnMut(&str),
    {
        let url = self.endpoint("/chat/completions");

        let req = self.client.post(&url).json(request);
        let mut es = EventSource::new(req).map_err(|e| PplxError::Stream(e.to_string()))?;

        let mut content = String::new();
        let mut usage = None;
        let mut citations = None;
        let mut search_results = None;
        let mut images = None;
        let mut related_questions = None;
        let mut model = String::new();

        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => {}
                Ok(Event::Message(msg)) => {
                    if msg.data == "[DONE]" {
                        break;
                    }

                    let chunk: ChatCompletionChunk = serde_json::from_str(&msg.data)
                        .map_err(|e| {
                            PplxError::Stream(format!("Failed to parse chunk: {e}"))
                        })?;

                    if model.is_empty() {
                        model = chunk.model.clone();
                    }

                    for choice in &chunk.choices {
                        if let Some(ref text) = choice.delta.content {
                            on_token(text);
                            content.push_str(text);
                        }
                    }

                    // Collect metadata from chunks (usually arrives in final chunks)
                    if chunk.usage.is_some() {
                        usage = chunk.usage;
                    }
                    if chunk.citations.is_some() {
                        citations = chunk.citations;
                    }
                    if chunk.search_results.is_some() {
                        search_results = chunk.search_results;
                    }
                    if chunk.images.is_some() {
                        images = chunk.images;
                    }
                    if chunk.related_questions.is_some() {
                        related_questions = chunk.related_questions;
                    }
                }
                Err(reqwest_eventsource::Error::StreamEnded) => break,
                Err(reqwest_eventsource::Error::InvalidStatusCode(status_code, response)) => {
                    let status = status_code.as_u16();
                    if status == 401 {
                        es.close();
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
                        es.close();
                        return Err(PplxError::RateLimited {
                            retry_after_secs: retry_after,
                        });
                    }
                    let body = response.text().await.unwrap_or_default();
                    es.close();
                    return Err(PplxError::Api {
                        status,
                        message: body,
                    });
                }
                Err(e) => {
                    es.close();
                    return Err(PplxError::Stream(e.to_string()));
                }
            }
        }

        es.close();

        Ok(StreamResult {
            content,
            usage,
            citations,
            search_results,
            images,
            related_questions,
            model,
        })
    }
}

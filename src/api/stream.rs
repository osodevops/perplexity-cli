use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};

use crate::error::PplxError;

use super::client::ApiClient;
use super::think::{ThinkEvent, ThinkParser};
use super::types::{ChatCompletionChunk, ChatCompletionRequest, StreamResult};

impl ApiClient {
    /// Send a streaming chat completion request.
    /// Calls `on_token` for each content delta as it arrives.
    /// Optionally calls `on_think_token` for content inside `<think>` blocks.
    pub async fn chat_completion_stream<F>(
        &self,
        request: &ChatCompletionRequest,
        mut on_token: F,
        mut on_think_token: Option<&mut dyn FnMut(&str)>,
    ) -> Result<StreamResult, PplxError>
    where
        F: FnMut(&str),
    {
        let url = self.endpoint("/chat/completions");

        let req = self.client.post(&url).json(request);
        let mut es = EventSource::new(req).map_err(|e| PplxError::Stream(e.to_string()))?;

        let mut content = String::new();
        let mut thinking_content = String::new();
        let mut usage = None;
        let mut citations = None;
        let mut search_results = None;
        let mut images = None;
        let mut related_questions = None;
        let mut model = String::new();

        let mut think_parser = ThinkParser::new();
        let use_think_parser = on_think_token.is_some();

        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => {}
                Ok(Event::Message(msg)) => {
                    if msg.data == "[DONE]" {
                        break;
                    }

                    let chunk: ChatCompletionChunk = serde_json::from_str(&msg.data)
                        .map_err(|e| PplxError::Stream(format!("Failed to parse chunk: {e}")))?;

                    if model.is_empty() {
                        model = chunk.model.clone();
                    }

                    for choice in &chunk.choices {
                        if let Some(ref text) = choice.delta.content {
                            if use_think_parser {
                                for event in think_parser.feed(text) {
                                    match event {
                                        ThinkEvent::Normal(s) => {
                                            on_token(&s);
                                            content.push_str(&s);
                                        }
                                        ThinkEvent::Think(s) => {
                                            if let Some(ref mut handler) = on_think_token {
                                                handler(&s);
                                            }
                                            thinking_content.push_str(&s);
                                        }
                                    }
                                }
                            } else {
                                on_token(text);
                                content.push_str(text);
                            }
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

        // Flush any remaining buffered content from the think parser
        if use_think_parser {
            for event in think_parser.flush() {
                match event {
                    ThinkEvent::Normal(s) => content.push_str(&s),
                    ThinkEvent::Think(s) => thinking_content.push_str(&s),
                }
            }
        }

        es.close();

        Ok(StreamResult {
            content,
            thinking_content: if thinking_content.is_empty() {
                None
            } else {
                Some(thinking_content)
            },
            usage,
            citations,
            search_results,
            images,
            related_questions,
            model,
        })
    }
}

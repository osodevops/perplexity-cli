pub mod commands;
pub mod history;

use anyhow::{Context, Result};

use crate::api::client::ApiClient;
use crate::api::types::{ChatCompletionRequest, Message};
use crate::config::types::ResolvedConfig;
use crate::cost::tracker::CostTracker;
use crate::output;
use crate::output::RenderFinalOpts;

use commands::{handle_command, CommandResult};

/// Mutable session configuration that slash commands can modify at runtime.
pub struct SessionConfig {
    pub model: String,
    pub system_prompt: Option<String>,
    pub output_format: String,
    pub show_reasoning: bool,
    pub search_mode: Option<String>,
    pub search_recency: Option<String>,
    pub search_domains: Vec<String>,
    pub search_exclude_domains: Vec<String>,
    pub search_context_size: Option<String>,
    pub no_color: bool,
}

impl SessionConfig {
    fn from_resolved(config: &ResolvedConfig) -> Self {
        Self {
            model: config.model.clone(),
            system_prompt: config.system_prompt.clone(),
            output_format: config.output_format.clone(),
            show_reasoning: config.show_reasoning,
            search_mode: config.search_mode.clone(),
            search_recency: config.search_recency.clone(),
            search_domains: config.search_domains.clone(),
            search_exclude_domains: config.search_exclude_domains.clone(),
            search_context_size: config.search_context_size.clone(),
            no_color: config.no_color,
        }
    }
}

/// Token limit warning threshold (80% of ~128k).
const TOKEN_WARNING_THRESHOLD: u64 = 100_000;

pub async fn run_interactive(config: &ResolvedConfig) -> Result<()> {
    let client = ApiClient::new(&config.api_key).context("Failed to create API client")?;
    let mut cost_tracker = CostTracker::new();
    let mut session_config = SessionConfig::from_resolved(config);
    let mut messages: Vec<Message> = Vec::new();
    let mut cumulative_tokens: u64 = 0;

    let is_stdout_tty = is_terminal::is_terminal(std::io::stdout());
    let use_color = !config.no_color && is_stdout_tty;

    // Print welcome banner
    let version = env!("CARGO_PKG_VERSION");
    if use_color {
        use owo_colors::OwoColorize;
        eprintln!(
            "{} {} (model: {})",
            "pplx interactive".bold().cyan(),
            version,
            session_config.model.green()
        );
    } else {
        eprintln!(
            "pplx interactive {} (model: {})",
            version, session_config.model
        );
    }
    eprintln!("Type /help for commands, /quit to exit.\n");

    let mut editor = history::create_editor();

    loop {
        // Use spawn_blocking for synchronous rustyline to avoid blocking tokio
        let line = {
            let prompt = "pplx> ".to_string();
            // Move editor into blocking task then get it back
            let result = tokio::task::spawn_blocking(move || {
                let readline = editor.readline(&prompt);
                (editor, readline)
            })
            .await
            .context("Readline task panicked")?;
            editor = result.0;
            result.1
        };

        match line {
            Ok(input) => {
                let input = input.trim().to_string();
                if input.is_empty() {
                    continue;
                }

                let _ = editor.add_history_entry(&input);

                // Slash command
                if input.starts_with('/') {
                    match handle_command(&input, &mut session_config, &mut messages, &cost_tracker)
                    {
                        CommandResult::Quit => break,
                        CommandResult::Error(e) => eprintln!("Error: {e}"),
                        CommandResult::Continue => {}
                    }
                    continue;
                }

                // Regular query — send to API
                messages.push(Message {
                    role: "user".to_string(),
                    content: input,
                });

                // Build the domain filter
                let domain_filter = build_domain_filter(
                    &session_config.search_domains,
                    &session_config.search_exclude_domains,
                );

                // Build request with full conversation history
                let mut req_messages = Vec::new();
                if let Some(ref sys) = session_config.system_prompt {
                    req_messages.push(Message {
                        role: "system".to_string(),
                        content: sys.clone(),
                    });
                }
                req_messages.extend(messages.clone());

                let request = ChatCompletionRequest {
                    model: session_config.model.clone(),
                    messages: req_messages,
                    max_tokens: config.max_tokens,
                    temperature: config.temperature,
                    top_p: config.top_p,
                    stream: Some(true),
                    search_domain_filter: domain_filter,
                    search_recency_filter: session_config.search_recency.clone(),
                    search_after_date_filter: config.after.clone(),
                    search_before_date_filter: config.before.clone(),
                    last_updated_after_filter: config.updated_after.clone(),
                    last_updated_before_filter: config.updated_before.clone(),
                    return_images: None,
                    return_related_questions: None,
                    search_mode: session_config.search_mode.clone(),
                    search_context_size: session_config.search_context_size.clone(),
                    reasoning_effort: config.reasoning_effort.clone(),
                    response_format: None,
                    disable_search: None,
                    enable_search_classifier: None,
                };

                // Create handlers
                let mut token_handler = output::create_token_handler(&session_config.output_format);

                let mut think_handler = if session_config.show_reasoning {
                    output::create_think_token_handler(&session_config.output_format)
                } else {
                    None
                };

                let result = client
                    .chat_completion_stream(
                        &request,
                        |token| {
                            token_handler(token);
                        },
                        think_handler
                            .as_mut()
                            .map(|h| h.as_mut() as &mut dyn FnMut(&str)),
                    )
                    .await;

                match result {
                    Ok(result) => {
                        // Newline after streamed content
                        if !result.content.is_empty() {
                            println!();
                        }

                        // Show citations inline if available
                        if config.show_citations {
                            if let Some(ref cites) = result.citations {
                                output::citations::render_citations(cites, use_color);
                            }
                        }

                        // Render metadata
                        output::render_final(&RenderFinalOpts {
                            format: &session_config.output_format,
                            show_citations: false, // already shown above
                            show_usage: config.show_usage,
                            show_cost: config.show_cost,
                            show_images: false,
                            show_related: false,
                            show_search_results: false,
                            use_color,
                            citations: None,
                            usage: result.usage.as_ref(),
                            images: None,
                            related: None,
                            search_results: None,
                        });

                        // Track cost
                        if let Some(ref usage) = result.usage {
                            cost_tracker.add(usage);
                            cumulative_tokens += usage.total_tokens as u64;
                        }

                        // Append assistant message to history
                        messages.push(Message {
                            role: "assistant".to_string(),
                            content: result.content,
                        });

                        // Context limit warning
                        if cumulative_tokens > TOKEN_WARNING_THRESHOLD {
                            if use_color {
                                use owo_colors::OwoColorize;
                                eprintln!(
                                    "\n{}",
                                    format!(
                                        "Warning: ~{cumulative_tokens} tokens used. Consider /clear to free context."
                                    )
                                    .dimmed()
                                );
                            } else {
                                eprintln!(
                                    "\nWarning: ~{cumulative_tokens} tokens used. Consider /clear to free context."
                                );
                            }
                        }

                        println!();
                    }
                    Err(e) => {
                        eprintln!("\nError: {e}");
                        // Remove the user message that caused the error
                        messages.pop();
                        println!();
                    }
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                // Ctrl+C — cancel current input, don't exit
                eprintln!("^C");
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                // Ctrl+D — exit
                break;
            }
            Err(e) => {
                eprintln!("Error reading input: {e}");
                break;
            }
        }
    }

    // Save history
    history::save_history(&mut editor);

    // Print session summary
    eprintln!();
    let summary = cost_tracker.summary_line();
    if use_color {
        use owo_colors::OwoColorize;
        eprintln!("{}", summary.dimmed());
    } else {
        eprintln!("{summary}");
    }

    Ok(())
}

fn build_domain_filter(include: &[String], exclude: &[String]) -> Option<Vec<String>> {
    if include.is_empty() && exclude.is_empty() {
        return None;
    }
    let mut filters: Vec<String> = include.to_vec();
    for domain in exclude {
        filters.push(format!("-{domain}"));
    }
    Some(filters)
}

mod api;
mod cli;
mod config;
mod cost;
mod error;
mod interactive;
mod output;

use std::io::Read;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use api::client::{with_retry, ApiClient};
use api::types::{
    AgentRequest, AgentTool, AsyncResearchSubmitRequest, ChatCompletionRequest, JsonSchemaWrapper,
    ResponseFormat, SearchQuery, SearchRequest,
};
use cli::args::{Cli, Commands, ConfigAction, ResearchAction};
use config::types::ResolvedConfig;
use output::RenderFinalOpts;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("pplx=debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("pplx=warn".parse().unwrap()),
            )
            .init();
    }

    let file_config = config::load_file_config(cli.config.as_deref());
    let resolved = config::resolve(&cli, &file_config)?;

    match &cli.command {
        Some(Commands::Completions { shell }) => {
            cli::completions::generate_completions(*shell);
            return Ok(());
        }
        Some(Commands::Config { action }) => {
            return run_config(action.as_ref(), &resolved);
        }
        Some(Commands::Interactive) => {
            return interactive::run_interactive(&resolved).await;
        }
        Some(Commands::Search {
            query,
            max_results,
            max_tokens_per_page,
            country,
        }) => {
            if query.is_empty() {
                anyhow::bail!("Search query required. Usage: pplx search <query>");
            }
            run_search(
                query,
                *max_results,
                *max_tokens_per_page,
                country.clone(),
                &resolved,
            )
            .await?;
        }
        Some(Commands::Research {
            action,
            query,
            async_mode,
        }) => {
            run_research(action.as_ref(), query, *async_mode, &resolved).await?;
        }
        Some(Commands::Agent {
            query,
            tools,
            max_steps: _,
        }) => {
            if query.is_empty() {
                anyhow::bail!("Agent query required. Usage: pplx agent <query>");
            }
            run_agent(query, tools, &resolved).await?;
        }
        Some(Commands::Ask { query }) => {
            run_ask(query, &resolved).await?;
        }
        None => {
            if cli.query.is_empty() {
                // Show help if no query and stdin is a TTY (no piped data)
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!();
                return Ok(());
            }
            run_ask(&cli.query, &resolved).await?;
        }
    }

    Ok(())
}

async fn run_ask(query_parts: &[String], config: &ResolvedConfig) -> Result<()> {
    let is_tty = is_terminal::is_terminal(std::io::stdin());
    let is_stdout_tty = is_terminal::is_terminal(std::io::stdout());

    // Build query from args + stdin
    let mut query = query_parts.join(" ");

    if !is_tty {
        let mut stdin_buf = String::new();
        std::io::stdin()
            .read_to_string(&mut stdin_buf)
            .context("Failed to read stdin")?;
        if !stdin_buf.trim().is_empty() {
            if query.is_empty() {
                query = stdin_buf.trim().to_string();
            } else {
                query = format!("{query}\n\n{}", stdin_buf.trim());
            }
        }
    }

    if query.trim().is_empty() {
        anyhow::bail!("No query provided. Usage: pplx \"your question here\"");
    }

    // Determine effective color/spinner settings
    let use_color = !config.no_color && is_stdout_tty;
    let use_spinner = is_stdout_tty && !config.no_stream && config.output_format != "json";

    let client = ApiClient::new(&config.api_key).context("Failed to create API client")?;

    // Build messages
    let mut messages = Vec::new();
    if let Some(ref sys) = config.system_prompt {
        messages.push(api::types::Message {
            role: "system".to_string(),
            content: sys.clone(),
        });
    }
    messages.push(api::types::Message {
        role: "user".to_string(),
        content: query,
    });

    // Build domain filter
    let domain_filter = build_domain_filter(&config.search_domains, &config.search_exclude_domains);

    let request = ChatCompletionRequest {
        model: config.model.clone(),
        messages,
        max_tokens: config.max_tokens,
        temperature: config.temperature,
        top_p: config.top_p,
        stream: Some(!config.no_stream),
        search_domain_filter: domain_filter,
        search_recency_filter: config.search_recency.clone(),
        search_after_date_filter: config.after.clone(),
        search_before_date_filter: config.before.clone(),
        last_updated_after_filter: config.updated_after.clone(),
        last_updated_before_filter: config.updated_before.clone(),
        return_images: if config.images { Some(true) } else { None },
        return_related_questions: if config.related { Some(true) } else { None },
        search_mode: config.search_mode.clone(),
        search_context_size: config.search_context_size.clone(),
        reasoning_effort: config.reasoning_effort.clone(),
        response_format: build_response_format(config)?,
        disable_search: if config.no_search { Some(true) } else { None },
        enable_search_classifier: if config.smart_search {
            Some(true)
        } else {
            None
        },
    };

    // Track response content for --save
    let save_content: String;

    if config.no_stream {
        // Non-streaming mode
        let spinner = if use_spinner {
            let sp = ProgressBar::new_spinner();
            sp.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            sp.set_message("Searching...");
            sp.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(sp)
        } else {
            None
        };

        let response = with_retry(|| client.chat_completion(&request)).await?;

        if let Some(sp) = spinner {
            sp.finish_and_clear();
        }

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .unwrap_or("");

        // Parse and render think blocks from non-streaming response
        if config.show_reasoning {
            let (thinking, clean) = extract_think_blocks(content);
            if let Some(ref thinking_text) = thinking {
                render_thinking_block(thinking_text, &config.output_format, use_color);
            }
            match config.output_format.as_str() {
                "json" => output::json::render_response(&response),
                "plain" | "raw" => output::plain::render_full(&clean),
                _ => output::markdown::render_full(&clean, use_color),
            }
        } else {
            match config.output_format.as_str() {
                "json" => output::json::render_response(&response),
                "plain" | "raw" => output::plain::render_full(content),
                _ => output::markdown::render_full(content, use_color),
            }
        }

        output::render_final(&RenderFinalOpts {
            format: &config.output_format,
            show_citations: config.show_citations,
            show_usage: config.show_usage,
            show_cost: config.show_cost,
            show_images: config.images,
            show_related: config.related,
            show_search_results: config.search_results,
            use_color,
            citations: response.citations.as_deref(),
            usage: response.usage.as_ref(),
            images: response.images.as_deref(),
            related: response.related_questions.as_deref(),
            search_results: response.search_results.as_deref(),
        });

        save_content = if config.output_format == "json" {
            serde_json::to_string_pretty(&response).unwrap_or_default()
        } else {
            content.to_string()
        };
    } else {
        // Streaming mode
        let spinner = if use_spinner {
            let sp = ProgressBar::new_spinner();
            sp.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            sp.set_message("Searching...");
            sp.enable_steady_tick(std::time::Duration::from_millis(80));
            Some(sp)
        } else {
            None
        };

        let mut first_token = true;
        let mut token_handler = output::create_token_handler(&config.output_format);

        // Create think token handler if reasoning display is enabled
        let mut think_handler = if config.show_reasoning {
            output::create_think_token_handler(&config.output_format)
        } else {
            None
        };

        let mut think_ended = false;

        let result = client
            .chat_completion_stream(
                &request,
                |token| {
                    if first_token {
                        if let Some(ref sp) = spinner {
                            sp.finish_and_clear();
                        }
                        first_token = false;
                    }
                    // If we were showing thinking and this is the first normal token, add a separator
                    if config.show_reasoning && !think_ended && config.output_format != "json" {
                        think_ended = true;
                        eprintln!();
                    }
                    token_handler(token);
                },
                think_handler
                    .as_mut()
                    .map(|h| h.as_mut() as &mut dyn FnMut(&str)),
            )
            .await;

        // Ensure spinner is cleared even on error
        if let Some(sp) = &spinner {
            sp.finish_and_clear();
        }

        let result = result?;

        // Newline after streamed content
        if config.output_format != "json" && !result.content.is_empty() {
            println!();
        }

        if config.output_format == "json" {
            output::json::render_stream_result(&result);
        }

        output::render_final(&RenderFinalOpts {
            format: &config.output_format,
            show_citations: config.show_citations,
            show_usage: config.show_usage,
            show_cost: config.show_cost,
            show_images: config.images,
            show_related: config.related,
            show_search_results: config.search_results,
            use_color,
            citations: result.citations.as_deref(),
            usage: result.usage.as_ref(),
            images: result.images.as_deref(),
            related: result.related_questions.as_deref(),
            search_results: result.search_results.as_deref(),
        });

        save_content = result.content;
    }

    if let Some(ref path) = config.save {
        save_response(path, &save_content, &config.output_format)?;
    }

    Ok(())
}

async fn run_search(
    query_parts: &[String],
    max_results: Option<u32>,
    max_tokens_per_page: Option<u32>,
    country: Option<String>,
    config: &ResolvedConfig,
) -> Result<()> {
    let is_stdout_tty = is_terminal::is_terminal(std::io::stdout());
    let use_color = !config.no_color && is_stdout_tty;
    let use_spinner = is_stdout_tty && config.output_format != "json";

    let client = ApiClient::new(&config.api_key).context("Failed to create API client")?;

    // Build domain filter
    let domain_filter = build_domain_filter(&config.search_domains, &config.search_exclude_domains);

    // Determine single or multi query
    let query = if query_parts.len() > 1 {
        SearchQuery::Multi(query_parts.to_vec())
    } else {
        SearchQuery::Single(query_parts.join(" "))
    };

    let request = SearchRequest {
        query,
        max_results,
        max_tokens_per_page,
        country,
        search_domain_filter: domain_filter,
        search_recency_filter: config.search_recency.clone(),
        search_after_date_filter: config.after.clone(),
        search_before_date_filter: config.before.clone(),
        search_mode: config.search_mode.clone(),
    };

    let spinner = if use_spinner {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        sp.set_message("Searching...");
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    let response = with_retry(|| client.search(&request)).await?;

    if let Some(sp) = spinner {
        sp.finish_and_clear();
    }

    output::render_search_output(&response, &config.output_format, use_color);

    if let Some(ref path) = config.save {
        let content = if config.output_format == "json" {
            serde_json::to_string_pretty(&response).unwrap_or_default()
        } else {
            response
                .results
                .iter()
                .map(|r| {
                    let mut s = format!("{}\n{}", r.title, r.url);
                    if let Some(ref snippet) = r.snippet {
                        s.push_str(&format!("\n{snippet}"));
                    }
                    s
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        };
        save_response(path, &content, &config.output_format)?;
    }

    Ok(())
}

async fn run_research(
    action: Option<&ResearchAction>,
    query: &[String],
    async_mode: bool,
    config: &ResolvedConfig,
) -> Result<()> {
    let is_stdout_tty = is_terminal::is_terminal(std::io::stdout());
    let use_color = !config.no_color && is_stdout_tty;

    let client = ApiClient::new(&config.api_key).context("Failed to create API client")?;

    match action {
        Some(ResearchAction::Status { id }) => {
            let status = client.research_status(id).await?;
            output::render_research_status(&status, &config.output_format);
        }
        Some(ResearchAction::List) => {
            let list = client.research_list().await?;
            output::render_research_list(&list, &config.output_format);
        }
        Some(ResearchAction::Get { id }) => {
            let status = client.research_status(id).await?;
            if status.status != "completed" {
                eprintln!("Job {} is still {}", id, status.status);
                return Ok(());
            }
            if let Some(response) = status.response {
                let content = response
                    .choices
                    .first()
                    .map(|c| c.message.content.as_str())
                    .unwrap_or("");
                match config.output_format.as_str() {
                    "json" => output::json::render_response(&response),
                    "plain" | "raw" => output::plain::render_full(content),
                    _ => output::markdown::render_full(content, use_color),
                }
                output::render_final(&RenderFinalOpts {
                    format: &config.output_format,
                    show_citations: config.show_citations,
                    show_usage: config.show_usage,
                    show_cost: config.show_cost,
                    show_images: config.images,
                    show_related: config.related,
                    show_search_results: config.search_results,
                    use_color,
                    citations: response.citations.as_deref(),
                    usage: response.usage.as_ref(),
                    images: response.images.as_deref(),
                    related: response.related_questions.as_deref(),
                    search_results: response.search_results.as_deref(),
                });
            } else {
                eprintln!("Job completed but no response data available.");
            }
        }
        None => {
            // Submit new research job
            if query.is_empty() {
                anyhow::bail!("Research query required. Usage: pplx research <query>");
            }

            let query_text = query.join(" ");
            let domain_filter =
                build_domain_filter(&config.search_domains, &config.search_exclude_domains);

            let request = AsyncResearchSubmitRequest {
                model: "sonar-deep-research".to_string(),
                messages: vec![api::types::Message {
                    role: "user".to_string(),
                    content: query_text,
                }],
                max_tokens: config.max_tokens,
                temperature: config.temperature,
                search_domain_filter: domain_filter,
                search_recency_filter: config.search_recency.clone(),
                search_after_date_filter: config.after.clone(),
                search_before_date_filter: config.before.clone(),
                search_mode: config.search_mode.clone(),
                search_context_size: config.search_context_size.clone(),
            };

            let submit = client.research_submit(&request).await?;

            if async_mode {
                println!("{}", submit.id);
                return Ok(());
            }

            // Poll for completion
            eprintln!("Research job submitted: {}", submit.id);
            poll_research_until_complete(&client, &submit.id, config).await?;
        }
    }

    Ok(())
}

async fn poll_research_until_complete(
    client: &ApiClient,
    job_id: &str,
    config: &ResolvedConfig,
) -> Result<()> {
    let is_stdout_tty = is_terminal::is_terminal(std::io::stdout());
    let use_color = !config.no_color && is_stdout_tty;
    let use_spinner = is_stdout_tty && config.output_format != "json";

    let spinner = if use_spinner {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        sp.set_message("Researching...");
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    let timeout = std::time::Duration::from_secs(300);
    let start = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_secs(5);

    loop {
        if start.elapsed() > timeout {
            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }
            return Err(error::PplxError::Research(format!(
                "Research job {job_id} timed out after 300s. Check status with: pplx research status {job_id}"
            ))
            .into());
        }

        tokio::time::sleep(poll_interval).await;

        let status = client.research_status(job_id).await?;

        if let Some(ref sp) = spinner {
            sp.set_message(format!("Researching... ({}s)", start.elapsed().as_secs()));
        }

        match status.status.as_str() {
            "completed" => {
                if let Some(sp) = spinner {
                    sp.finish_and_clear();
                }
                if let Some(response) = status.response {
                    let content = response
                        .choices
                        .first()
                        .map(|c| c.message.content.as_str())
                        .unwrap_or("");
                    match config.output_format.as_str() {
                        "json" => output::json::render_response(&response),
                        "plain" | "raw" => output::plain::render_full(content),
                        _ => output::markdown::render_full(content, use_color),
                    }
                    output::render_final(&RenderFinalOpts {
                        format: &config.output_format,
                        show_citations: config.show_citations,
                        show_usage: config.show_usage,
                        show_cost: config.show_cost,
                        show_images: config.images,
                        show_related: config.related,
                        show_search_results: config.search_results,
                        use_color,
                        citations: response.citations.as_deref(),
                        usage: response.usage.as_ref(),
                        images: response.images.as_deref(),
                        related: response.related_questions.as_deref(),
                        search_results: response.search_results.as_deref(),
                    });

                    if let Some(ref path) = config.save {
                        let save_content = if config.output_format == "json" {
                            serde_json::to_string_pretty(&response).unwrap_or_default()
                        } else {
                            content.to_string()
                        };
                        save_response(path, &save_content, &config.output_format)?;
                    }
                }
                return Ok(());
            }
            "failed" => {
                if let Some(sp) = spinner {
                    sp.finish_and_clear();
                }
                return Err(
                    error::PplxError::Research(format!("Research job {job_id} failed")).into(),
                );
            }
            _ => {
                // Still pending, continue polling
            }
        }
    }
}

async fn run_agent(
    query_parts: &[String],
    tools: &[String],
    config: &ResolvedConfig,
) -> Result<()> {
    let is_stdout_tty = is_terminal::is_terminal(std::io::stdout());
    let use_color = !config.no_color && is_stdout_tty;
    let use_spinner = is_stdout_tty && config.output_format != "json";

    let client = ApiClient::new(&config.api_key).context("Failed to create API client")?;

    let query = query_parts.join(" ");

    let agent_tools = if tools.is_empty() {
        None
    } else {
        Some(
            tools
                .iter()
                .map(|t| AgentTool { r#type: t.clone() })
                .collect(),
        )
    };

    let request = AgentRequest {
        model: config.model.clone(),
        input: query,
        tools: agent_tools,
        instructions: config.system_prompt.clone(),
        max_tokens: config.max_tokens,
        temperature: config.temperature,
        stream: Some(false),
    };

    let spinner = if use_spinner {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        sp.set_message("Processing...");
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    let response = client.agent_completion(&request).await?;

    if let Some(sp) = spinner {
        sp.finish_and_clear();
    }

    let text = output::extract_agent_text(&response);
    let citations = output::extract_agent_citations(&response);

    match config.output_format.as_str() {
        "json" => {
            if let Ok(json) = serde_json::to_string_pretty(&response) {
                println!("{json}");
            }
        }
        "plain" | "raw" => output::plain::render_full(&text),
        _ => output::markdown::render_full(&text, use_color),
    }

    // Show citations if present
    if config.show_citations && !citations.is_empty() {
        output::citations::render_citations(&citations, use_color);
    }

    // Show usage if present
    if config.show_usage {
        if let Some(ref usage) = response.usage {
            println!();
            if use_color {
                use owo_colors::OwoColorize;
                println!("{}", "Usage:".bold());
            } else {
                println!("Usage:");
            }
            println!("  Input tokens:  {}", usage.input_tokens);
            println!("  Output tokens: {}", usage.output_tokens);
            println!("  Total tokens:  {}", usage.total_tokens);
        }
    }

    if let Some(ref path) = config.save {
        let save_content = if config.output_format == "json" {
            serde_json::to_string_pretty(&response).unwrap_or_default()
        } else {
            text
        };
        save_response(path, &save_content, &config.output_format)?;
    }

    Ok(())
}

/// Extract `<think>...</think>` blocks from a complete response string.
/// Returns (thinking_content, clean_content).
fn extract_think_blocks(content: &str) -> (Option<String>, String) {
    use api::think::ThinkParser;

    let mut parser = ThinkParser::new();
    let mut thinking = String::new();
    let mut clean = String::new();

    for event in parser.feed(content) {
        match event {
            api::think::ThinkEvent::Think(s) => thinking.push_str(&s),
            api::think::ThinkEvent::Normal(s) => clean.push_str(&s),
        }
    }
    for event in parser.flush() {
        match event {
            api::think::ThinkEvent::Think(s) => thinking.push_str(&s),
            api::think::ThinkEvent::Normal(s) => clean.push_str(&s),
        }
    }

    let thinking = if thinking.is_empty() {
        None
    } else {
        Some(thinking)
    };
    (thinking, clean)
}

/// Render thinking content block before the main response.
fn render_thinking_block(thinking: &str, format: &str, use_color: bool) {
    match format {
        "json" => {} // handled in JSON output
        "plain" | "raw" => {
            for line in thinking.lines() {
                println!("[thinking] {line}");
            }
            println!();
        }
        _ => {
            if use_color {
                use owo_colors::OwoColorize;
                for line in thinking.lines() {
                    println!("{}", line.dimmed().italic());
                }
            } else {
                for line in thinking.lines() {
                    println!("{line}");
                }
            }
            println!();
        }
    }
}

fn build_response_format(config: &ResolvedConfig) -> Result<Option<ResponseFormat>> {
    let Some(ref schema_input) = config.json_schema else {
        return Ok(None);
    };

    let schema: serde_json::Value = if std::path::Path::new(schema_input).exists() {
        let content =
            std::fs::read_to_string(schema_input).context("Failed to read JSON schema file")?;
        serde_json::from_str(&content).context("Failed to parse JSON schema file")?
    } else {
        serde_json::from_str(schema_input).context("Failed to parse inline JSON schema")?
    };

    Ok(Some(ResponseFormat {
        r#type: "json_schema".to_string(),
        json_schema: Some(JsonSchemaWrapper { schema }),
    }))
}

fn save_response(path: &str, content: &str, format: &str) -> Result<()> {
    let save_content = if path.ends_with(".json") && format != "json" {
        // Wrap plain content in JSON if saving to .json but format isn't json
        serde_json::to_string_pretty(&serde_json::json!({ "content": content }))
            .unwrap_or_else(|_| content.to_string())
    } else {
        content.to_string()
    };

    std::fs::write(path, &save_content).with_context(|| format!("Failed to write to {path}"))?;
    eprintln!("Saved to: {path}");
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

fn run_config(action: Option<&ConfigAction>, config: &ResolvedConfig) -> Result<()> {
    match action {
        Some(ConfigAction::Init) => {
            let config_dir = dirs::config_dir()
                .map(|d| d.join("pplx"))
                .context("Could not determine config directory")?;
            let config_path = config_dir.join("config.toml");

            if config_path.exists() {
                eprintln!("Config file already exists at: {}", config_path.display());
                eprintln!("Edit it directly or use `pplx config set <key> <value>`");
                return Ok(());
            }

            // Prompt for API key
            eprint!("Enter your Perplexity API key: ");
            let mut api_key = String::new();
            std::io::stdin()
                .read_line(&mut api_key)
                .context("Failed to read API key")?;
            let api_key = api_key.trim();

            if api_key.is_empty() {
                anyhow::bail!(
                    "API key cannot be empty. Get one at https://www.perplexity.ai/settings/api"
                );
            }

            std::fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

            let config_content = format!(
                r#"[auth]
api_key = "{api_key}"

[defaults]
model = "sonar-pro"
output = "md"
# temperature = 0.7
# show_citations = true
# show_cost = false
# show_usage = false

# [defaults.search]
# mode = "web"
# recency = "month"
# domains = []
# exclude_domains = ["pinterest.com", "quora.com"]
"#
            );

            std::fs::write(&config_path, config_content).context("Failed to write config file")?;

            println!("Config created at: {}", config_path.display());
            println!("You're ready to go! Try: pplx \"What is Rust?\"");
            Ok(())
        }
        Some(ConfigAction::Show) => {
            println!("Effective configuration:");
            println!("  model:         {}", config.model);
            println!("  output:        {}", config.output_format);
            println!("  citations:     {}", config.show_citations);
            println!("  usage:         {}", config.show_usage);
            println!("  cost:          {}", config.show_cost);
            println!("  no_stream:     {}", config.no_stream);
            println!("  no_color:      {}", config.no_color);
            if let Some(ref t) = config.temperature {
                println!("  temperature:   {t}");
            }
            if let Some(ref m) = config.search_mode {
                println!("  search_mode:   {m}");
            }
            if let Some(ref r) = config.search_recency {
                println!("  recency:       {r}");
            }
            if let Some(ref c) = config.search_context_size {
                println!("  context_size:  {c}");
            }
            if !config.search_domains.is_empty() {
                println!("  domains:       {:?}", config.search_domains);
            }
            if !config.search_exclude_domains.is_empty() {
                println!("  exclude:       {:?}", config.search_exclude_domains);
            }
            println!(
                "  api_key:       {}",
                if config.api_key.is_empty() {
                    "(not set)"
                } else {
                    "(set)"
                }
            );

            let config_path = dirs::config_dir().map(|d| d.join("pplx").join("config.toml"));
            if let Some(path) = config_path {
                if path.exists() {
                    println!("\n  config file:   {}", path.display());
                } else {
                    println!("\n  config file:   (not created — run `pplx config init`)");
                }
            }
            Ok(())
        }
        Some(ConfigAction::Set { key, value }) => {
            config::set_config_value(key, value)?;
            println!("Set {key} = {value}");
            Ok(())
        }
        None => {
            // No subcommand — show help for config
            println!("Usage: pplx config <init|show|set>");
            println!();
            println!("  init    Create config file with your API key");
            println!("  show    Show current effective configuration");
            println!("  set     Set a configuration value");
            Ok(())
        }
    }
}

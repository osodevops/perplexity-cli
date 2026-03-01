mod api;
mod cli;
mod config;
mod cost;
mod error;
mod output;

use std::io::Read;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use api::client::{with_retry, ApiClient};
use api::types::ChatCompletionRequest;
use cli::args::{Cli, Commands, ConfigAction};
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
            eprintln!("Interactive mode not yet implemented (Phase 3)");
            return Ok(());
        }
        Some(Commands::Search { query }) => {
            if query.is_empty() {
                anyhow::bail!("Search query required. Usage: pplx search <query>");
            }
            eprintln!("Search API not yet implemented (Phase 2)");
            return Ok(());
        }
        Some(Commands::Research { query }) => {
            if query.is_empty() {
                anyhow::bail!("Research query required. Usage: pplx research <query>");
            }
            eprintln!("Research API not yet implemented (Phase 3)");
            return Ok(());
        }
        Some(Commands::Agent { query }) => {
            if query.is_empty() {
                anyhow::bail!("Agent query required. Usage: pplx agent <query>");
            }
            eprintln!("Agent API not yet implemented (Phase 3)");
            return Ok(());
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
    let domain_filter =
        build_domain_filter(&config.search_domains, &config.search_exclude_domains);

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
        response_format: None,
        disable_search: if config.no_search { Some(true) } else { None },
        enable_search_classifier: if config.smart_search {
            Some(true)
        } else {
            None
        },
    };

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

        let result = client
            .chat_completion_stream(&request, |token| {
                if first_token {
                    if let Some(ref sp) = spinner {
                        sp.finish_and_clear();
                    }
                    first_token = false;
                }
                token_handler(token);
            })
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
                anyhow::bail!("API key cannot be empty. Get one at https://www.perplexity.ai/settings/api");
            }

            std::fs::create_dir_all(&config_dir)
                .context("Failed to create config directory")?;

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

            std::fs::write(&config_path, config_content)
                .context("Failed to write config file")?;

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
            println!("  api_key:       {}", if config.api_key.is_empty() { "(not set)" } else { "(set)" });

            let config_path = dirs::config_dir()
                .map(|d| d.join("pplx").join("config.toml"));
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
            eprintln!("`pplx config set {key} {value}` is not yet implemented.");
            eprintln!("Edit your config file directly: ~/.config/pplx/config.toml");
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

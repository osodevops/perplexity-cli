pub mod citations;
pub mod json;
pub mod markdown;
pub mod plain;

use crate::api::types::{
    AgentResponse, AsyncResearchListResponse, AsyncResearchStatusResponse, ImageResult,
    SearchResponse, SearchResult, Usage,
};

/// Create a token handler based on output format.
/// For JSON output in streaming mode, we still collect tokens silently.
pub fn create_token_handler(format: &str) -> Box<dyn FnMut(&str)> {
    match format {
        "json" => Box::new(|_: &str| {}), // silent; we render JSON at the end
        "plain" | "raw" => plain::streaming_token_handler(),
        _ => markdown::streaming_token_handler(), // "md" and default
    }
}

/// A boxed mutable closure for handling token callbacks.
pub type TokenHandler = Box<dyn FnMut(&str)>;

/// Create a think token handler based on output format.
/// Returns None for JSON (thinking content rendered in final JSON).
pub fn create_think_token_handler(format: &str) -> Option<TokenHandler> {
    match format {
        "json" => None,
        "plain" | "raw" => Some(plain::streaming_think_token_handler()),
        _ => Some(markdown::streaming_think_token_handler()),
    }
}

/// Render research job status.
pub fn render_research_status(response: &AsyncResearchStatusResponse, format: &str) {
    match format {
        "json" => {
            if let Ok(json) = serde_json::to_string_pretty(response) {
                println!("{json}");
            }
        }
        _ => {
            println!("Job ID:  {}", response.id);
            println!("Status:  {}", response.status);
        }
    }
}

/// Render research job list.
pub fn render_research_list(response: &AsyncResearchListResponse, format: &str) {
    match format {
        "json" => {
            if let Ok(json) = serde_json::to_string_pretty(response) {
                println!("{json}");
            }
        }
        _ => {
            if response.items.is_empty() {
                println!("No research jobs found.");
                return;
            }
            for item in &response.items {
                let ts = item
                    .created_at
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "-".to_string());
                println!("  {} [{}] created={}", item.id, item.status, ts);
            }
        }
    }
}

/// Extract text content from an AgentResponse.
pub fn extract_agent_text(response: &AgentResponse) -> String {
    let mut parts = Vec::new();
    for item in &response.output {
        if let Some(ref text) = item.text {
            parts.push(text.clone());
        }
        if let Some(ref content) = item.content {
            for c in content {
                if let Some(ref text) = c.text {
                    parts.push(text.clone());
                }
            }
        }
    }
    parts.join("")
}

/// Extract citation URLs from an AgentResponse.
pub fn extract_agent_citations(response: &AgentResponse) -> Vec<String> {
    let mut urls = Vec::new();
    for item in &response.output {
        if let Some(ref content) = item.content {
            for c in content {
                if let Some(ref annotations) = c.annotations {
                    for ann in annotations {
                        if let Some(ref url) = ann.url {
                            if !urls.contains(url) {
                                urls.push(url.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    urls
}

/// Options for rendering final metadata after the response.
pub struct RenderFinalOpts<'a> {
    pub format: &'a str,
    pub show_citations: bool,
    pub show_usage: bool,
    pub show_cost: bool,
    pub show_images: bool,
    pub show_related: bool,
    pub show_search_results: bool,
    pub use_color: bool,
    pub citations: Option<&'a [String]>,
    pub usage: Option<&'a Usage>,
    pub images: Option<&'a [ImageResult]>,
    pub related: Option<&'a [String]>,
    pub search_results: Option<&'a [SearchResult]>,
}

/// Render final metadata below the response.
pub fn render_final(opts: &RenderFinalOpts<'_>) {
    // JSON mode handles its own rendering
    if opts.format == "json" {
        return;
    }

    if opts.show_citations {
        if let Some(cites) = opts.citations {
            self::citations::render_citations(cites, opts.use_color);
        }
    }

    if opts.show_search_results {
        if let Some(results) = opts.search_results {
            render_search_results(results, opts.use_color);
        }
    }

    if opts.show_images {
        if let Some(imgs) = opts.images {
            render_images(imgs);
        }
    }

    if opts.show_related {
        if let Some(questions) = opts.related {
            render_related(questions, opts.use_color);
        }
    }

    if opts.show_usage {
        if let Some(u) = opts.usage {
            render_usage(u, opts.use_color);
        }
    }

    if opts.show_cost {
        if let Some(u) = opts.usage {
            crate::cost::render(u, opts.use_color);
        }
    }
}

/// Render search results in markdown/plain format.
pub fn render_search_output(response: &SearchResponse, format: &str, use_color: bool) {
    match format {
        "json" => json::render_search_response(response),
        _ => {
            render_search_results(&response.results, use_color);
        }
    }
}

fn render_usage(usage: &Usage, use_color: bool) {
    use owo_colors::OwoColorize;
    println!();
    if use_color {
        println!("{}", "Usage:".bold());
    } else {
        println!("Usage:");
    }
    println!("  Prompt tokens:     {}", usage.prompt_tokens);
    println!("  Completion tokens: {}", usage.completion_tokens);
    println!("  Total tokens:      {}", usage.total_tokens);
    if let Some(ct) = usage.citation_tokens {
        println!("  Citation tokens:   {ct}");
    }
    if let Some(rt) = usage.reasoning_tokens {
        println!("  Reasoning tokens:  {rt}");
    }
    if let Some(sq) = usage.num_search_queries {
        println!("  Search queries:    {sq}");
    }
}

fn render_search_results(results: &[SearchResult], use_color: bool) {
    if results.is_empty() {
        return;
    }
    use owo_colors::OwoColorize;
    println!();
    if use_color {
        println!("{}", "Search Results:".bold());
    } else {
        println!("Search Results:");
    }
    for (i, r) in results.iter().enumerate() {
        if use_color {
            println!(
                "  [{}] {} - {}",
                (i + 1).dimmed(),
                r.title.bold(),
                r.url.cyan()
            );
        } else {
            println!("  [{}] {} - {}", i + 1, r.title, r.url);
        }
        if let Some(ref snippet) = r.snippet {
            println!("      {snippet}");
        }
    }
}

fn render_images(images: &[ImageResult]) {
    if images.is_empty() {
        return;
    }
    println!();
    println!("Images:");
    for (i, img) in images.iter().enumerate() {
        let title = img.title.as_deref().unwrap_or("Untitled");
        println!("  [{}] {} - {}", i + 1, title, img.image_url);
    }
}

fn render_related(questions: &[String], use_color: bool) {
    if questions.is_empty() {
        return;
    }
    use owo_colors::OwoColorize;
    println!();
    if use_color {
        println!("{}", "Related Questions:".bold());
    } else {
        println!("Related Questions:");
    }
    for q in questions {
        println!("  - {q}");
    }
}

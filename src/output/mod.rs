pub mod citations;
pub mod json;
pub mod markdown;
pub mod plain;

use crate::api::types::{ImageResult, SearchResult, Usage};

/// Create a token handler based on output format.
/// For JSON output in streaming mode, we still collect tokens silently.
pub fn create_token_handler(format: &str) -> Box<dyn FnMut(&str)> {
    match format {
        "json" => Box::new(|_: &str| {}), // silent; we render JSON at the end
        "plain" | "raw" => plain::streaming_token_handler(),
        _ => markdown::streaming_token_handler(), // "md" and default
    }
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
            println!("  [{}] {} - {}", (i + 1).dimmed(), r.title.bold(), r.url.cyan());
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

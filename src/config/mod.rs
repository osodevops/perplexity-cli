pub mod types;

use std::path::PathBuf;

use crate::cli::args::Cli;
use crate::error::PplxError;
use types::{FileConfig, ResolvedConfig};

/// Returns the config file path: $XDG_CONFIG_HOME/pplx/config.toml
fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("pplx").join("config.toml"))
}

/// Load configuration from TOML file. Returns default if file doesn't exist.
pub fn load_file_config(path_override: Option<&str>) -> FileConfig {
    let path = path_override
        .map(PathBuf::from)
        .or_else(default_config_path);

    let Some(path) = path else {
        return FileConfig::default();
    };

    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse config file {}: {}", path.display(), e);
            FileConfig::default()
        }),
        Err(_) => FileConfig::default(),
    }
}

/// Resolve configuration: CLI flags > env vars > TOML file > defaults
pub fn resolve(cli: &Cli, file: &FileConfig) -> Result<ResolvedConfig, PplxError> {
    let search = file.defaults.search.as_ref();

    // API key: CLI env already handled by clap env, then file config
    let api_key = std::env::var("PERPLEXITY_API_KEY")
        .ok()
        .or_else(|| file.auth.api_key.clone())
        .unwrap_or_default();

    let model = cli
        .model
        .clone()
        .or_else(|| file.defaults.model.clone())
        .unwrap_or_else(|| "sonar-pro".to_string());

    let output_format = cli
        .output
        .map(|o| match o {
            crate::cli::args::OutputFormat::Md => "md",
            crate::cli::args::OutputFormat::Plain => "plain",
            crate::cli::args::OutputFormat::Json => "json",
            crate::cli::args::OutputFormat::Raw => "raw",
        })
        .map(String::from)
        .or_else(|| file.defaults.output.clone())
        .unwrap_or_else(|| "md".to_string());

    let show_citations = cli.citations || file.defaults.show_citations.unwrap_or(false);

    let show_cost = cli.cost || file.defaults.show_cost.unwrap_or(false);

    let show_usage = cli.usage || file.defaults.show_usage.unwrap_or(false);

    let temperature = cli.temperature.or(file.defaults.temperature);

    let search_context_size = cli
        .context_size
        .map(|c| c.as_api_str().to_string())
        .or_else(|| file.defaults.search_context_size.clone());

    let search_mode = cli
        .search_mode
        .map(|m| m.as_api_str().to_string())
        .or_else(|| search.and_then(|s| s.mode.clone()));

    let search_recency = cli
        .recency
        .map(|r| r.as_api_str().to_string())
        .or_else(|| search.and_then(|s| s.recency.clone()));

    let mut search_domains = cli.domains.clone();
    if search_domains.is_empty() {
        if let Some(d) = search.and_then(|s| s.domains.as_ref()) {
            search_domains = d.clone();
        }
    }

    let mut search_exclude_domains = cli.exclude_domains.clone();
    if search_exclude_domains.is_empty() {
        if let Some(d) = search.and_then(|s| s.exclude_domains.as_ref()) {
            search_exclude_domains = d.clone();
        }
    }

    let reasoning_effort = cli.reasoning_effort.map(|r| r.as_api_str().to_string());

    Ok(ResolvedConfig {
        api_key,
        model,
        output_format,
        show_citations,
        show_cost,
        show_usage,
        temperature,
        search_context_size,
        search_mode,
        search_recency,
        search_domains,
        search_exclude_domains,
        no_stream: cli.no_stream,
        no_color: cli.no_color,
        verbose: cli.verbose,
        max_tokens: cli.max_tokens,
        top_p: cli.top_p,
        reasoning_effort,
        images: cli.images,
        related: cli.related,
        search_results: cli.search_results,
        no_search: cli.no_search,
        smart_search: cli.smart_search,
        system_prompt: cli.system.clone(),
        after: cli.after.clone(),
        before: cli.before.clone(),
        updated_after: cli.updated_after.clone(),
        updated_before: cli.updated_before.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_file_config() {
        let config = FileConfig::default();
        assert!(config.auth.api_key.is_none());
        assert!(config.defaults.model.is_none());
    }

    #[test]
    fn test_parse_toml_config() {
        let toml_str = r#"
[auth]
api_key = "pplx-test-key"

[defaults]
model = "sonar"
output = "plain"
show_citations = true

[defaults.search]
mode = "academic"
recency = "week"
domains = ["arxiv.org"]
exclude_domains = ["pinterest.com"]
"#;
        let config: FileConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.auth.api_key.as_deref(), Some("pplx-test-key"));
        assert_eq!(config.defaults.model.as_deref(), Some("sonar"));
        assert_eq!(config.defaults.show_citations, Some(true));
        let search = config.defaults.search.unwrap();
        assert_eq!(search.mode.as_deref(), Some("academic"));
        assert_eq!(
            search.domains.as_deref(),
            Some(&["arxiv.org".to_string()][..])
        );
    }
}

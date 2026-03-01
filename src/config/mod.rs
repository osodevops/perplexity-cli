pub mod types;

use std::path::PathBuf;

use crate::cli::args::Cli;
use crate::error::PplxError;
use types::{FileConfig, ResolvedConfig};

/// Returns the config file path: $XDG_CONFIG_HOME/pplx/config.toml
pub fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("pplx").join("config.toml"))
}

/// Known top-level config keys that can be set via `pplx config set`.
const KNOWN_CONFIG_KEYS: &[&str] = &[
    "model",
    "output",
    "temperature",
    "top_p",
    "api_key",
    "context_size",
    "show_citations",
    "show_cost",
    "show_usage",
];

/// Set a configuration value in the TOML config file, preserving formatting.
pub fn set_config_value(key: &str, value: &str) -> Result<(), PplxError> {
    use std::io::Write;

    if !KNOWN_CONFIG_KEYS.contains(&key) {
        return Err(PplxError::Config(format!(
            "unknown config key '{key}'. Valid keys: {}",
            KNOWN_CONFIG_KEYS.join(", ")
        )));
    }

    // Validate the value for keys that have constraints
    match key {
        "temperature" => {
            let t: f32 = value.parse().map_err(|_| {
                PplxError::Validation(format!("temperature must be a number, got '{value}'"))
            })?;
            if !(0.0..=2.0).contains(&t) {
                return Err(PplxError::Validation(format!(
                    "temperature must be between 0.0 and 2.0, got {t}"
                )));
            }
        }
        "top_p" => {
            let p: f32 = value.parse().map_err(|_| {
                PplxError::Validation(format!("top_p must be a number, got '{value}'"))
            })?;
            if !(0.0..=1.0).contains(&p) {
                return Err(PplxError::Validation(format!(
                    "top_p must be between 0.0 and 1.0, got {p}"
                )));
            }
        }
        "output" => match value {
            "md" | "plain" | "json" | "raw" => {}
            _ => {
                return Err(PplxError::Validation(format!(
                    "output must be one of: md, plain, json, raw — got '{value}'"
                )));
            }
        },
        "context_size" => match value {
            "minimal" | "low" | "medium" | "high" => {}
            _ => {
                return Err(PplxError::Validation(format!(
                    "context_size must be one of: minimal, low, medium, high — got '{value}'"
                )));
            }
        },
        "show_citations" | "show_cost" | "show_usage" => {
            if value != "true" && value != "false" {
                return Err(PplxError::Validation(format!(
                    "{key} must be 'true' or 'false', got '{value}'"
                )));
            }
        }
        _ => {} // model, api_key — any string is fine
    }

    let config_path = default_config_path()
        .ok_or_else(|| PplxError::Config("could not determine config directory".to_string()))?;

    // Read existing file or start empty
    let contents = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .map_err(|e| PplxError::Config(format!("failed to read config file: {e}")))?
    } else {
        String::new()
    };

    let mut doc: toml_edit::DocumentMut = contents
        .parse()
        .map_err(|e| PplxError::Config(format!("failed to parse config file: {e}")))?;

    // Map key to the correct TOML section and field
    match key {
        "api_key" => {
            if !doc.contains_table("auth") {
                doc["auth"] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            doc["auth"]["api_key"] = toml_edit::value(value);
        }
        "context_size" => {
            if !doc.contains_table("defaults") {
                doc["defaults"] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            doc["defaults"]["search_context_size"] = toml_edit::value(value);
        }
        "temperature" => {
            if !doc.contains_table("defaults") {
                doc["defaults"] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            let t: f64 = value.parse().unwrap();
            doc["defaults"]["temperature"] = toml_edit::value(t);
        }
        "top_p" => {
            if !doc.contains_table("defaults") {
                doc["defaults"] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            let p: f64 = value.parse().unwrap();
            doc["defaults"]["top_p"] = toml_edit::value(p);
        }
        "show_citations" | "show_cost" | "show_usage" => {
            if !doc.contains_table("defaults") {
                doc["defaults"] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            let b: bool = value.parse().unwrap();
            doc["defaults"][key] = toml_edit::value(b);
        }
        _ => {
            // model, output
            if !doc.contains_table("defaults") {
                doc["defaults"] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            doc["defaults"][key] = toml_edit::value(value);
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| PplxError::Config(format!("failed to create config directory: {e}")))?;
    }

    let mut file = std::fs::File::create(&config_path)
        .map_err(|e| PplxError::Config(format!("failed to write config file: {e}")))?;
    write!(file, "{doc}")
        .map_err(|e| PplxError::Config(format!("failed to write config file: {e}")))?;

    Ok(())
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

    // Validate temperature
    if let Some(t) = temperature {
        if !(0.0..=2.0).contains(&t) {
            return Err(PplxError::Validation(format!(
                "temperature must be between 0.0 and 2.0, got {t}"
            )));
        }
    }

    // Validate top_p
    let top_p = cli.top_p;
    if let Some(p) = top_p {
        if !(0.0..=1.0).contains(&p) {
            return Err(PplxError::Validation(format!(
                "top_p must be between 0.0 and 1.0, got {p}"
            )));
        }
    }

    // Validate output format (from file config; CLI enum is already validated by clap)
    match output_format.as_str() {
        "md" | "plain" | "json" | "raw" => {}
        other => {
            return Err(PplxError::Validation(format!(
                "output format must be one of: md, plain, json, raw — got '{other}'"
            )));
        }
    }

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
        max_tokens: cli.max_tokens,
        top_p,
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
        json_schema: cli.json_schema.clone(),
        save: cli.save.clone(),
        show_reasoning: cli.reasoning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_cli() -> Cli {
        Cli {
            query: vec![],
            command: None,
            model: None,
            output: None,
            system: None,
            citations: false,
            usage: false,
            cost: false,
            no_stream: false,
            no_color: false,
            verbose: false,
            config: None,
            search_mode: None,
            domains: vec![],
            exclude_domains: vec![],
            recency: None,
            after: None,
            before: None,
            updated_after: None,
            updated_before: None,
            context_size: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            reasoning_effort: None,
            reasoning: false,
            images: false,
            related: false,
            search_results: false,
            no_search: false,
            smart_search: false,
            json_schema: None,
            save: None,
        }
    }

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

    #[test]
    fn test_valid_temperature_boundary() {
        let mut cli = minimal_cli();
        cli.temperature = Some(0.0);
        let file = FileConfig::default();
        assert!(resolve(&cli, &file).is_ok());

        cli.temperature = Some(2.0);
        assert!(resolve(&cli, &file).is_ok());
    }

    #[test]
    fn test_invalid_temperature_too_high() {
        let mut cli = minimal_cli();
        cli.temperature = Some(2.5);
        let file = FileConfig::default();
        let err = resolve(&cli, &file).unwrap_err();
        assert!(err.to_string().contains("temperature"));
    }

    #[test]
    fn test_invalid_temperature_negative() {
        let mut cli = minimal_cli();
        cli.temperature = Some(-0.1);
        let file = FileConfig::default();
        let err = resolve(&cli, &file).unwrap_err();
        assert!(err.to_string().contains("temperature"));
    }

    #[test]
    fn test_valid_top_p_boundary() {
        let mut cli = minimal_cli();
        cli.top_p = Some(0.0);
        let file = FileConfig::default();
        assert!(resolve(&cli, &file).is_ok());

        cli.top_p = Some(1.0);
        assert!(resolve(&cli, &file).is_ok());
    }

    #[test]
    fn test_invalid_top_p() {
        let mut cli = minimal_cli();
        cli.top_p = Some(1.5);
        let file = FileConfig::default();
        let err = resolve(&cli, &file).unwrap_err();
        assert!(err.to_string().contains("top_p"));
    }

    #[test]
    fn test_invalid_output_format_from_file() {
        let cli = minimal_cli();
        let mut file = FileConfig::default();
        file.defaults.output = Some("xml".to_string());
        let err = resolve(&cli, &file).unwrap_err();
        assert!(err.to_string().contains("output format"));
    }

    #[test]
    fn test_set_config_unknown_key() {
        let err = set_config_value("nonexistent_key", "value").unwrap_err();
        assert!(err.to_string().contains("unknown config key"));
    }

    #[test]
    fn test_set_config_invalid_temperature() {
        let err = set_config_value("temperature", "5.0").unwrap_err();
        assert!(err.to_string().contains("temperature"));
    }

    #[test]
    fn test_set_config_invalid_output() {
        let err = set_config_value("output", "xml").unwrap_err();
        assert!(err.to_string().contains("output"));
    }
}

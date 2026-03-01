use serde::Deserialize;

/// Configuration loaded from TOML file. All fields optional.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FileConfig {
    pub auth: AuthConfig,
    pub defaults: DefaultsConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub api_key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct DefaultsConfig {
    pub model: Option<String>,
    pub output: Option<String>,
    pub temperature: Option<f32>,
    pub search_context_size: Option<String>,
    pub show_citations: Option<bool>,
    pub show_cost: Option<bool>,
    pub show_usage: Option<bool>,
    pub search: Option<SearchDefaults>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SearchDefaults {
    pub mode: Option<String>,
    pub recency: Option<String>,
    pub domains: Option<Vec<String>>,
    pub exclude_domains: Option<Vec<String>>,
}

/// Fully resolved configuration with concrete values.
#[derive(Debug)]
pub struct ResolvedConfig {
    pub api_key: String,
    pub model: String,
    pub output_format: String,
    pub show_citations: bool,
    pub show_cost: bool,
    pub show_usage: bool,
    pub temperature: Option<f32>,
    pub search_context_size: Option<String>,
    pub search_mode: Option<String>,
    pub search_recency: Option<String>,
    pub search_domains: Vec<String>,
    pub search_exclude_domains: Vec<String>,
    pub no_stream: bool,
    pub no_color: bool,
    #[allow(dead_code)]
    pub verbose: bool,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub reasoning_effort: Option<String>,
    pub images: bool,
    pub related: bool,
    pub search_results: bool,
    pub no_search: bool,
    pub smart_search: bool,
    pub system_prompt: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub updated_after: Option<String>,
    pub updated_before: Option<String>,
}

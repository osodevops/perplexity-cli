use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "pplx",
    about = "A powerful Perplexity API client for the terminal",
    version,
    after_help = "Examples:\n  pplx \"What is Rust?\"\n  pplx ask -m sonar-pro \"Explain quantum computing\"\n  pplx ask --no-stream -o json \"test\" | jq .\n  echo \"summarize this\" | pplx ask\n  pplx completions zsh"
)]
pub struct Cli {
    /// Query to send (implicit 'ask' command)
    #[arg(trailing_var_arg = true)]
    pub query: Vec<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    // --- Global options ---
    /// Model to use
    #[arg(short, long, global = true, env = "PPLX_MODEL")]
    pub model: Option<String>,

    /// Output format
    #[arg(short, long, global = true, env = "PPLX_OUTPUT", value_enum)]
    pub output: Option<OutputFormat>,

    /// System prompt
    #[arg(short, long, global = true)]
    pub system: Option<String>,

    /// Show citations
    #[arg(short, long, global = true)]
    pub citations: bool,

    /// Show token usage
    #[arg(short, long, global = true)]
    pub usage: bool,

    /// Show cost breakdown
    #[arg(long, global = true)]
    pub cost: bool,

    /// Disable streaming
    #[arg(long, global = true)]
    pub no_stream: bool,

    /// Disable colour output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Verbose/debug output
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Config file path override
    #[arg(long, global = true)]
    pub config: Option<String>,

    // --- Search filters ---
    /// Search mode
    #[arg(long, global = true, value_enum)]
    pub search_mode: Option<SearchMode>,

    /// Include domain (repeatable)
    #[arg(long = "domain", global = true)]
    pub domains: Vec<String>,

    /// Exclude domain (repeatable)
    #[arg(long = "exclude-domain", global = true)]
    pub exclude_domains: Vec<String>,

    /// Recency filter
    #[arg(long, global = true, value_enum)]
    pub recency: Option<RecencyFilter>,

    /// Results after date (MM/DD/YYYY)
    #[arg(long, global = true)]
    pub after: Option<String>,

    /// Results before date (MM/DD/YYYY)
    #[arg(long, global = true)]
    pub before: Option<String>,

    /// Last updated after date (MM/DD/YYYY)
    #[arg(long, global = true)]
    pub updated_after: Option<String>,

    /// Last updated before date (MM/DD/YYYY)
    #[arg(long, global = true)]
    pub updated_before: Option<String>,

    /// Search context size
    #[arg(long, global = true, env = "PPLX_CONTEXT_SIZE", value_enum)]
    pub context_size: Option<ContextSize>,

    // --- Model parameters ---
    /// Maximum response tokens
    #[arg(long, global = true)]
    pub max_tokens: Option<u32>,

    /// Temperature (0.0-2.0)
    #[arg(short, long, global = true)]
    pub temperature: Option<f32>,

    /// Nucleus sampling (0.0-1.0)
    #[arg(long, global = true)]
    pub top_p: Option<f32>,

    /// Reasoning effort level
    #[arg(long, global = true, value_enum)]
    pub reasoning_effort: Option<ReasoningEffort>,

    // --- Response enrichment ---
    /// Return image URLs
    #[arg(long, global = true)]
    pub images: bool,

    /// Return related questions
    #[arg(long, global = true)]
    pub related: bool,

    /// Show full search result metadata
    #[arg(long, global = true)]
    pub search_results: bool,

    /// Disable web search
    #[arg(long, global = true)]
    pub no_search: bool,

    /// Enable search classifier
    #[arg(long, global = true)]
    pub smart_search: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Send a query to Perplexity
    Ask {
        /// The query to ask
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },

    /// Raw web search via Search API
    Search {
        /// The search query
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },

    /// Deep research with async support
    Research {
        /// The research query
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },

    /// Use Agent API with third-party models
    Agent {
        /// The query
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },

    /// Start interactive REPL session
    Interactive,

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Create default config file with API key
    Init,
    /// Show current effective configuration
    Show,
    /// Set a configuration value
    Set {
        /// Config key (e.g. model, search.recency)
        key: String,
        /// Value to set
        value: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Rendered markdown with colours
    Md,
    /// Plain text, no formatting
    Plain,
    /// Full API response as JSON
    Json,
    /// Raw response content only
    Raw,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SearchMode {
    Web,
    Academic,
    Sec,
}

impl SearchMode {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            SearchMode::Web => "web",
            SearchMode::Academic => "academic",
            SearchMode::Sec => "sec",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RecencyFilter {
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl RecencyFilter {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            RecencyFilter::Hour => "hour",
            RecencyFilter::Day => "day",
            RecencyFilter::Week => "week",
            RecencyFilter::Month => "month",
            RecencyFilter::Year => "year",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ContextSize {
    Minimal,
    Low,
    Medium,
    High,
}

impl ContextSize {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            ContextSize::Minimal => "minimal",
            ContextSize::Low => "low",
            ContextSize::Medium => "medium",
            ContextSize::High => "high",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            ReasoningEffort::Minimal => "minimal",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
        }
    }
}

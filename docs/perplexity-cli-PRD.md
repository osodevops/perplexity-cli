# Product Requirements Document: `pplx` — A Rust-Based Perplexity CLI

**Version:** 1.0  
**Date:** March 2026  
**Author:** [Your Name]  
**Status:** Draft  

***

## Executive Summary

`pplx` is a high-performance, feature-rich command-line interface for the Perplexity API, built in Rust. It targets developers, researchers, and power users who prefer terminal-first workflows and want full access to every Perplexity API capability — including the Sonar Chat Completions API, the Search API, the Async Deep Research API, and the Agent API — from a single binary with zero runtime dependencies.[^1][^2]

The existing `perplexity-cli` by dawid-szewc is a minimal Python script supporting basic querying, model selection, token stats, citations, and Glow formatting. It lacks streaming, search filters, structured output, interactive mode, config files, cost tracking, pipe support, and coverage of the Search and Agent APIs. It also suffers from Python dependency issues (e.g., `ModuleNotFoundError: requests`) and limited maintenance. Community feedback reveals frustration with API results being worse than the web UI, unreliable domain filters, citation mapping bugs, deep research timeouts, and context loss in multi-turn conversations. `pplx` addresses all of these gaps.[^3][^4][^5][^6]

***

## Goals and Non-Goals

### Goals

- Provide a single, statically-linked Rust binary with zero runtime dependencies
- Support 100% of Perplexity's current API surface: Sonar Chat Completions, Search API, Async Deep Research, and Agent API[^7][^8][^1]
- Real-time streaming with rendered markdown output in the terminal
- Interactive (REPL) mode with multi-turn conversation history
- First-class support for all search filters (domain, recency, date range, mode, context size)[^9][^10][^11]
- Structured output (JSON schema) support for programmatic use[^12]
- Pipe-friendly: accept queries from stdin, output clean text/JSON to stdout
- Per-query cost tracking and cumulative session cost reporting[^2]
- Persistent, layered configuration (global config file → environment variables → CLI flags)
- Cross-platform: Linux, macOS, Windows

### Non-Goals

- GUI or TUI dashboard (this is a CLI tool, not a TUI app)
- Acting as an API proxy or server
- Supporting non-Perplexity LLM providers
- Caching or offline mode
- Plugin/extension system (v1)

***

## Competitive Analysis

| Feature | dawid-szewc/perplexity-cli | cloudxabide/perplexity-cli | Perplexity MCP Server | **pplx (this project)** |
|---|---|---|---|---|
| Language | Python | Python | Node.js / Go | **Rust** |
| Streaming | ❌ | ❌ | ✅ | **✅** |
| Interactive REPL | ❌ | ❌ | N/A | **✅** |
| Search filters | ❌ | ❌ | Partial | **Full** |
| Structured output | ❌ | ❌ | ❌ | **✅** |
| Search API | ❌ | ❌ | ✅ | **✅** |
| Async deep research | ❌ | ❌ | ✅ | **✅** |
| Agent API | ❌ | ❌ | ❌ | **✅** |
| Cost tracking | ❌ | ❌ | ❌ | **✅** |
| Config file | ❌ | ✅ | Via env | **✅** |
| Pipe support | ❌ | ❌ | N/A | **✅** |
| Markdown rendering | Via Glow (external) | ❌ | N/A | **Built-in** |
| Zero deps install | ❌ (Python + pip) | ❌ (Python + pip) | ❌ (Node/Docker) | **✅ (single binary)** |
| Reasoning display | ❌ | ❌ | ✅ | **✅** |

Sources:[^13][^14][^15][^3]

***

## User Personas

### Developer (Primary)

A software engineer who lives in the terminal, uses tools like `curl`, `jq`, and `ripgrep` daily. Wants to quickly query Perplexity for technical answers, pipe output to other tools, and integrate searches into shell scripts. Values speed, streaming, and machine-readable output.

### Researcher (Secondary)

An academic or analyst who needs deep research capabilities with domain-filtered, date-constrained searches. Wants citation URLs, structured JSON output for downstream processing, and the ability to kick off long-running deep research jobs asynchronously.

### Power User (Tertiary)

A Perplexity Pro subscriber who prefers the terminal over the web UI. Wants an interactive REPL with conversation history, markdown rendering, image references, related questions, and full control over all API parameters.

***

## Architecture Overview

### Technology Stack

| Component | Crate | Purpose |
|---|---|---|
| HTTP client | `reqwest` (with `stream` feature) | API requests, SSE streaming |
| SSE parsing | `reqwest-sse` or `eventsource-client` | Server-Sent Events for streaming[^16][^17] |
| Async runtime | `tokio` | Async I/O, task spawning, channels |
| CLI parsing | `clap` (derive mode) | Argument parsing, subcommands, completions |
| JSON handling | `serde` + `serde_json` | Serialization/deserialization |
| Config | `toml` + `dirs` | Config file parsing + XDG paths |
| Markdown rendering | `termimad` | Terminal markdown rendering with colour |
| Syntax highlighting | `syntect` | Code block highlighting in responses |
| Spinner/progress | `indicatif` | Loading spinners, progress for async jobs |
| Colour | `crossterm` + `owo-colors` | Cross-platform terminal colours |
| Keyring | `keyring` | Secure API key storage (optional) |

### Module Structure

```
src/
├── main.rs              # Entry point, CLI dispatch
├── cli/
│   ├── mod.rs           # Clap app definition
│   ├── args.rs          # Argument structs & validation
│   └── completions.rs   # Shell completion generation
├── api/
│   ├── mod.rs           # Shared HTTP client, auth
│   ├── chat.rs          # POST /chat/completions (sync + stream)
│   ├── search.rs        # POST /search
│   ├── async_research.rs# POST/GET /async/chat/completions
│   ├── agent.rs         # POST /responses (Agent API)
│   └── types.rs         # Request/response type definitions
├── config/
│   ├── mod.rs           # Config loading & merging
│   └── types.rs         # Config struct definitions
├── output/
│   ├── mod.rs           # Output dispatcher
│   ├── markdown.rs      # Markdown terminal rendering
│   ├── json.rs          # JSON output formatting
│   ├── plain.rs         # Plain text output
│   └── citations.rs     # Citation formatting
├── interactive/
│   ├── mod.rs           # REPL loop
│   ├── history.rs       # Conversation history management
│   └── commands.rs      # REPL slash commands
├── cost/
│   └── mod.rs           # Cost calculation & tracking
└── error.rs             # Error types
```

***

## Detailed Feature Specification

### F1: Core Query (Chat Completions API)

**Endpoint:** `POST https://api.perplexity.ai/chat/completions`[^2]

The primary command. Send a query and receive a web-grounded AI response.

```bash
# Basic query
pplx ask "What is the current population of London?"

# With model selection
pplx ask -m sonar-pro "Explain quantum entanglement"

# With system prompt
pplx ask -s "You are a concise technical writer" "Explain gRPC vs REST"

# Short form (ask is the default subcommand)
pplx "What is Rust's borrow checker?"
```

**Supported API Parameters (all exposed as CLI flags):**

| CLI Flag | API Parameter | Type | Default |
|---|---|---|---|
| `-m, --model` | `model` | enum | `sonar-pro` |
| `-s, --system` | `messages.role=system` | string | none |
| `--max-tokens` | `max_tokens` | integer | none (API default) |
| `-t, --temperature` | `temperature` | float 0.0-2.0 | none (API default) |
| `--top-p` | `top_p` | float 0.0-1.0 | none (API default) |
| `--no-stream` | `stream: false` | flag | streaming ON |
| `--search-mode` | `search_mode` | web/academic/sec | web |
| `--domain` | `search_domain_filter` | string (repeatable) | none |
| `--exclude-domain` | `search_domain_filter` (with `-` prefix) | string (repeatable) | none |
| `--recency` | `search_recency_filter` | hour/day/week/month/year | none |
| `--after` | `search_after_date_filter` | date (MM/DD/YYYY) | none |
| `--before` | `search_before_date_filter` | date (MM/DD/YYYY) | none |
| `--updated-after` | `last_updated_after_filter` | date (MM/DD/YYYY) | none |
| `--updated-before` | `last_updated_before_filter` | date (MM/DD/YYYY) | none |
| `--context-size` | `search_context_size` | minimal/low/medium/high | low |
| `--reasoning-effort` | `reasoning_effort` | minimal/low/medium/high | none |
| `--images` | `return_images` | flag | false |
| `--related` | `return_related_questions` | flag | false |
| `--json-schema` | `response_format` | file path or inline JSON | none |
| `--no-search` | `disable_search` | flag | false |
| `--smart-search` | `enable_search_classifier` | flag | false |

**Available Models:**[^18][^2]

| Model | Description | Best For |
|---|---|---|
| `sonar` | Fast, lightweight (Llama 3.3 70B based, 1200 tok/s) | Quick factual lookups |
| `sonar-pro` | Deeper multi-step search, larger context (200K) | Complex queries, follow-ups |
| `sonar-reasoning-pro` | Chain-of-thought reasoning, `<think>` tags | Multi-step analysis, math |
| `sonar-deep-research` | Autonomous multi-search research (128K) | Comprehensive reports |

Sources:[^19][^7][^18]

***

### F2: Real-Time Streaming

Streaming is enabled by default for all queries. Response tokens are rendered to the terminal as they arrive via Server-Sent Events (SSE).[^12]

**Behaviour:**

1. On query dispatch, display a spinner ("Searching...")
2. Replace spinner with streamed content as first token arrives
3. Render markdown incrementally using `termimad`
4. On stream completion, append citations, images, related questions, and cost summary below the response
5. Search results and usage metadata arrive in the final SSE chunk(s)[^12]

**Implementation notes:**

- Use `reqwest` with streaming body and parse SSE via `reqwest-sse` or `eventsource-client`[^16][^17]
- Each SSE `data:` line contains a JSON chunk following the OpenAI streaming format[^1]
- The `[DONE]` sentinel signals end of stream
- For `sonar-reasoning-pro`, parse and display `<think>` blocks in a distinct style (dimmed/italic) before the main response[^18]

***

### F3: Interactive Mode (REPL)

Launch an interactive session with multi-turn conversation history.

```bash
pplx interactive
pplx -i  # short form
```

**REPL features:**

- Persistent conversation history (messages array sent with each request)
- Readline-style editing (via `rustyline` or similar)
- Slash commands for in-session control:

| Command | Action |
|---|---|
| `/model <name>` | Switch model mid-conversation |
| `/system <prompt>` | Set/update system prompt |
| `/clear` | Clear conversation history |
| `/history` | Show conversation turns |
| `/cost` | Show session cost summary |
| `/domain <add\|remove\|clear> [domain]` | Manage domain filters |
| `/recency <value>` | Set recency filter |
| `/mode <web\|academic\|sec>` | Switch search mode |
| `/context <minimal\|low\|medium\|high>` | Set search context size |
| `/export [file]` | Export conversation to markdown/JSON |
| `/help` | Show available commands |
| `/quit` or Ctrl+D | Exit |

**Context management:** Conversation history is maintained as an array of `{role, content}` messages. If the conversation approaches the model's context limit, display a warning and suggest `/clear`. This directly addresses user complaints about context loss in API conversations.[^5]

***

### F4: Search API

**Endpoint:** `POST https://api.perplexity.ai/search`[^20][^21]

Raw web search results without LLM synthesis. Useful for programmatic search, data pipelines, and when you need source material without AI interpretation.

```bash
# Basic search
pplx search "Rust async runtime benchmarks"

# Multi-query search
pplx search "Kafka 4.0 features" "Kafka KRaft migration guide"

# With filters
pplx search --domain arxiv.org --recency month "transformer architecture improvements"

# Academic mode
pplx search --mode academic "CRISPR gene therapy clinical trials 2025"

# SEC filings mode
pplx search --mode sec "Apple 10-K 2025"

# Output as JSON for piping
pplx search -o json "latest Kubernetes CVEs" | jq '.results[].url'
```

**Parameters:**

| CLI Flag | API Parameter | Type |
|---|---|---|
| `--max-results` | `max_results` | integer |
| `--max-tokens` | `max_tokens` | integer |
| `--max-tokens-per-page` | `max_tokens_per_page` | integer |
| `--mode` | `search_mode` | web/academic/sec |
| `--domain` | `search_domain_filter` | string (repeatable) |
| `--recency` | `search_recency_filter` | hour/day/week/month/year |
| `--after` | `search_after_date_filter` | date |
| `--before` | `search_before_date_filter` | date |

**Pricing:** $5 per 1,000 requests, no token costs.[^7]

***

### F5: Async Deep Research

**Endpoints:**[^22][^23]
- `POST /async/chat/completions` — Submit research job
- `GET /async/chat/completions` — List all jobs
- `GET /async/chat/completions/{request_id}` — Get job status/result

For complex research queries that may take minutes to process. Submit and poll or wait.

```bash
# Submit and wait (blocking with progress spinner)
pplx research "Comprehensive analysis of Rust vs Go for distributed systems in 2026"

# Submit and return immediately (get job ID)
pplx research --async "Full competitive analysis of the API gateway market"
# Output: Job submitted: req_abc123. Check status with: pplx research status req_abc123

# Check status
pplx research status req_abc123

# List all jobs
pplx research list

# Get result
pplx research get req_abc123

# With reasoning effort control
pplx research --reasoning-effort high "Detailed analysis of EU AI Act implications"
```

**Behaviour:**

- Default mode: submit job, display a progress spinner, poll every 5 seconds, render result when complete
- `--async` flag: submit and return immediately, print job ID
- Results have a 7-day TTL[^23]
- Display full cost breakdown for deep research (input + output + citation + reasoning + search query costs)[^7]

***

### F6: Agent API

**Endpoint:** `POST https://api.perplexity.ai/responses`[^8][^24]

Access third-party models (OpenAI, Anthropic, Google, xAI) with optional tool use (web_search, fetch_url, custom functions).

```bash
# Use a third-party model
pplx agent -m openai/gpt-5.2 "Explain the CAP theorem"

# With web search tool enabled
pplx agent -m openai/gpt-5.2 --tool web_search "What happened in tech news today?"

# With URL fetching
pplx agent -m anthropic/claude-4-sonnet --tool fetch_url "Summarize https://example.com/article"

# With domain-filtered search
pplx agent -m openai/gpt-5.2 --tool web_search --domain nature.com --recency month \
  "Recent findings on neuroplasticity"

# Structured output
pplx agent -m openai/gpt-5.2 --json-schema schema.json "Extract company data from latest Apple 10-K"
```

**Tool pricing:** `web_search` at $0.005/invocation, `fetch_url` at $0.0005/invocation.[^7]

***

### F7: Output Modes

`pplx` supports multiple output formats controlled by the `-o` / `--output` flag:

| Mode | Flag | Description |
|---|---|---|
| **Markdown** (default) | `-o md` | Rendered markdown with syntax highlighting and colours in terminal |
| **Plain** | `-o plain` | Clean text, no formatting (good for piping) |
| **JSON** | `-o json` | Full API response as JSON (for `jq`, scripts) |
| **Raw** | `-o raw` | Raw response content only, no metadata |
| **Glow** | `-o glow` | Glow-compatible markdown (for glow rendering) |

**Additional output controls:**

| Flag | Description |
|---|---|
| `--citations` / `-c` | Show numbered citation URLs below response |
| `--search-results` | Show full search result metadata (title, URL, date, snippet) |
| `--usage` / `-u` | Show token usage statistics |
| `--cost` | Show cost breakdown for the request |
| `--images` | Show image URLs returned by the API |
| `--related` | Show related questions |
| `--reasoning` | Show `<think>` reasoning blocks (for reasoning models) |
| `--save <file>` | Save response to file (auto-detects format from extension) |
| `--no-color` | Disable colour output |

***

### F8: Pipe and Scripting Support

`pplx` is designed to be a first-class Unix citizen.

```bash
# Read query from stdin
echo "What is Rust?" | pplx ask

# Pipe file content as context
cat error.log | pplx ask "What's causing this error?"

# Chain with jq
pplx ask -o json "Top 5 Rust web frameworks" | jq '.citations'

# Use in scripts
ANSWER=$(pplx ask -o raw "Current UTC time")

# Multi-query from file
cat questions.txt | while read q; do pplx ask -o raw "$q"; done

# Search and extract URLs
pplx search -o json "Rust async patterns" | jq -r '.results[].url'
```

**stdin behaviour:**

- If stdin is a pipe (not a TTY), read input as the query or append to query
- If both stdin and positional argument provided, concatenate: `pplx ask "Summarize this:" < article.txt`
- In pipe mode, disable spinners, colours, and interactive features automatically

***

### F9: Configuration System

**Config file location:** `$XDG_CONFIG_HOME/pplx/config.toml` (default `~/.config/pplx/config.toml`)

**Precedence (highest to lowest):**
1. CLI flags
2. Environment variables (`PPLX_*`)
3. Config file
4. Built-in defaults

**Sample `config.toml`:**

```toml
[auth]
api_key = "pplx-xxxxxxxxxxxx"  # Or use PERPLEXITY_API_KEY env var

[defaults]
model = "sonar-pro"
output = "md"
temperature = 0.7
search_context_size = "medium"
show_citations = true
show_cost = false
show_usage = false

[defaults.search]
mode = "web"
recency = "month"
domains = []
exclude_domains = ["pinterest.com", "quora.com"]

[interactive]
history_file = "~/.local/share/pplx/history"
max_history = 1000

[rendering]
theme = "auto"  # auto, dark, light
syntax_theme = "base16-ocean.dark"
max_width = 100
```

**Environment variable mapping:**

| Env Var | Config Equivalent |
|---|---|
| `PERPLEXITY_API_KEY` | `auth.api_key` |
| `PPLX_MODEL` | `defaults.model` |
| `PPLX_OUTPUT` | `defaults.output` |
| `PPLX_CONTEXT_SIZE` | `defaults.search_context_size` |
| `PPLX_SEARCH_MODE` | `defaults.search.mode` |

**Config management commands:**

```bash
pplx config init          # Create default config file
pplx config show          # Print current effective config
pplx config set model sonar-pro
pplx config set search.recency week
```

***

### F10: Cost Tracking

The Perplexity API response includes detailed cost information in the `usage` field. `pplx` parses and displays this.[^2]

**Per-request cost display (when `--cost` flag is set):**

```
Cost: $0.0057
  Input tokens:  500 ($0.0005)
  Output tokens: 200 ($0.0002)
  Request fee:         ($0.005)
```

**Deep research cost display:**

```
Cost: $0.4094
  Input tokens:     33 ($0.000066)
  Output tokens:  7163 ($0.057304)
  Citation tokens: 20016 ($0.040032)
  Reasoning tokens: 73997 ($0.221991)
  Search queries:    18 ($0.09)
```

**Session cost tracking (interactive mode):**

- Running total displayed via `/cost` command
- Summary on session exit

**Pricing reference per model:**[^7]

| Model | Input $/1M | Output $/1M | Request Fee (low/med/high) |
|---|---|---|---|
| Sonar | $1 | $1 | $5/$5/$5 per 1K |
| Sonar Pro | $3 | $15 | $5/$5/$5 per 1K |
| Sonar Reasoning Pro | $2 | $8 | $6/$10/$14 per 1K |
| Sonar Deep Research | $2 | $8 | + $2 citation + $5/1K searches + $3 reasoning per 1M |

***

### F11: Shell Completions

Generate shell completion scripts for all major shells.

```bash
pplx completions bash > ~/.local/share/bash-completion/completions/pplx
pplx completions zsh > ~/.zsh/completions/_pplx
pplx completions fish > ~/.config/fish/completions/pplx.fish
pplx completions powershell > pplx.ps1
```

Completions include: subcommands, flags, model names, search modes, recency values, context sizes, and reasoning effort levels. Use `clap_complete` for generation.

***

### F12: Markdown Rendering

Built-in terminal markdown rendering using `termimad`:

- **Headers** rendered with bold and colour differentiation
- **Code blocks** with syntax highlighting via `syntect` (language auto-detection)
- **Inline code** with distinct background
- **Bold**, *italic*, ~~strikethrough~~ support
- **Lists** (ordered and unordered) with proper indentation
- **Tables** rendered with Unicode box-drawing characters
- **Links** shown inline with URL in dimmed text
- **Block quotes** with left border
- Respects terminal width (configurable `max_width`)
- Automatic dark/light theme detection

***

## Error Handling

| Error | Behaviour |
|---|---|
| Missing API key | Clear message: "API key not found. Set PERPLEXITY_API_KEY or run `pplx config init`" |
| 401 Unauthorized | "Invalid API key. Check your key at https://www.perplexity.ai/settings/api" |
| 429 Rate Limited | Automatic exponential backoff retry (3 attempts), then show rate limit reset time |
| 500/502/503 Server Error | Retry with backoff (3 attempts), then display error with suggestion to check status.perplexity.ai |
| Network timeout | Configurable timeout (default 30s, deep research 300s), clear timeout message |
| Invalid model name | Show error with list of valid models |
| Invalid date format | Show expected format: "Date must be MM/DD/YYYY, got: [input]" |
| Invalid JSON schema file | Parse error with line/column information |
| Deep research stuck | For async: show status and elapsed time. For blocking: timeout after configurable period |
| Streaming interrupted | Partial response displayed with "[Response interrupted]" message |

Known API issues to handle gracefully: domain filter unreliability, citation mapping inconsistencies, and deep research IN_PROGRESS timeouts.[^4][^25]

***

## Installation and Distribution

### Installation Methods

```bash
# Homebrew (macOS/Linux)
brew install pplx

# Cargo (Rust toolchain)
cargo install pplx

# Pre-built binaries (GitHub Releases)
curl -sSL https://github.com/<org>/pplx/releases/latest/download/pplx-$(uname -s)-$(uname -m).tar.gz | tar xz
sudo mv pplx /usr/local/bin/

# Nix
nix profile install github:<org>/pplx

# AUR (Arch Linux)
yay -S pplx
```

### Build Targets

| Target | Architecture |
|---|---|
| `x86_64-unknown-linux-gnu` | Linux x86_64 |
| `x86_64-unknown-linux-musl` | Linux x86_64 (static) |
| `aarch64-unknown-linux-gnu` | Linux ARM64 |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows x86_64 |

CI/CD via GitHub Actions: build matrix for all targets, cross-compilation using `cross`, release binaries on tag push.

***

## Testing Strategy

| Layer | Approach | Tools |
|---|---|---|
| Unit tests | Type serialization/deserialization, config merging, cost calculation, date validation, citation parsing | `cargo test`, `serde_test` |
| Integration tests | Full CLI invocation with mock HTTP server | `assert_cmd`, `predicates`, `wiremock` |
| API contract tests | Validate request/response shapes against Perplexity API docs | `wiremock` with recorded fixtures |
| Streaming tests | Verify SSE parsing, incremental rendering, `[DONE]` handling | Custom SSE mock server |
| E2E tests (optional) | Real API calls with test key (CI only, gated) | `cargo test --features e2e` |
| Fuzz testing | Malformed SSE streams, invalid JSON, extreme inputs | `cargo-fuzz` |

***

## Development Milestones

### Phase 1: Core MVP (Weeks 1–3)

- Project scaffolding: Cargo workspace, CI/CD, linting
- Config system: TOML config, env vars, CLI flag precedence
- Auth: API key from config/env/flag, secure handling
- `pplx ask` command: basic query, all models, system prompt
- Streaming: SSE parsing, incremental markdown rendering
- Output modes: markdown (rendered), plain, JSON, raw
- Citations display
- Token usage display
- Cost display
- Error handling with retries
- Shell completions

### Phase 2: Search & Filters (Weeks 3–4)

- All search filter flags: domain, recency, date range, mode, context size
- `pplx search` command: Search API integration
- Structured output: `--json-schema` flag
- Reasoning effort parameter
- `--images` and `--related` flags
- Pipe/stdin support
- `--save` to file

### Phase 3: Advanced Features (Weeks 4–6)

- `pplx interactive` REPL mode with conversation history
- Slash commands for in-session control
- `pplx research` command: async deep research with polling
- `pplx agent` command: Agent API with tool support
- Session cost tracking
- `pplx config` management subcommand
- Reasoning/think block rendering for reasoning models

### Phase 4: Polish & Distribution (Weeks 6–8)

- Homebrew formula
- AUR package
- Nix flake
- Cross-platform pre-built binaries
- Comprehensive test suite
- Man page generation
- README with examples and GIFs
- `--version`, `--help` polish
- Performance benchmarking vs Python alternatives

***

## Acceptance Criteria

1. `pplx ask "query"` returns a streamed, markdown-rendered response in under 500ms to first token (network permitting)
2. All Perplexity API parameters are accessible via CLI flags and/or config
3. `echo "query" | pplx ask -o json | jq .citations` works correctly (pipe support)
4. `pplx interactive` maintains conversation context across 10+ turns without context loss
5. `pplx research` successfully submits, polls, and retrieves async deep research results
6. `pplx search` returns raw search results with domain and date filtering
7. `pplx agent` works with at least OpenAI and Anthropic models via the Agent API
8. Cost tracking matches the costs reported in the API response `usage.cost` field
9. Single binary with zero runtime dependencies, under 15MB compressed
10. Works on Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), and Windows
11. Shell completions work for bash, zsh, fish, and PowerShell
12. All error conditions produce clear, actionable messages
13. `--no-stream` mode works for all commands that support streaming
14. Config file, env vars, and CLI flags merge correctly with documented precedence

***

## Open Questions

1. **MCP Server mode:** Should `pplx` also function as an MCP (Model Context Protocol) server for integration with Claude Code, Cursor, etc.? This would be a natural extension but increases scope.
2. **Conversation persistence:** Should interactive mode conversations be saved to disk and resumable across sessions?
3. **Embedding support:** The Perplexity Embeddings API exists. Should `pplx embed` be a subcommand?[^7]
4. **Update notifications:** Should the binary check for new versions periodically?
5. **Proxy support:** Should HTTP/SOCKS proxy configuration be explicitly supported beyond env vars (`HTTP_PROXY`)?

***

## Appendix A: Full CLI Reference

```
pplx — A powerful Perplexity API client for the terminal

USAGE:
    pplx [OPTIONS] [QUERY]
    pplx <COMMAND> [OPTIONS] [ARGS]

COMMANDS:
    ask           Send a query (default if no subcommand given)
    search        Raw web search via Search API
    research      Deep research with async support
    agent         Use Agent API with third-party models
    interactive   Start interactive REPL session
    config        Manage configuration
    completions   Generate shell completions

GLOBAL OPTIONS:
    -m, --model <MODEL>         Model to use [default: sonar-pro]
    -o, --output <FORMAT>       Output format: md, plain, json, raw, glow
    -s, --system <PROMPT>       System prompt
    -c, --citations             Show citations
    -u, --usage                 Show token usage
    --cost                      Show cost breakdown
    --no-stream                 Disable streaming
    --no-color                  Disable colour output
    --verbose                   Verbose/debug output
    --config <PATH>             Config file path override
    -h, --help                  Show help
    -V, --version               Show version

SEARCH FILTERS:
    --search-mode <MODE>        Search mode: web, academic, sec
    --domain <DOMAIN>           Include domain (repeatable)
    --exclude-domain <DOMAIN>   Exclude domain (repeatable)
    --recency <PERIOD>          Recency filter: hour, day, week, month, year
    --after <DATE>              Results after date (MM/DD/YYYY)
    --before <DATE>             Results before date (MM/DD/YYYY)
    --updated-after <DATE>      Last updated after date (MM/DD/YYYY)
    --updated-before <DATE>     Last updated before date (MM/DD/YYYY)
    --context-size <SIZE>       Search context: minimal, low, medium, high

MODEL PARAMETERS:
    --max-tokens <N>            Maximum response tokens
    -t, --temperature <FLOAT>   Temperature (0.0-2.0)
    --top-p <FLOAT>             Nucleus sampling (0.0-1.0)
    --reasoning-effort <LEVEL>  Reasoning effort: minimal, low, medium, high
    --json-schema <FILE|JSON>   Enable structured JSON output

RESPONSE ENRICHMENT:
    --images                    Return image URLs
    --related                   Return related questions
    --reasoning                 Show reasoning/think blocks
    --search-results            Show full search result metadata

OUTPUT:
    --save <FILE>               Save response to file
```

## Appendix B: API Response Type Definitions (Rust)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_recency_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_after_date_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_before_date_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated_after_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated_before_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_related_questions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_search_classifier: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<JsonSchema>,
}

#[derive(Debug, Serialize)]
pub struct JsonSchema {
    pub schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub model: String,
    pub created: u64,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
    pub citations: Option<Vec<String>>,
    pub search_results: Option<Vec<SearchResult>>,
    pub images: Option<Vec<ImageResult>>,
    pub related_questions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Option<Message>,
    pub delta: Option<Delta>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost: Option<Cost>,
    pub search_context_size: Option<String>,
    pub citation_tokens: Option<u32>,
    pub num_search_queries: Option<u32>,
    pub reasoning_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Cost {
    pub input_tokens_cost: Option<f64>,
    pub output_tokens_cost: Option<f64>,
    pub total_cost: Option<f64>,
    pub reasoning_tokens_cost: Option<f64>,
    pub request_cost: Option<f64>,
    pub citation_tokens_cost: Option<f64>,
    pub search_queries_cost: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub date: Option<String>,
    pub last_updated: Option<String>,
    pub snippet: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImageResult {
    pub image_url: String,
    pub origin_url: Option<String>,
    pub title: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
```

## Appendix C: Licence

MIT License.

---

## References

1. [Sonar API - Perplexity](https://docs.perplexity.ai/docs/sonar/quickstart) - Overview. Perplexity's Sonar API provides web-grounded AI responses with support for streaming, tool...

2. [Create Chat Completion - Perplexity](https://docs.perplexity.ai/api-reference/chat-completions-post) - Generate a chat completion response for the given conversation.

3. [GitHub - lucabased/perplexity-cli-lucabased: [THIS IS A FORK] 🧠 A simple command-line client for the Perplexity API. Ask questions and receive answers directly from the terminal! 🚀🚀🚀](https://github.com/lucabased/perplexity-cli-lucabased) - [THIS IS A FORK] 🧠 A simple command-line client for the Perplexity API. Ask questions and receive an...

4. [Bug Reports - Perplexity API Platform Forum](https://community.perplexity.ai/c/bug-reports/10) - Purpose: Have a question? You're in the right place! Ask anything about the API and get help from st...

5. [Is it just me, or do Perplexity's API models are... bad?](https://www.reddit.com/r/perplexity_ai/comments/1m4kddv/is_it_just_me_or_do_perplexitys_api_models_are_bad/) - Is it just me, or do Perplexity's API models are... bad?

6. [Quality of Perplexity Pro has seriously taken a nose dive!](https://www.reddit.com/r/perplexity_ai/comments/1fvl54y/quality_of_perplexity_pro_has_seriously_taken_a/) - Quality of Perplexity Pro has seriously taken a nose dive!

7. [Pricing - Perplexity](https://docs.perplexity.ai/docs/getting-started/pricing) - Token pricing remains the same as standard Sonar Pro ($3 per 1M input, $15 per 1M output).

8. [Agent API - Perplexity](https://docs.perplexity.ai/docs/agent-api/quickstart) - The Agent API is a multi-provider, interoperable API specification for building LLM applications. Ac...

9. [Search Domain Filter - Perplexity](https://docs.perplexity.ai/docs/search/filters/domain-filter)

10. [OpenAI SDK Compatibility - Perplexity](https://docs.perplexity.ai/docs/sonar/openai-compatibility) - search_recency_filter - Filter by content recency (“day”, “week”, “month”, “year”); return_images - ...

11. [Search Date and Time Filters - Perplexity](https://docs.perplexity.ai/docs/search/filters/date-time-filters) - The search_after_date_filter and search_before_date_filter parameters allow you to restrict search r...

12. [Core Features - Perplexity](https://docs.perplexity.ai/docs/sonar/features) - This guide covers three core capabilities: streaming responses for real-time experiences, structured...

13. [GitHub - cloudxabide/perplexity-cli: Perplexity AI CLI Client written in Python by Amazon Q](https://github.com/cloudxabide/perplexity-cli) - Perplexity AI CLI Client written in Python by Amazon Q - cloudxabide/perplexity-cli

14. [perplexity-mcp/main.go at main · Alcova-AI/perplexity-mcp - GitHub](https://github.com/Alcova-AI/perplexity-mcp/blob/main/main.go) - An MCP server for the Perplexity for use with Claude Code and Claude Desktop, giving you enhanced se...

15. [MekayelAnik/perplexity-mcp-docker - GitHub](https://github.com/MekayelAnik/perplexity-mcp-docker) - Perplexity MCP Server brings real-time web search, advanced reasoning, and deep research capabilitie...

16. [Crate reqwest_sseCopy item path](https://docs.rs/reqwest-sse/latest/reqwest_sse/) - `reqwest-sse`

17. [GitHub - launchdarkly/rust-eventsource-client: Server-sent events (SSE) client implementation for Rust](https://github.com/launchdarkly/rust-eventsource-client) - Server-sent events (SSE) client implementation for Rust - launchdarkly/rust-eventsource-client

18. [Sonar reasoning pro - Perplexity](https://docs.perplexity.ai/docs/getting-started/models/models/sonar-reasoning-pro) - The sonar-reasoning-pro model is designed to output a <think> section containing reasoning tokens, i...

19. [Meet New Sonar - Perplexity API Platform](https://www.perplexity.ai/api-platform/resources/meet-new-sonar) - Build with the best AI answer engine API, created by Perplexity. Power your products with the fastes...

20. [Search API | perplexityai/perplexity-node | DeepWiki](https://deepwiki.com/perplexityai/perplexity-node/4.1-search-api) - This document provides complete technical documentation of the Search resource, which enables progra...

21. [[REQUEST] Add Perplexity search endpoint · Issue #1795 - GitHub](https://github.com/simstudioai/sim/issues/1795) - Add the perplexity search endpoint. Currently the Perplexity block only supports the /chat/completio...

22. [Changelog - Perplexity](https://docs.perplexity.ai/docs/resources/changelog) - Full type definitions for all request parameters and response fields; Support for Sonar and Search A...

23. [New Async Mode for Sonar Deep Research! - Announcements](https://community.perplexity.ai/t/new-async-mode-for-sonar-deep-research/424) - Our Sonar Deep Research model now offers an async mode that lets you submit research-intensive queri...

24. [Tools - Perplexity](https://docs.perplexity.ai/docs/agent-api/tools) - The Agent API provides tools that extend model capabilities beyond their training data. ... Explore ...

25. [How should I get search_domain_filter working for API completions?](https://www.reddit.com/r/perplexity_ai/comments/1i7c2rr/how_should_i_get_search_domain_filter_working_for/) - How should I get search_domain_filter working for API completions?


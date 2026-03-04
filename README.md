# pplx

A fast, single-binary CLI for the [Perplexity API](https://docs.perplexity.ai/), built in Rust.

Query the web from your terminal with real-time streaming, markdown rendering, citations, cost tracking, and full access to every Perplexity API parameter.

[![CI](https://github.com/osodevops/perplexity-cli/actions/workflows/test.yml/badge.svg)](https://github.com/osodevops/perplexity-cli/actions/workflows/test.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

<p align="center">
  <img src="recordings/ask-basic-github-optimised.gif" alt="pplx demo" width="800">
</p>

## Install

```bash
# Homebrew (macOS/Linux)
brew install osodevops/tap/pplx

# Pre-built binaries — download from GitHub Releases
# https://github.com/osodevops/perplexity-cli/releases

# From source
cargo install --git https://github.com/osodevops/perplexity-cli
```

## Setup

Run the one-time setup to store your [Perplexity API key](https://www.perplexity.ai/settings/api):

```bash
pplx config init
# Enter your Perplexity API key: pplx-...
# Config created at: ~/.config/pplx/config.toml
# You're ready to go! Try: pplx "What is Rust?"
```

This creates `~/.config/pplx/config.toml` with your key. You only need to do this once.

Alternatively, set the key as an environment variable:

```bash
export PERPLEXITY_API_KEY="pplx-..."
```

## Quick Start

```bash
# Ask anything (ask is the default subcommand)
pplx "What causes aurora borealis?"

# Pick a model
pplx ask -m sonar "Quick: capital of France?"
pplx ask -m sonar-reasoning-pro "Prove that sqrt(2) is irrational"

# Set a persona with a system prompt
pplx ask -s "You are a senior Rust developer" "Review this error handling pattern"

# Pipe content as context
cat error.log | pplx ask "What's causing this error?"

# JSON output for scripting
pplx ask --no-stream -o json "Top 5 Rust web frameworks" | jq '.citations'

# Search filters
pplx ask --domain arxiv.org --recency month "transformer architecture improvements"
pplx ask --search-mode academic "CRISPR clinical trials 2025"
pplx ask --after 01/01/2025 --context-size high "EU AI Act enforcement updates"

# Show citations, usage stats, and cost
pplx ask --citations --usage --cost "Latest developments in fusion energy"

# Reasoning models — show chain-of-thought
pplx ask -m sonar-reasoning-pro --reasoning "Is P = NP?"

# Save output to file
pplx ask --save report.md "Summarize recent AI breakthroughs"

# Shell completions
pplx completions zsh > ~/.zsh/completions/_pplx
```

### Web Search

```bash
# Raw web search (returns URLs, titles, snippets)
pplx search "Rust async runtime benchmarks"

# Multi-query search
pplx search "Rust tokio" "Rust async-std"

# With filters
pplx search --domain docs.rs --max-results 5 "serde derive"
```

### Deep Research

```bash
# Submit and wait for results
pplx research "Comprehensive analysis of WebAssembly adoption in 2025"

# Submit async and check later
pplx research --async "Long research topic"
pplx research status <job-id>
pplx research get <job-id>

# List all research jobs
pplx research list
```

### Agent API

```bash
# Use third-party models through Perplexity
pplx agent -m openai/gpt-4o "Explain monads"

# Enable tools
pplx agent --tool web_search --tool fetch_url "Latest Rust release notes"
```

### Interactive REPL

```bash
# Start a session
pplx interactive

# In the REPL:
#   /model sonar-pro     — switch model
#   /clear               — reset conversation
#   /cost                — show session costs
#   /help                — list commands
#   /quit                — exit
```

### Config Management

```bash
# Set defaults
pplx config set model sonar-pro
pplx config set output plain
pplx config set temperature 0.7

# View current config
pplx config show
```

## Features

- **Real-time streaming** — responses stream token-by-token with a search spinner
- **Markdown rendering** — built-in terminal rendering with syntax highlighting
- **Multiple output formats** — `md` (default), `plain`, `json`, `raw`
- **All API parameters exposed** — model, temperature, top-p, search mode, domain filters, recency, date ranges, context size, reasoning effort
- **Citations** — numbered source URLs below responses
- **Cost tracking** — per-request cost breakdown (input/output/request/citation/reasoning/search)
- **Token usage** — prompt, completion, and total token counts
- **Pipe-friendly** — reads from stdin, disables colours when piped, works with `jq`
- **Layered config** — CLI flags > environment variables > TOML config file > defaults
- **Shell completions** — bash, zsh, fish, PowerShell
- **Retry with backoff** — automatic retry on 429/5xx with exponential backoff
- **Cross-platform** — macOS (Apple Silicon + Intel), Linux, Windows
- **Zero runtime dependencies** — single statically-linked binary

## Models

| Model | Best For |
|-------|----------|
| `sonar` | Quick factual lookups (~1200 tok/s) |
| `sonar-pro` | Complex queries, multi-step search (200K context) |
| `sonar-reasoning-pro` | Chain-of-thought reasoning, math, analysis |
| `sonar-deep-research` | Comprehensive autonomous research reports |

## Configuration

`pplx config init` creates `~/.config/pplx/config.toml`. Edit it to set defaults:

```toml
[auth]
api_key = "pplx-..."

[defaults]
model = "sonar-pro"
output = "md"
temperature = 0.7
show_citations = true
show_cost = false

[defaults.search]
mode = "web"
recency = "month"
exclude_domains = ["pinterest.com", "quora.com"]
```

Check your current effective config:

```bash
pplx config show
```

**Precedence:** CLI flags > environment variables > config file > built-in defaults

### Environment Variables

| Variable | Description |
|----------|-------------|
| `PERPLEXITY_API_KEY` | API key (required) |
| `PPLX_MODEL` | Default model |
| `PPLX_OUTPUT` | Default output format |
| `PPLX_CONTEXT_SIZE` | Default search context size |

## Output Formats

```bash
# Rendered markdown (default) — colours, bold, code highlighting
pplx "Explain async/await in Rust"

# Plain text — no formatting, clean for piping
pplx ask -o plain "summarize this" < article.txt

# JSON — full API response for programmatic use
pplx ask -o json "top databases 2025" | jq '.citations[]'

# Raw — response content only, no metadata
RESULT=$(pplx ask -o raw "current BTC price")
```

## CLI Reference

```
pplx [OPTIONS] [QUERY]... [COMMAND]

Commands:
  ask          Send a query (default if no subcommand given)
  search       Raw web search via Search API
  research     Deep research with async support
  agent        Agent API with third-party models
  interactive  Start interactive REPL session
  config       Manage configuration (init, show, set)
  completions  Generate shell completions

Global Options:
  -m, --model <MODEL>           Model to use [default: sonar-pro]
  -o, --output <FORMAT>         Output format: md, plain, json, raw
  -s, --system <PROMPT>         System prompt
  -c, --citations               Show citations
  -u, --usage                   Show token usage
      --cost                    Show cost breakdown
      --no-stream               Disable streaming
      --no-color                Disable colour output
      --verbose                 Enable debug logging
      --config <PATH>           Config file path override

Search Filters:
      --search-mode <MODE>      web, academic, sec
      --domain <DOMAIN>         Include domain (repeatable)
      --exclude-domain <DOMAIN> Exclude domain (repeatable)
      --recency <PERIOD>        hour, day, week, month, year
      --after <DATE>            Results after date (MM/DD/YYYY)
      --before <DATE>           Results before date (MM/DD/YYYY)
      --context-size <SIZE>     minimal, low, medium, high

Model Parameters:
      --max-tokens <N>          Maximum response tokens
  -t, --temperature <FLOAT>     Temperature (0.0-2.0)
      --top-p <FLOAT>           Nucleus sampling (0.0-1.0)
      --reasoning-effort <LVL>  minimal, low, medium, high

Response Enrichment:
      --images                  Return image URLs
      --related                 Return related questions
      --search-results          Show full search result metadata
      --no-search               Disable web search
      --smart-search            Enable search classifier
      --reasoning               Show reasoning/thinking blocks
      --json-schema <SCHEMA>    JSON schema for structured output
      --save <PATH>             Save response to file
```

## Building from Source

```bash
git clone https://github.com/osodevops/perplexity-cli
cd perplexity-cli
cargo build --release
# Binary at target/release/pplx
```

**Requirements:** Rust 1.75+

```bash
# Run tests
cargo test

# Lint
cargo clippy --all-targets -- -D warnings
```

## Roadmap

- [x] **Phase 1: Core MVP** — `ask` command, streaming, output formats, config, citations, cost, retry, completions
- [x] **Phase 2: Search & Filters** — `search` command (Search API), `--json-schema`, `--save`, image/related rendering
- [x] **Phase 3: Advanced** — `interactive` REPL, `research` (async deep research), `agent` (third-party models), `<think>` block rendering
- [x] **Phase 4: Polish** — man pages, input validation, `config set`, expanded test suite, `--version` with git hash, AUR/Nix packaging

## License

[MIT](LICENSE)

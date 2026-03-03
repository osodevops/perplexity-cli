# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.3.6] - 2026-03-03

### Fixed

- `pplx config init` now prompts to update API key when config already exists instead of just exiting
- Fix Homebrew formula: ARM Mac URL was pointing to source tarball instead of prebuilt binary
- Fix auto-tag workflow to use PAT so it triggers the release pipeline automatically

## [0.3.0] - 2026-03-02

### Added

- Structured error handling with semantic exit codes (0-7) for programmatic consumption
- JSON error output on stdout (`-o json`) with `code`, `message`, `suggestion`, `exit_code` fields
- `--quiet` / `-q` global flag for bare content output (no formatting, spinners, or metadata)
- `--no-spinner` global flag to disable progress spinner
- `pplx describe` command outputting machine-readable JSON capability schema
- `research --dry-run` flag to preview request JSON without making an API call
- `cli_version` field injected into all JSON output envelopes
- Per-subcommand help examples (`pplx ask --help`, `pplx search --help`, etc.)
- `PplxError` methods: `error_code()`, `exit_code()`, `suggestion()`, `to_json()`
- Agent usage guide in CLAUDE.md with exit codes, error recovery, and quick reference

### Changed

- `main()` restructured into `main()`/`run()`/`handle_error()` for proper exit code propagation
- Validation errors (`anyhow::bail!`) converted to `PplxError::Validation` for exit code 2
- Integration tests fixed to use `--config /dev/null` to bypass user config file

## [0.2.0] - 2026-03-01

### Added

- `pplx search` command for raw web search via Search API (single and multi-query)
- `pplx research` command for async deep research with job management (submit, status, list, get)
- `pplx agent` command for Agent API with third-party models and tool support
- `pplx interactive` command for REPL sessions with slash commands (/model, /clear, /cost, etc.)
- `pplx config set` command for format-preserving config file updates
- `--reasoning` flag to display `<think>` blocks from reasoning models
- `--reasoning-effort` flag (minimal, low, medium, high)
- `--json-schema` flag for structured output (inline JSON or file path)
- `--save` flag to save responses to file
- `--images`, `--related`, `--search-results` flags for response enrichment
- `--no-search` and `--smart-search` flags for search control
- `--domain`, `--exclude-domain` flags for domain filtering
- `--after`, `--before`, `--updated-after`, `--updated-before` date filters
- `--search-mode` flag (web, academic, sec)
- `--recency` filter (hour, day, week, month, year)
- `--context-size` flag (minimal, low, medium, high)
- Cost tracking with per-request breakdown (input/output/request/citation/reasoning/search)
- Session cost accumulation in interactive mode
- Input validation for temperature, top_p, and other parameters
- Man page generation via `gen-man` binary
- `--version` output includes git commit hash
- AUR PKGBUILD and Nix flake packaging stubs

### Changed

- Version bumped to 0.2.0
- `pplx config` now supports subcommands: `init`, `show`, `set`
- Streaming uses ThinkParser state machine for `<think>` block extraction
- Removed dead code and `#[allow(dead_code)]` annotations

## [0.1.0] - 2025-01-15

### Added

- `pplx ask` command with real-time SSE streaming
- Implicit ask: `pplx "query"` works without subcommand
- Stdin pipe support: `cat file | pplx ask "summarize"`
- Multiple output formats: `md` (rendered markdown), `plain`, `json`, `raw`
- Layered configuration: CLI flags > env vars > TOML file > defaults
- `pplx config init` for one-time API key setup
- `pplx config show` to display effective configuration
- `pplx completions` for bash, zsh, fish, PowerShell
- Citations display with `--citations` flag
- Token usage display with `--usage` flag
- System prompt support with `--system` flag
- Automatic retry with exponential backoff on 429/5xx errors
- Spinner animation during search
- Colour auto-detection (disabled when piped)
- Cross-platform binaries (macOS ARM/Intel, Linux, Windows)

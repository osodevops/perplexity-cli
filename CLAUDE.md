# CLAUDE.md — pplx

## Project Overview

`pplx` is a Rust CLI for the Perplexity API. Single binary, zero runtime dependencies.

## Build & Test

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests (unit + integration)
cargo clippy --all-targets -- -D warnings  # Lint check
cargo fmt -- --check     # Format check
```

## Architecture

- **Single crate** with `lib.rs` (for integration tests) + `main.rs` (binary entry point)
- Modules: `cli/`, `api/`, `config/`, `output/`, `cost/`, `interactive/`, `error.rs`
- Error handling: `thiserror` for typed errors (`PplxError`), `anyhow` at binary boundary
- Config: layered merge (CLI flags > env vars > TOML file > defaults)
- Streaming: `reqwest-eventsource` for SSE parsing

## Key Conventions

- All clippy warnings treated as errors in CI (`-D warnings`)
- Integration tests use `wiremock` for HTTP mocking and `assert_cmd` for CLI invocation
- Test fixtures live in `tests/fixtures/`
- API types split: `ChatCompletionResponse` (non-streaming) vs `ChatCompletionChunk` (streaming)
- Output rendering: raw token printing during streaming, `termimad` for `--no-stream` mode

## Environment Variables

- `PERPLEXITY_API_KEY` — API authentication
- `PPLX_MODEL` — Default model
- `PPLX_OUTPUT` — Default output format
- `PPLX_CONTEXT_SIZE` — Default search context size

## Phase Status

- Phase 1 (Core MVP): Implemented
- Phase 2 (Search & Filters): Implemented
- Phase 3 (Advanced): Implemented
- Phase 4 (Polish & Distribution): Implemented

## Agent Usage Guide

For coding agents (Claude Code, Cursor, etc.) invoking `pplx` programmatically:

### Quick Reference

```bash
# JSON output for parsing
pplx ask -o json "your question" 2>/dev/null

# Quiet mode: bare content only, no spinners/formatting
pplx ask -q "your question"

# Disable spinner only (keep formatting)
pplx ask --no-spinner "your question"

# Machine-readable capabilities
pplx describe | jq .exit_codes

# Dry-run research (preview request without API call)
pplx research --dry-run "your query"
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Validation / config error |
| 3 | Authentication failed |
| 4 | Rate limited |
| 5 | Server / stream / research error |
| 6 | Not found (404) |
| 7 | Network / HTTP error |

### Structured Error JSON

With `-o json`, errors are emitted to **stdout** as:
```json
{"error": {"code": "auth_failed", "message": "...", "suggestion": "...", "exit_code": 3}}
```

### Error Recovery

- Exit 3 → set `PERPLEXITY_API_KEY` or run `pplx config init`
- Exit 4 → wait and retry (check `retry_after_secs` in error)
- Exit 5 → retry after brief delay, or use `--no-stream`
- Exit 7 → check network connectivity

### Environment Variables

- `PERPLEXITY_API_KEY` — API authentication (required)
- `PPLX_MODEL` — Default model
- `PPLX_OUTPUT` — Default output format
- `PPLX_CONTEXT_SIZE` — Default search context size
- `NO_COLOR` — Disable colour output (standard)

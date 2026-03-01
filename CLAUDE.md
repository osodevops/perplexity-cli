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
- Modules: `cli/`, `api/`, `config/`, `output/`, `cost/`, `error.rs`
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
- Phase 2 (Search & Filters): Stub
- Phase 3 (Advanced): Stub
- Phase 4 (Polish & Distribution): Partial (CI/CD configured)

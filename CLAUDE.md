# CLAUDE.md

## Project Overview

**TokenZip** is a Claude Code context optimizer that reduces LLM token consumption by 60-90%. Fork of RTK (rtk-ai/rtk) with 6 additional noise filters.

Binary: `tokenzip`
Language: Rust
License: MIT (based on RTK by rtk-ai)

## Build & Test

```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
```

## Architecture

```
tokenzip binary
  ├── ANSI preprocessor (ansi_filter.rs) — all output
  ├── Command router (main.rs, Clap enum)
  │     ├── 34 RTK modules (git, test, ls, grep, cargo, etc.)
  │     ├── error_cmd.rs — stacktrace compression (5 languages)
  │     ├── web_cmd.rs — HTML content extraction
  │     ├── build_cmd.rs — build error grouping
  │     ├── pkg_cmd.rs — package install log compression
  │     └── docker_cmd.rs — Docker build log compression
  ├── Error post-processor (error_cmd.rs) — after all modules
  └── SQLite tracking (tracking.rs) — feature column
```

## Key Modules

| Module | Purpose |
|--------|---------|
| ansi_filter.rs | ANSI/spinner/decoration preprocessor |
| error_cmd.rs | Error stacktrace compression (Node, Python, Rust, Go, Java) |
| web_cmd.rs | Web page content extraction (scraper crate) |
| build_cmd.rs | Build error grouping (tsc, eslint, cargo, mypy, pylint) |
| pkg_cmd.rs | Package install log compression (npm, pip, cargo) |
| docker_cmd.rs | Docker build log compression |
| tracking.rs | SQLite tracking with feature column |
| gain.rs | Analytics dashboard (--by-feature, --graph, --history) |
| init.rs | Hook installation, uninstall, update |
| main.rs | CLI router (Clap 4) |

## Conventions

- `lazy_static!` for all regex (never Regex::new() in functions)
- `anyhow::Result` for error handling
- Fallback to raw output if filter fails
- Tests in `#[cfg(test)] mod tests` within each module
- No async — single-threaded by design

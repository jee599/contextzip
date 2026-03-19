# r/rust

**Title:** ContextZip: A Rust CLI that compresses LLM context by 60-90% — 1,056 tests, built as an RTK fork

**Body:**

I forked RTK (Rust Token Killer) to build ContextZip — a CLI proxy that compresses command output before it reaches AI coding assistants like Claude Code.

Technical highlights:
- Single-threaded, zero async — <10ms overhead per command
- lazy_static! regex patterns for all 40+ filters
- scraper crate for HTML content extraction
- 5-language stacktrace parser (Node.js, Python, Rust, Go, Java)
- SQLite tracking with feature-level analytics
- 1,056 tests, cargo clippy clean

The Rust panic compression was interesting — detecting numbered backtrace frames, pairing them with `at` continuation lines, filtering /rustc/ paths and std::panicking/tokio::runtime/core::ops frames. Went from 2% savings to 80% after rewriting the parser.

102 benchmark tests with honest results (including cases where savings are negative).

`cargo install --git https://github.com/jee599/contextzip`

GitHub: https://github.com/jee599/contextzip

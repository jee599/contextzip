# Show HN

**Title:** Show HN: ContextZip – Cut Claude Code token usage by 70% (Rust CLI)

**URL:** https://github.com/jee599/contextzip

**First comment (post as reply after submission):**

Hi HN, I built ContextZip because I kept hitting Claude Code's context limit.

The problem: every time Claude Code runs git status, npm install, or cargo test, the raw output fills the context window with noise — ANSI escape codes, node_modules stacktrace frames, deprecated warnings, Docker layer hashes.

ContextZip hooks into Claude Code's PreToolUse system and compresses CLI output before it reaches the context. It's a fork of RTK (Rust Token Killer) with 6 additional noise filters:

1. Error stacktraces — hides framework frames for Node.js, Python, Rust, Go, Java
2. Web page extraction — strips nav/footer/ads, keeps article content
3. ANSI preprocessor — removes escape codes, spinners, progress bars
4. Build error grouping — groups 40 identical TS2322 errors into one, preserving all line numbers
5. Package install logs — removes deprecated/funding noise, keeps security warnings
6. Docker build logs — success = 1 line, failure = context preserved

I tested with 102 production-like inputs. Weighted average: 61% savings. Honest numbers — some edge cases have negative savings (output grows when formatting overhead exceeds noise). Those are in the README.

Tech: Rust, single-threaded, <10ms overhead, 1,056 tests, cargo clippy clean.

Install: npx contextzip (macOS/Linux/Windows)

Happy to answer questions about the implementation.

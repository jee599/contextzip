# r/opensource

**Title:** ContextZip — open source Claude Code context optimizer (MIT, Rust)

**Body:**

Just open-sourced ContextZip, a CLI tool that compresses command output by 60-90% for AI coding assistants.

It's a fork of RTK (28k stars) with 6 additional noise filters: error stacktraces, web pages, ANSI/spinners, build errors, package install logs, Docker builds.

- 1,056 tests, 102 benchmarks
- MIT license
- Rust, single binary, <10ms overhead
- Install: `npx contextzip`

GitHub: https://github.com/jee599/contextzip

Contributions welcome — there are "good first issue" labels for Swift/Xcode and Kotlin/Android stacktrace support.

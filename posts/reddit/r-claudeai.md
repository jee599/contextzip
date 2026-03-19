# r/ClaudeAI

**Title:** I built a tool that saves 60-90% of Claude Code's context window — open source, 5-second install

**Body:**

I kept hitting Claude Code's context limit because of noise — npm install dumping 150 lines of deprecated warnings, Node.js stacktraces with 30 lines of node_modules frames, Docker builds with 50 lines of layer hashes.

So I built ContextZip. It hooks into Claude Code and compresses CLI output before it reaches the context window.

Real examples:
- Node.js error: 30 lines → 3 lines (93% saved). Keeps your code frames, hides Express internals.
- npm install: 150 lines → 3 lines (95% saved). Security warnings preserved, deprecated noise gone.
- Docker build: 50 lines → 1 line (96% saved).

Install:
```
npx contextzip
```
Restart Claude Code. Done.

It's a Rust binary (<10ms overhead), fork of RTK with 6 additional noise filters. 1,056 tests, 102 benchmarks.

GitHub: https://github.com/jee599/contextzip

Happy to answer questions!

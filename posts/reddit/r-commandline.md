# r/commandline

**Title:** ContextZip: compress CLI output by 60-90% for AI coding assistants

**Body:**

ContextZip is a transparent CLI proxy that compresses command output. It hooks into your shell via Claude Code's PreToolUse hook — every command gets filtered automatically.

Before/after:
- `git status`: 12 lines → 4 lines
- `npm install`: 150 lines → 3 lines
- `docker build`: 50 lines → 1 line
- Error stacktraces: 30 lines → 3 lines

After each command:
💾 contextzip: 200 → 40 tokens (saved 80%)

Track savings: `contextzip gain --graph` shows daily ASCII chart.

Single Rust binary, <10ms overhead, zero config.

Install: `npx contextzip` or `curl -fsSL https://raw.githubusercontent.com/jee599/contextzip/main/install.sh | bash`

GitHub: https://github.com/jee599/contextzip

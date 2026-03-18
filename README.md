# TokenZip

Claude Code context optimizer. Fork of [RTK](https://github.com/rtk-ai/rtk) with 6 additional noise filters.

Single Rust binary, zero dependencies, <10ms overhead.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/jee599/tokenzip/main/install.sh | bash
```

Or build from source:

```bash
cargo install --git https://github.com/jee599/tokenzip
```

## What's New vs RTK

```
Noise Source              RTK    TokenZip
────────────────────────────────────────────
CLI output (git, test)    ✓      ✓ (RTK fork)
Error stacktraces         ✗      ✓
Web page fetch            ✗      ✓
ANSI/spinner/decoration   ~      ✓ (enhanced)
Build error grouping      ~      ✓ (enhanced)
Package install logs      ✗      ✓
Docker build logs         ~      ✓ (enhanced)
```

## Before / After

### Error Stacktraces (`tokenzip err`)

Detects 5 languages (Node.js, Python, Rust, Go, Java). Removes framework frames, keeps user code.

**Before** (Node.js, 8 lines):
```
TypeError: Cannot read properties of undefined (reading 'id')
    at getUserProfile (/app/src/api/users.ts:47:12)
    at processAuth (/app/src/middleware/auth.ts:12:5)
    at Module._compile (node:internal/modules/cjs/loader:1376:14)
    at Object.Module._extensions..js (node:internal/modules/cjs/loader:1435:10)
    at Module.load (node:internal/modules/cjs/loader:1207:32)
    at Router.handle (/app/node_modules/express/lib/router/index.js:45:12)
    at Layer.handle (/app/node_modules/express/lib/router/layer.js:95:5)
```

**After** (4 lines):
```
TypeError: Cannot read properties of undefined (reading 'id')
  → src/api/users.ts:47         getUserProfile()
  → src/middleware/auth.ts:12   processAuth()
  (+ 5 framework frames hidden)
```

### Web Page Fetch (`tokenzip web`)

Strips HTML boilerplate, nav, footer. Extracts `<main>` or `<article>` content.

**Before** (200+ lines of raw HTML):
```html
<!DOCTYPE html><html><head><meta charset="utf-8">
<title>API Docs</title><link rel="stylesheet" href="...">
<script>/* analytics */</script></head><body>
<nav>Home | Docs | Blog | Login</nav>
<main><h1>Authentication</h1><p>Use Bearer tokens...</p></main>
<footer>Copyright 2025</footer></body></html>
```

**After** (2 lines):
```
Authentication
Use Bearer tokens...
```

### Build Error Grouping (`tokenzip err`)

Groups repeated errors by code. Preserves ALL file:line locations.

**Before** (5 separate errors):
```
src/api/users.ts(47,5): error TS2322: Type 'string' is not assignable to type 'number'.
src/api/users.ts(83,10): error TS2322: Type 'string' is not assignable to type 'number'.
src/api/orders.ts(12,3): error TS2322: Type 'string' is not assignable to type 'number'.
src/api/orders.ts(45,7): error TS2345: Argument of type 'number' is not assignable.
src/api/products.ts(23,1): error TS2322: Type 'string' is not assignable to type 'number'.
```

**After** (grouped, 6 lines):
```
TS2322: Type 'string' is not assignable to type 'number'. (x4)
  src/api/users.ts     :47, :83
  src/api/orders.ts    :12
  src/api/products.ts  :23
TS2345: Argument of type 'number' is not assignable.
  src/api/orders.ts    :45
```

### Package Install Logs (`tokenzip pkg`)

Strips deprecation spam, funding messages, progress bars. Preserves security warnings.

**Before** (npm install, 15+ lines):
```
npm warn deprecated inflight@1.0.6: This module is not supported
npm warn deprecated glob@7.2.3: Glob versions prior to v9 are no longer supported
npm warn deprecated rimraf@3.0.2: Rimraf versions prior to v4 are no longer supported
added 847 packages, and audited 848 packages in 14s
119 packages are looking for funding
  run `npm fund` for details
3 vulnerabilities (1 moderate, 2 high)
```

**After** (2 lines):
```
added 847 packages in 14s
3 vulnerabilities (1 moderate, 2 high)
```

### Docker Build Logs (`tokenzip docker`)

Success: 1-line summary. Failure: failed step + 2 context steps + exit code.

**Before** (success, 30+ lines):
```
Step 1/12 : FROM node:20-alpine
 ---> abc123def456
Step 2/12 : WORKDIR /app
 ---> Using cache
 ---> def456789012
... (20+ more lines)
Successfully built 999aaa000bbb
Successfully tagged my-app:latest
```

**After** (1 line):
```
✓ built my-app:latest (12 steps, 7 cached)
```

**Before** (failure, 20+ lines):
```
Step 7/12 : RUN npm run build
 ---> Running in container123
error: Module not found: 'react-dom/client'
The command '/bin/sh -c npm run build' returned a non-zero code: 1
```

**After** (5 lines):
```
✗ Docker build failed at step 7/12
Step 5/12 : COPY . .
Step 6/12 : RUN npm ci    (cached ✓)
Step 7/12 : RUN npm run build        ← FAILED
  Module not found: 'react-dom/client'
  Exit code: 1
```

### ANSI / Spinner / Decoration Filter

Runs as preprocessor on ALL command output. Strips ANSI codes, spinner chars, progress bars (keeps final state), decoration lines, carriage-return overwrites.

**Before**:
```
⠙ Building modules...
⠸ Building modules...
[32m✓ Compiled successfully[0m
═══════════════════════════
```

**After**:
```
✓ Compiled successfully
```

## CLI Reference

```bash
# Core RTK commands (inherited)
tokenzip git status              # Compact git status
tokenzip git log -n 10           # One-line commits
tokenzip git diff                # Condensed diff
tokenzip test cargo test         # Show failures only
tokenzip ls .                    # Token-optimized directory tree
tokenzip read file.rs            # Smart file reading
tokenzip grep "pattern" .        # Grouped search results

# New TokenZip commands
tokenzip err <command>           # Error stacktrace compression
tokenzip web <url>               # Web page content extraction
tokenzip pkg <command>           # Package install log filtering

# Analytics
tokenzip gain                    # Token savings summary
tokenzip gain --graph            # ASCII graph (last 30 days)
tokenzip gain --history          # Recent command history
tokenzip gain --by-feature       # Savings by filter category

# Setup
tokenzip init --global           # Install auto-rewrite hook for Claude Code
tokenzip uninstall               # Remove hook and config
tokenzip update                  # Self-update to latest version
```

## Auto-Rewrite Hook

The hook transparently rewrites Bash commands (e.g., `git status` -> `tokenzip git status`) before execution. Claude never sees the rewrite, it just gets compressed output.

```bash
tokenzip init --global          # Install hook
# Restart Claude Code, then test:
git status                      # Automatically rewritten
```

Only applies to Bash tool calls. Claude Code built-in tools (Read, Grep, Glob) bypass the hook.

## Configuration

Config file: `~/.config/rtk/config.toml`

```toml
[tracking]
database_path = "/path/to/custom.db"

[hooks]
exclude_commands = ["curl", "playwright"]

[tee]
enabled = true
mode = "failures"
max_files = 20
```

## Attribution

Based on [RTK](https://github.com/rtk-ai/rtk) by rtk-ai. MIT License.

See [LICENSE](LICENSE) for details.

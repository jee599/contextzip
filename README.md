# TokenZip

**Your Claude Code is wasting tokens on noise. Fix it in 5 seconds.**

[English](#) | [한국어](docs/README.ko.md) | [日本語](docs/README.ja.md) | [中文](docs/README.zh.md)

---

## 5-Second Setup

```bash
# Homebrew (macOS/Linux)
brew install jee599/tap/tokenzip

# or curl
curl -fsSL https://raw.githubusercontent.com/jee599/tokenzip/main/install.sh | bash
```

Restart Claude Code. Done. Every command is now compressed automatically.

---

## The Problem

Every time Claude Code runs `git status`, `npm install`, or `cargo test`, the raw output eats your context window. 30 lines of `node_modules` stacktrace frames. 150 lines of `npm warn deprecated`. ANSI color codes nobody reads.

**Result:** You hit the context limit faster. Claude forgets earlier code. You pay more.

## The Fix

TokenZip intercepts CLI output and strips noise before it reaches Claude's context. Zero config. Zero overhead (<10ms).

### Real Examples

**`git status` — Before vs After**

Before (raw):
```
On branch main
Your branch is up to date with 'origin/main'.

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
        modified:   src/api/users.ts
        modified:   src/api/orders.ts

Untracked files:
  (use "git add <file>..." to include in what will be committed)
        src/api/products.ts

no changes added to commit
```
(12 lines, ~200 tokens)

After (tokenzip):
```
* main...origin/main
M src/api/users.ts
M src/api/orders.ts
? src/api/products.ts
```
(4 lines, ~40 tokens) — **80% saved**

---

**Node.js Error — Before vs After**

Before (30 lines, ~1,500 tokens):
```
TypeError: Cannot read properties of undefined (reading 'id')
    at getUserProfile (/app/src/api/users.ts:47:23)
    at processAuth (/app/src/middleware/auth.ts:12:5)
    at Layer.handle (/app/node_modules/express/lib/router/layer.js:95:5)
    at next (/app/node_modules/express/lib/router/route.js:144:13)
    at Route.dispatch (/app/node_modules/express/lib/router/route.js:114:3)
    ... 25 more node_modules frames
```

After (3 lines, ~100 tokens):
```
TypeError: Cannot read properties of undefined (reading 'id')
  → src/api/users.ts:47         getUserProfile()
  → src/middleware/auth.ts:12   processAuth()
  (+ 27 framework frames hidden)
```
**93% saved** — Claude sees the error + your code. Not Express internals.

---

**`npm install` — Before vs After**

Before (150 lines, ~2,000 tokens):
```
npm warn deprecated inflight@1.0.6: This module is not supported...
npm warn deprecated rimraf@3.0.2: Rimraf v3 is no longer supported...
... 47 more deprecated warnings ...
added 847 packages, and audited 848 packages in 32s
143 packages are looking for funding
  run `npm fund` for details
8 vulnerabilities (2 moderate, 6 high)
```

After (3 lines, ~50 tokens):
```
✓ 847 packages (32s)
⚠ 8 vulnerabilities (6 high, 2 moderate)
⚠ deprecated bcrypt@3.0.0: security vulnerability (CVE-2023-31484)
```
**95% saved** — Security warnings kept. Noise deleted.

---

**Docker Build (Success) — Before vs After**

Before (50 lines): Step-by-step with hashes, cache lines, intermediate containers
After (1 line): `✓ built my-app:latest (12 steps, 8 cached)` — **96% saved**

**Docker Build (Failure)** — Preserves what matters: failed step + 2 context steps + error + exit code.

---

## What Gets Compressed

| Source | What's removed | What's kept | Savings |
|--------|---------------|-------------|---------|
| **Error stacktraces** | node_modules, site-packages, java.lang.reflect frames | Error message + your code frames | ~93% |
| **Web pages** | nav, footer, ads, cookies, scripts | Article content, code blocks, tables | ~73% |
| **ANSI/spinners** | Color codes, progress bars, decorations | Final status, errors, timestamps | ~85% |
| **Build errors** | 40x duplicate TS2322 | Grouped by code, ALL line numbers kept | ~81% |
| **Package install** | deprecated, funding, resolution | Summary + security warnings | ~95% |
| **Docker build** | Layer hashes, cache lines, pull progress | Success: 1-line. Failure: context | ~96% |
| **CLI output** | git/test/ls verbosity | Essential info only (via RTK) | ~78% |

---

## Every Command Shows Savings

```
$ git status
* main...origin/main
M src/api/users.ts
💾 tokenzip: 200 → 40 tokens (saved 80%)
```

Track your total savings anytime:

```bash
tokenzip gain                  # Total savings dashboard
tokenzip gain --by-feature     # Which filters save most
tokenzip gain --graph          # Daily savings chart
tokenzip gain --history        # Recent command details
```

---

## CLI Reference

```bash
# These happen automatically via hook:
git status          # → tokenzip git status (compressed)
cargo test          # → tokenzip cargo test (failures only)
npm install         # → tokenzip npm install (noise removed)
docker build .      # → tokenzip docker build (summarized)

# Manual commands:
tokenzip web https://docs.example.com    # Extract page content
tokenzip err node server.js              # Error-focused output

# Analytics:
tokenzip gain                  # Savings dashboard
tokenzip gain --by-feature     # By filter type
tokenzip gain --graph          # Daily chart
tokenzip gain --history        # Recent commands

# Setup:
tokenzip init -g --auto-patch  # Install hook (done by installer)
tokenzip init --show           # Check installation
tokenzip update                # Self-update
tokenzip uninstall             # Clean removal
```

---

## How It Works

1. Claude Code hook intercepts bash commands
2. TokenZip compresses the output (ANSI → command filter → error post-processor)
3. Compressed result goes to Claude's context
4. You see savings after each command

**Zero config. Zero overhead. Just less waste.**

---

## Built on RTK

TokenZip is a fork of [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk) with 6 additional noise filters. All 34 RTK commands included. MIT License.

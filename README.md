<p align="center">
  <h1 align="center">⚡ ContextZip</h1>
</p>

<p align="center">
  <strong>Compress Claude Code context by 60-90%. Six noise filters RTK doesn't have.</strong>
</p>

<p align="center">
  <a href="https://github.com/jee599/contextzip/releases"><img src="https://img.shields.io/github/v/release/jee599/contextzip?style=flat-square" alt="Release" /></a>
  <a href="https://github.com/jee599/contextzip/actions"><img src="https://img.shields.io/github/actions/workflow/status/jee599/contextzip/ci.yml?style=flat-square" alt="CI" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/jee599/contextzip?style=flat-square" alt="License" /></a>
  <a href="https://github.com/jee599/contextzip/stargazers"><img src="https://img.shields.io/github/stars/jee599/contextzip?style=flat-square" alt="Stars" /></a>
</p>

<p align="center">
  <a href="#-5-second-setup">Install</a> •
  <a href="#-before--after">Examples</a> •
  <a href="#-benchmark-102-tests">Benchmark</a> •
  <a href="docs/README.ko.md">한국어</a> •
  <a href="docs/README.ja.md">日本語</a> •
  <a href="docs/README.zh.md">中文</a>
</p>

---

## 🔥 The Problem

Claude Code runs `git status`, `npm install`, `cargo test` — and dumps **raw output** into your context window.

- 30 lines of `node_modules` stacktrace. Only 3 matter.
- 150 lines of `npm warn deprecated`. None matter.
- ANSI color codes, spinners, progress bars. Zero information.

**Result:** Context limit hit faster. Claude forgets your code. You pay more.

## ⚡ The Fix

ContextZip intercepts CLI output and strips noise. Zero config. <10ms overhead.

```
Without ContextZip:          With ContextZip:
─────────────────────        ─────────────────────
Input: 2,000 tokens          Input: 2,000 tokens
       ↓                            ↓
Claude reads all 2,000       ContextZip filters
       ↓                            ↓
Context: 2,000 tokens        Context: 200 tokens
                              (saved 90%)
```

---

## 📦 5-Second Setup

```bash
curl -fsSL https://raw.githubusercontent.com/jee599/contextzip/main/install.sh | bash
```

Restart Claude Code. Done.

<details>
<summary><b>Other install methods</b></summary>

```bash
# Homebrew (macOS/Linux)
brew install jee599/tap/contextzip

# Cargo (Rust developers)
cargo install --git https://github.com/jee599/contextzip
```

</details>

> [!TIP]
> Verify: `contextzip --version` → `contextzip 0.1.0 (based on rtk 0.30.1)`

---

## 🚀 Quickstart

After install, every command is compressed automatically via hook:

```bash
$ git status
* main...origin/main
M src/api/users.ts
💾 contextzip: 200 → 40 tokens (saved 80%)
```

No prefix needed. The hook rewrites `git status` → `contextzip git status` transparently.

---

## 🔬 Before / After

**Node.js Error** — 30 lines → 3 lines

```diff
- TypeError: Cannot read properties of undefined (reading 'id')
-     at getUserProfile (/app/src/api/users.ts:47:23)
-     at processAuth (/app/src/middleware/auth.ts:12:5)
-     at Layer.handle (/app/node_modules/express/lib/router/layer.js:95:5)
-     at next (/app/node_modules/express/lib/router/route.js:144:13)
-     ... 25 more node_modules frames

+ TypeError: Cannot read properties of undefined (reading 'id')
+   → src/api/users.ts:47         getUserProfile()
+   → src/middleware/auth.ts:12   processAuth()
+   (+ 27 framework frames hidden)
```

**93% saved.** Error message + your code. Not Express internals.

<details>
<summary><b>📦 More examples</b></summary>

**`npm install`** — 150 lines → 3 lines

```diff
- npm warn deprecated inflight@1.0.6: This module is not supported
- npm warn deprecated rimraf@3.0.2: Rimraf v3 is no longer supported
- ... 47 more deprecated warnings
- added 847 packages, and audited 848 packages in 32s
- 143 packages are looking for funding
- 8 vulnerabilities (2 moderate, 6 high)

+ ✓ 847 packages (32s)
+ ⚠ 8 vulnerabilities (6 high, 2 moderate)
+ ⚠ deprecated bcrypt@3.0.0: security vulnerability (CVE-2023-31484)
```

**95% saved.** Security warnings kept. Noise deleted.

---

**Docker Build (Success)** — 50 lines → 1 line

```diff
- Step 1/12 : FROM node:20-alpine
-  ---> abc123def456
- Step 2/12 : WORKDIR /app
-  ---> Using cache
- ... 10 more steps with hashes and cache lines
- Successfully built abc123final
- Successfully tagged my-app:latest

+ ✓ built my-app:latest (12 steps, 8 cached)
```

**96% saved.**

---

**Docker Build (Failure)** — Keeps what matters:

```
✗ Docker build failed at step 7/12

Step 5/12 : COPY package*.json ./    (cached ✓)
Step 6/12 : RUN npm install          (cached ✓)
Step 7/12 : RUN npm run build        ← FAILED
  error: Module not found: 'react-dom/client'
  Exit code: 1
```

---

**Python Traceback** — Framework frames hidden:

```diff
- Traceback (most recent call last):
-   File "/app/main.py", line 10, in handler
-     process(data)
-   File "/usr/lib/python3.11/importlib/__init__.py", line 126, in import_module
-   File "/app/venv/lib/python3.11/site-packages/flask/app.py", line 1498, in __call__
- ValueError: invalid literal for int()

+ Traceback (most recent call last):
+   → /app/main.py:10         process(data)
+   (+ 2 framework frames hidden)
+ ValueError: invalid literal for int()
```

---

**Rust Panic** — std/tokio frames removed:

```diff
- thread 'main' panicked at 'index out of bounds', src/handler.rs:42:5
- stack backtrace:
-    0: std::panicking::begin_panic
-    1: core::panicking::panic_fmt
-    2: myapp::handler::process at ./src/handler.rs:42:5
-    3: myapp::main at ./src/main.rs:15:3
-    4: std::rt::lang_start
-    5: tokio::runtime::enter

+ thread 'main' panicked at 'index out of bounds', src/handler.rs:42:5
+   (+ 2 framework frames hidden)
+   → ./src/handler.rs:42  myapp::handler::process()
+   → ./src/main.rs:15     myapp::main()
+   (+ 2 framework frames hidden)
```

**80% saved.**

</details>

---

## 📊 Benchmark (102 Tests)

Tested with production-like inputs. [Full results →](docs/benchmark-results.md)

| Category | Cases | Avg Savings | Best | Worst |
|:---------|------:|------------:|-----:|------:|
| Docker build logs | 10 | **88.2%** | 97% | 77% |
| ANSI/spinners | 15 | **82.5%** | 98% | 41% |
| Error stacktraces | 20 | **58.7%** | 97% | 2%* |
| Build errors | 15 | **55.6%** | 90% | -10%* |
| Web pages | 15 | **42.5%** | 64% | 5% |
| CLI commands | 12 | **42.0%** | 78% | -56%* |
| Package install | 15 | **39.2%** | 99% | 2% |
| **Overall** | **102** | **57.4%** | | |

> **Weighted: 61.1%** savings (326K chars in → 127K chars out)

\* Negative = output grew. Happens with tiny inputs where formatting overhead exceeds noise.

---

## 🔄 ContextZip vs RTK

Built on [RTK](https://github.com/rtk-ai/rtk). All 34 RTK commands included, plus:

| Noise Source | RTK | ContextZip |
|:-------------|:---:|:----------:|
| CLI output (git, test, ls) | ✓ | ✓ |
| Error stacktraces (5 languages) | ✗ | **✓** |
| Web page fetch | ✗ | **✓** |
| ANSI/spinner/decoration | partial | **✓ enhanced** |
| Build error grouping | partial | **✓ enhanced** |
| Package install logs | ✗ | **✓** |
| Docker build logs | partial | **✓ enhanced** |
| Per-command savings display | ✗ | **✓** |

---

## ⚙️ How It Works

```
Claude Code hook intercepts bash command
       ↓
contextzip binary
  ├── [1] ANSI preprocessor ──→ strip escape codes, spinners
  ├── [2] Command router ──→ 40+ specialized filters
  ├── [3] Error post-processor ──→ compress stacktraces
  └── [4] SQLite tracking ──→ record savings
       ↓
Compressed output → Claude's context
💾 contextzip: 2,000 → 200 tokens (saved 90%)
```

---

## 📈 Track Your Savings

```bash
$ contextzip gain
📊 ContextZip Token Savings
════════════════════════════════════
Total commands:    2,927
Tokens saved:      10.3M (89.2%)

$ contextzip gain --by-feature
Feature        Commands  Saved     Avg%
cli (RTK)      2,100     6.8M     78%
error          89        1.2M     93%
web            43        0.9M     73%
build          112       0.4M     81%
pkg            34        0.3M     95%
docker         22        0.2M     85%
```

```bash
contextzip gain --graph      # Daily savings chart
contextzip gain --history    # Recent command details
```

---

## 🛡️ Safety Guarantees

| What | Rule |
|:-----|:-----|
| Error messages | **ALWAYS** preserved (first line + user code frames) |
| File:line locations | **NEVER** removed from build errors |
| Security warnings | **ALWAYS** kept (CVE, GHSA, vulnerability) |
| Docker failure context | **ALWAYS** preserved (failed step + 2 prior + exit code) |
| Exit codes | **ALWAYS** propagated (CI/CD safe) |

> [!IMPORTANT]
> ContextZip only removes **confirmed noise**. If in doubt, the original output passes through unchanged.

---

## 🔧 CLI Reference

```bash
# Automatic (via hook — no prefix needed):
git status          # → contextzip git status
cargo test          # → contextzip cargo test
npm install         # → contextzip npm install

# Manual commands:
contextzip web https://docs.example.com    # Extract page content
contextzip err node server.js              # Error-focused output

# Analytics:
contextzip gain                  # Savings dashboard
contextzip gain --by-feature     # By filter type
contextzip gain --graph          # Daily chart
contextzip gain --history        # Recent commands

# Setup:
contextzip init --show           # Check installation
contextzip update                # Self-update
contextzip uninstall             # Clean removal
```

---

## 🤝 Contributing

Contributions welcome! ContextZip is a Rust project.

```bash
git clone https://github.com/jee599/contextzip.git
cd contextzip
cargo test         # 1056 tests
cargo clippy       # Lint check
```

## 📜 License

MIT — Based on [RTK](https://github.com/rtk-ai/rtk) by rtk-ai.

---

<p align="center">
  <sub>⚡ Save tokens. Ship faster.</sub>
</p>

[![Star History Chart](https://api.star-history.com/svg?repos=jee599/contextzip&type=Date)](https://star-history.com/#jee599/contextzip&Date)
</content>
</invoke>
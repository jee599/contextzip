<h1 align="center">
  <br>
  ⚡ ContextZip
  <br>
</h1>

<h3 align="center">
  Claude's output is full of noise you don't need.<br>
  ContextZip compresses it by 60-90%. <code>curl | bash</code> → 5 seconds.
</h3>

<p align="center">
  <a href="https://github.com/jee599/contextzip/releases"><img src="https://img.shields.io/github/v/release/jee599/contextzip?style=flat-square&color=blue" alt="Release" /></a>
  <a href="https://github.com/jee599/contextzip/actions"><img src="https://img.shields.io/github/actions/workflow/status/jee599/contextzip/ci.yml?style=flat-square" alt="CI" /></a>
  <img src="https://img.shields.io/badge/tests-1%2C056_passing-brightgreen?style=flat-square" alt="Tests" />
  <img src="https://img.shields.io/badge/benchmarks-102_cases-orange?style=flat-square" alt="Benchmarks" />
  <a href="LICENSE"><img src="https://img.shields.io/github/license/jee599/contextzip?style=flat-square" alt="License" /></a>
</p>

<p align="center">
  <a href="#-install-it-now">Install</a> •
  <a href="#-see-the-difference">Before/After</a> •
  <a href="#-the-numbers-dont-lie">Benchmark</a> •
  <a href="docs/README.ko.md">한국어</a> •
  <a href="docs/README.ja.md">日本語</a> •
  <a href="docs/README.zh.md">中文</a>
</p>

---

```
  30 lines of node_modules stacktrace    →    3 lines
  150 lines of npm deprecated warnings   →    3 lines
  50 lines of Docker build hashes        →    1 line
  ANSI colors, spinners, progress bars   →    gone
```

<h3 align="center">⬇️ One line. That's it.</h3>

```bash
curl -fsSL https://raw.githubusercontent.com/jee599/contextzip/main/install.sh | bash
```

<p align="center">Restart Claude Code. Every command is now compressed. Zero config.</p>

<details>
<summary>brew / cargo install</summary>

```bash
brew install jee599/tap/contextzip          # Homebrew
cargo install --git https://github.com/jee599/contextzip   # Cargo
```

</details>

---

## 👀 See the Difference

<table>
<tr>
<td width="50%">

**❌ Without ContextZip**
```
TypeError: Cannot read properties
  of undefined (reading 'id')
    at getUserProfile (users.ts:47)
    at processAuth (auth.ts:12)
    at Layer.handle (node_modules/
      express/lib/router/layer.js:95)
    at next (node_modules/express/
      lib/router/route.js:144)
    at Route.dispatch (node_modules/
      express/lib/router/route.js:114)
    ... 25 more lines




~1,500 tokens eaten
```

</td>
<td width="50%">

**✅ With ContextZip**
```
TypeError: Cannot read properties
  of undefined (reading 'id')
  → users.ts:47    getUserProfile()
  → auth.ts:12     processAuth()
  (+ 27 framework frames hidden)






~100 tokens. Done.

💾 contextzip: 1,500 → 100 (93% saved)
```

</td>
</tr>
</table>

<details>
<summary><b>📦 More before/after examples</b></summary>

<br>

### npm install — 150 lines → 3 lines

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
> 95% saved. Security warnings kept. Noise deleted.

### Docker build (success) — 50 lines → 1 line

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
> 96% saved.

### Docker build (failure) — keeps context

```
✗ Docker build failed at step 7/12

Step 5/12 : COPY package*.json ./    (cached ✓)
Step 6/12 : RUN npm install          (cached ✓)
Step 7/12 : RUN npm run build        ← FAILED
  error: Module not found: 'react-dom/client'
  Exit code: 1
```

### Python traceback

```diff
- Traceback (most recent call last):
-   File "/app/main.py", line 10, in handler
-   File "/usr/lib/python3.11/importlib/__init__.py", line 126
-   File "/app/venv/lib/site-packages/flask/app.py", line 1498
- ValueError: invalid literal for int()

+ Traceback (most recent call last):
+   → /app/main.py:10         process(data)
+   (+ 2 framework frames hidden)
+ ValueError: invalid literal for int()
```

### Rust panic

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
> 80% saved.

</details>

---

## 📊 The Numbers Don't Lie

> **102 real-world tests. No cherry-picking.**

| Category | Tests | Avg Savings | 🏆 Best | 💀 Worst |
|:---------|------:|:----------:|:-------:|:-------:|
| 🐳 Docker build | 10 | **88%** | 97% | 77% |
| 🎨 ANSI/spinners | 15 | **83%** | 98% | 41% |
| 💥 Error traces | 20 | **59%** | 97% | 2% |
| 🔨 Build errors | 15 | **56%** | 90% | -10% |
| 🌐 Web pages | 15 | **43%** | 64% | 5% |
| 💻 CLI commands | 12 | **42%** | 78% | -56% |
| 📦 Package install | 15 | **39%** | 99% | 2% |

**Weighted total: 61% savings** → 326K chars in, 127K chars out

> [!NOTE]
> Negative = output grew. Happens on tiny inputs. We put the worst numbers in the table because hiding them would be dishonest. [Full benchmark →](docs/benchmark-results.md)

---

## 🏎️ How It Works

```
  ┌─────────────────────────────────────────────┐
  │  Claude Code runs: git status               │
  │                         ↓                   │
  │  Hook rewrites → contextzip git status      │
  │                         ↓                   │
  │  ┌──────────────────────────────────────┐   │
  │  │ [1] ANSI preprocessor    strip junk  │   │
  │  │ [2] Command router    40+ filters    │   │
  │  │ [3] Error post-proc   compress stack │   │
  │  │ [4] SQLite tracker    record savings │   │
  │  └──────────────────────────────────────┘   │
  │                         ↓                   │
  │  Compressed output → Claude's context       │
  │  💾 contextzip: 200 → 40 tokens (80%)       │
  └─────────────────────────────────────────────┘
```

---

## 🆚 Why Not Just RTK?

Built on [RTK](https://github.com/rtk-ai/rtk) (28k⭐). All 34 RTK commands included. **Plus:**

| | RTK | ContextZip |
|:---|:---:|:---:|
| CLI compression (git, test, ls) | ✅ | ✅ |
| Error stacktraces (Node/Python/Rust/Go/Java) | ❌ | ✅ |
| Web page content extraction | ❌ | ✅ |
| ANSI / spinner / decoration removal | 🟡 | ✅ |
| Build error grouping (tsc/eslint/cargo) | 🟡 | ✅ |
| Package install noise (npm/pip/cargo) | ❌ | ✅ |
| Docker build compression | 🟡 | ✅ |
| Per-command savings display | ❌ | ✅ |

---

## 📈 Track Everything

```bash
$ contextzip gain
📊 ContextZip Token Savings
════════════════════════════════════════
Total commands:    2,927
Tokens saved:      10.3M (89.2%)
Efficiency meter: █████████████████████░░░ 89%

$ contextzip gain --by-feature
Feature        Commands  Saved     Avg%
cli (RTK)      2,100     6.8M     78%
error          89        1.2M     93%
web            43        0.9M     73%
build          112       0.4M     81%
pkg            34        0.3M     95%
docker         22        0.2M     85%
```

<p align="center">
  <code>--graph</code> daily chart &nbsp;•&nbsp; <code>--history</code> recent commands
</p>

---

## 🛡️ Nothing Important Gets Lost

| | |
|:---|:---|
| 🔴 Error messages | **ALWAYS** preserved |
| 📍 File:line in build errors | **NEVER** removed |
| 🔒 Security warnings (CVE, GHSA) | **ALWAYS** kept |
| 🐳 Docker failure context | **ALWAYS** preserved |
| ⏎ Exit codes | **ALWAYS** propagated |

> [!IMPORTANT]
> ContextZip only removes **confirmed noise**. When in doubt → passthrough.

---

## 🔧 Commands

```bash
# Automatic (hook rewrites these — no prefix needed):
git status              npm install             cargo test
docker build .          pip install flask        go test ./...

# Manual:
contextzip web https://docs.example.com    # page → content only
contextzip err node server.js              # error-focused output

# Analytics:
contextzip gain                  # dashboard
contextzip gain --by-feature     # per-filter stats
contextzip gain --graph          # daily chart
contextzip gain --history        # recent commands

# Manage:
contextzip init --show           # check setup
contextzip update                # self-update
contextzip uninstall             # clean removal
```

---

## 🤝 Contribute

```bash
git clone https://github.com/jee599/contextzip.git && cd contextzip
cargo test         # 1,056 tests
cargo clippy       # lint
```

## 📜 License

MIT — Fork of [RTK](https://github.com/rtk-ai/rtk) by rtk-ai.

---

<p align="center">
  <b>⚡ Less noise. More code. Ship faster.</b>
</p>

<p align="center">
  <a href="https://github.com/jee599/contextzip">
    <img src="https://img.shields.io/badge/GitHub-⭐_Star_this_repo-yellow?style=for-the-badge&logo=github" alt="Star" />
  </a>
</p>

[![Star History Chart](https://api.star-history.com/svg?repos=jee599/contextzip&type=Date)](https://star-history.com/#jee599/contextzip&Date)

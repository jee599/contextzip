<h1 align="center">
  <br>
  ⚡ ContextZip
  <br>
</h1>

<h3 align="center">
  Claude's output is full of noise you don't need.<br>
  ContextZip compresses it by 60-90%. <code>npx contextzip</code> → 5 seconds.
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
npx contextzip
```

<p align="center">Restart Claude Code. Every command is now compressed. Zero config.<br>
<b>macOS · Linux · Windows</b></p>

<details>
<summary>Other install methods</summary>

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/jee599/contextzip/main/install.sh | bash
brew install jee599/tap/contextzip

# Rust developers
cargo install --git https://github.com/jee599/contextzip
```

`npx contextzip` works on all platforms (macOS, Linux, Windows).

</details>

---

## 👀 See the Difference

### 💥 Node.js Error — 30 lines → 3 lines (93% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
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
    ... 25 more node_modules lines
```

</td>
<td width="50%">

**✅ After**
```
TypeError: Cannot read properties
  of undefined (reading 'id')
  → users.ts:47    getUserProfile()
  → auth.ts:12     processAuth()
  (+ 27 framework frames hidden)



💾 saved 93%
```

</td>
</tr>
</table>

### 📦 npm install — 150 lines → 3 lines (95% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
```
npm warn deprecated inflight@1.0.6
npm warn deprecated rimraf@3.0.2
npm warn deprecated glob@7.2.3
npm warn deprecated bcrypt@3.0.0:
  security vulnerability CVE-2023-31484
... 45 more deprecated warnings
added 847 packages, audited 848
143 packages looking for funding
  run `npm fund` for details
8 vulnerabilities (2 moderate, 6 high)
  To address issues: npm audit fix
  ... 20 more lines
```

</td>
<td width="50%">

**✅ After**
```
✓ 847 packages (32s)
⚠ 8 vulnerabilities (6 high, 2 mod)
⚠ bcrypt@3.0.0: CVE-2023-31484




Security kept. Noise gone.

💾 saved 95%
```

</td>
</tr>
</table>

### 🐳 Docker Build — 50 lines → 1 line (96% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
```
Step 1/12 : FROM node:20-alpine
 ---> abc123def456
Step 2/12 : WORKDIR /app
 ---> Using cache
 ---> 789ghi012jkl
Step 3/12 : COPY package*.json ./
 ---> Using cache
... 8 more steps with hashes
Removing intermediate container xyz
Successfully built abc123final
Successfully tagged my-app:latest
```

</td>
<td width="50%">

**✅ After**
```
✓ built my-app:latest (12 steps, 8 cached)








💾 saved 96%
```

</td>
</tr>
</table>

### 🐍 Python Traceback — framework frames hidden (72% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
```
Traceback (most recent call last):
  File "/app/main.py", line 10,
    in handler
    process(data)
  File "/usr/lib/python3.11/
    importlib/__init__.py", line 126
  File "/app/venv/lib/site-packages/
    flask/app.py", line 1498
  File "/app/venv/lib/site-packages/
    flask/app.py", line 1476
ValueError: invalid literal for int()
```

</td>
<td width="50%">

**✅ After**
```
Traceback (most recent call last):
  → /app/main.py:10  process(data)
  (+ 3 framework frames hidden)
ValueError: invalid literal for int()




💾 saved 72%
```

</td>
</tr>
</table>

### 🦀 Rust Panic — std/tokio removed (80% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
```
thread 'main' panicked at
  'index out of bounds',
  src/handler.rs:42:5
stack backtrace:
   0: std::panicking::begin_panic
   1: core::panicking::panic_fmt
   2: myapp::handler::process
        at ./src/handler.rs:42:5
   3: myapp::main
        at ./src/main.rs:15:3
   4: std::rt::lang_start
   5: tokio::runtime::enter
```

</td>
<td width="50%">

**✅ After**
```
thread 'main' panicked at
  'index out of bounds',
  src/handler.rs:42:5
  (+ 2 framework frames hidden)
  → handler.rs:42  process()
  → main.rs:15     main()
  (+ 2 framework frames hidden)


💾 saved 80%
```

</td>
</tr>
</table>

### 🔨 TypeScript Build — 40 errors grouped (81% saved)

<table>
<tr>
<td width="50%">

**❌ Before**
```
src/api/users.ts:47:5 - error TS2322:
  Type 'string' not assignable to 'number'
src/api/users.ts:83:5 - error TS2322:
  Type 'string' not assignable to 'number'
src/api/orders.ts:12:5 - error TS2322:
  Type 'string' not assignable to 'number'
src/api/orders.ts:45:5 - error TS2322:
  Type 'string' not assignable to 'number'
... 36 more identical errors
Found 40 errors in 8 files.
```

</td>
<td width="50%">

**✅ After**
```
TS2322: Type 'string' not assignable
        to type 'number' (×40)
  src/api/users.ts    :47, :83
  src/api/orders.ts   :12, :45, :67
  src/api/products.ts :23, :89
  src/lib/helpers.ts  :156
  ... +4 files (28 occurrences)

All line numbers preserved.
💾 saved 81%
```

</td>
</tr>
</table>

### 🌐 Web Page — nav/footer/ads stripped (73% saved)

<table>
<tr>
<td width="50%">

**❌ Before (curl output)**
```
[Skip to content]
[Nav: Products, Pricing, Docs, Blog]
[Sidebar: Getting Started, Auth,
  Database, Storage, Functions]
# Email/Password Authentication
Use supabase.auth.signInWithPassword
  to sign in users...
[code example]
[code example]
[Footer: © 2026 Supabase Inc]
[Terms | Privacy | Status]
[Newsletter: Subscribe for updates]
[Social: Twitter GitHub Discord]
```

</td>
<td width="50%">

**✅ After**
```
# Email/Password Authentication
Use supabase.auth.signInWithPassword
  to sign in users...
[code example]
[code example]




Nav, footer, sidebar, newsletter,
social links — all stripped.
💾 saved 73%
```

</td>
</tr>
</table>

### 🎨 ANSI / Spinners — invisible noise removed (83% saved)

<table>
<tr>
<td width="50%">

**❌ Before (raw terminal)**
```
\033[32m✓ Success\033[0m
\033[31m✗ Error\033[0m
⠋ Installing dependencies...
⠙ Installing dependencies...
⠹ Installing dependencies...
⠸ Installing dependencies...
████░░░░░░ 40%
████████░░ 80%
██████████ 100%
═══════════════════════
Done.
```

</td>
<td width="50%">

**✅ After**
```
✓ Success
✗ Error
██████████ 100%
Done.





Only final states kept.
💾 saved 83%
```

</td>
</tr>
</table>

<details>
<summary><b>🐳 Docker failure — context preserved</b></summary>

```
✗ Docker build failed at step 7/12

Step 5/12 : COPY package*.json ./    (cached ✓)
Step 6/12 : RUN npm install          (cached ✓)
Step 7/12 : RUN npm run build        ← FAILED
  error: Module not found: 'react-dom/client'
  Exit code: 1
```

> Failed step + 2 prior steps + error message + exit code. Always.

</details>

<details>
<summary><b>☕ Java / 🐹 Go stacktraces</b></summary>

**Java** — removes `java.lang.reflect`, `sun.reflect`, `org.springframework`, `org.apache`, `jdk.internal`:
```diff
- java.lang.NullPointerException: Cannot invoke method on null
-   at com.myapp.UserService.getUser(UserService.java:42)
-   at com.myapp.Controller.handle(Controller.java:15)
-   at java.lang.reflect.Method.invoke(Method.java:498)
-   at sun.reflect.DelegatingMethodAccessorImpl.invoke(...)
-   at org.springframework.web.servlet.FrameworkServlet.service(...)
-   at org.apache.catalina.core.ApplicationFilterChain.internalDoFilter(...)

+ java.lang.NullPointerException: Cannot invoke method on null
+   at com.myapp.UserService.getUser(UserService.java:42)
+   at com.myapp.Controller.handle(Controller.java:15)
+   (+ 4 framework frames hidden)
```

**Go** — removes `runtime/`, `runtime.gopanic`, `runtime.main`:
```diff
- goroutine 1 [running]:
- runtime/debug.Stack()
-   /usr/local/go/src/runtime/debug/stack.go:24
- runtime.gopanic({0x1234, 0x5678})
-   /usr/local/go/src/runtime/panic.go:884
- main.handler()
-   /app/handler.go:42 +0x1a4
- main.main()
-   /app/main.go:15 +0x58

+ goroutine 1 [running]:
+   (+ 2 framework frames hidden)
+   → main.handler()  /app/handler.go:42
+   → main.main()     /app/main.go:15
+   (+ 1 framework frames hidden)
```

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

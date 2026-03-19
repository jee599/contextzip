<p align="center">
  <h1 align="center">⚡ ContextZip</h1>
</p>

<p align="center">
  <strong>压缩 Claude Code 上下文 60-90%。RTK 没有的 6 个噪音过滤器。</strong>
</p>

<p align="center">
  <a href="https://github.com/jee599/contextzip/releases"><img src="https://img.shields.io/github/v/release/jee599/contextzip?style=flat-square" alt="Release" /></a>
  <a href="https://github.com/jee599/contextzip/actions"><img src="https://img.shields.io/github/actions/workflow/status/jee599/contextzip/ci.yml?style=flat-square" alt="CI" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/jee599/contextzip?style=flat-square" alt="License" /></a>
  <a href="https://github.com/jee599/contextzip/stargazers"><img src="https://img.shields.io/github/stars/jee599/contextzip?style=flat-square" alt="Stars" /></a>
</p>

<p align="center">
  <a href="#-5-秒安装">安装</a> •
  <a href="#-before--after">示例</a> •
  <a href="#-基准测试102-项">基准测试</a> •
  <a href="../README.md">English</a> •
  <a href="README.ko.md">한국어</a> •
  <a href="README.ja.md">日本語</a> •
  中文
</p>

---

## 🔥 问题

Claude Code 执行 `git status`、`npm install`、`cargo test`，然后把**原始输出**全部倾倒进上下文窗口。

- 30 行 `node_modules` 堆栈帧。有用的只有 3 行。
- 150 行 `npm warn deprecated`。有用的 0 行。
- ANSI 颜色代码、加载动画、进度条。信息量为零。

**后果：** 上下文上限提前触达。Claude 忘掉之前的代码。费用增加。

## ⚡ 解决方案

ContextZip 拦截 CLI 输出，剥离噪音。零配置。开销 <10ms。

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

## 📦 5 秒安装

```bash
curl -fsSL https://raw.githubusercontent.com/jee599/contextzip/main/install.sh | bash
```

重启 Claude Code。搞定。

<details>
<summary><b>其他安装方式</b></summary>

```bash
# Homebrew (macOS/Linux)
brew install jee599/tap/contextzip

# Cargo (Rust developers)
cargo install --git https://github.com/jee599/contextzip
```

</details>

> [!TIP]
> 验证: `contextzip --version` → `contextzip 0.1.0 (based on rtk 0.30.1)`

---

## 🚀 Quickstart

安装后，所有命令通过 hook 自动压缩：

```bash
$ git status
* main...origin/main
M src/api/users.ts
💾 contextzip: 200 → 40 tokens (saved 80%)
```

无需前缀。Hook 将 `git status` → `contextzip git status` 透明转换。

---

## 🔬 Before / After

**Node.js 报错** — 30 行 → 3 行

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

**节省 93%。** 错误信息 + 你的代码。不是 Express 内部实现。

<details>
<summary><b>📦 更多示例</b></summary>

**`npm install`** — 150 行 → 3 行

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

**节省 95%。** 安全警告保留。噪音删除。

---

**Docker 构建（成功）** — 50 行 → 1 行

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

**节省 96%。**

---

**Docker 构建（失败）** — 只保留关键信息：

```
✗ Docker build failed at step 7/12

Step 5/12 : COPY package*.json ./    (cached ✓)
Step 6/12 : RUN npm install          (cached ✓)
Step 7/12 : RUN npm run build        ← FAILED
  error: Module not found: 'react-dom/client'
  Exit code: 1
```

---

**Python Traceback** — 隐藏框架帧：

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

**Rust Panic** — 移除 std/tokio 帧：

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

**节省 80%。**

</details>

---

## 📊 基准测试（102 项）

使用生产级输入测试。[完整结果 →](benchmark-results.md)

| 类别 | 数量 | 平均节省 | 最高 | 最低 |
|:---------|------:|------------:|-----:|------:|
| Docker 构建日志 | 10 | **88.2%** | 97% | 77% |
| ANSI/加载动画 | 15 | **82.5%** | 98% | 41% |
| 错误堆栈 | 20 | **58.7%** | 97% | 2%* |
| 构建错误 | 15 | **55.6%** | 90% | -10%* |
| 网页 | 15 | **42.5%** | 64% | 5% |
| CLI 命令 | 12 | **42.0%** | 78% | -56%* |
| 包安装 | 15 | **39.2%** | 99% | 2% |
| **总计** | **102** | **57.4%** | | |

> **加权平均: 61.1%** 节省 (326K chars in → 127K chars out)

\* 负数 = 输出增大。极小输入下格式开销超过噪音时发生。

---

## 🔄 ContextZip vs RTK

基于 [RTK](https://github.com/rtk-ai/rtk)。包含 RTK 全部 34 个命令，另外增加：

| 噪音来源 | RTK | ContextZip |
|:-------------|:---:|:----------:|
| CLI 输出 (git, test, ls) | ✓ | ✓ |
| 错误堆栈（5 种语言） | ✗ | **✓** |
| 网页抓取 | ✗ | **✓** |
| ANSI/加载动画/装饰 | partial | **✓ enhanced** |
| 构建错误分组 | partial | **✓ enhanced** |
| 包安装日志 | ✗ | **✓** |
| Docker 构建日志 | partial | **✓ enhanced** |
| 逐命令节省量显示 | ✗ | **✓** |

---

## ⚙️ 工作原理

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

## 📈 追踪节省量

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
contextzip gain --graph      # 每日节省图表
contextzip gain --history    # 最近命令详情
```

---

## 🛡️ 安全保证

| 对象 | 规则 |
|:-----|:-----|
| 错误信息 | **始终**保留（首行 + 用户代码帧） |
| 文件:行号位置 | 构建错误中**绝不**移除 |
| 安全警告 | **始终**保留（CVE, GHSA, vulnerability） |
| Docker 失败上下文 | **始终**保留（失败步骤 + 前 2 步 + 退出码） |
| 退出码 | **始终**传播（CI/CD 安全） |

> [!IMPORTANT]
> ContextZip 只移除**已确认的噪音**。不确定时，原始输出原样通过。

---

## 🔧 CLI 参考

```bash
# 自动（通过 hook — 无需前缀）：
git status          # → contextzip git status
cargo test          # → contextzip cargo test
npm install         # → contextzip npm install

# 手动命令：
contextzip web https://docs.example.com    # 提取页面内容
contextzip err node server.js              # 错误聚焦输出

# 分析：
contextzip gain                  # 节省量仪表盘
contextzip gain --by-feature     # 按过滤器类型
contextzip gain --graph          # 每日图表
contextzip gain --history        # 最近命令

# 设置：
contextzip init --show           # 检查安装状态
contextzip update                # 自更新
contextzip uninstall             # 干净卸载
```

---

## 🤝 Contributing

欢迎贡献！ContextZip 是一个 Rust 项目。

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
  <sub>⚡ 节省 token，更快交付。</sub>
</p>

[![Star History Chart](https://api.star-history.com/svg?repos=jee599/contextzip&type=Date)](https://star-history.com/#jee599/contextzip&Date)
